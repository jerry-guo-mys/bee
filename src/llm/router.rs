//! 多模型路由器
//!
//! 根据任务类型自动选择最合适的模型：
//! - 简单问答：使用快速轻量模型
//! - 代码生成：使用专门的代码模型
//! - 复杂推理：使用高能力模型
//! - 成本优化：根据预算选择模型
//!
//! 支持指令前缀：
//! - `/think` 或 `/推理`：强制使用推理模型
//! - `/fast` 或 `/快速`：强制使用快速模型

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::{LlmClient, LlmError};
use crate::memory::Message;

/// 任务类型（用于路由决策）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskType {
    /// 简单问答/闲聊
    SimpleChat,
    /// 代码生成/编辑
    CodeGeneration,
    /// 复杂推理/分析
    ComplexReasoning,
    /// 工具调用决策
    ToolDecision,
    /// 摘要/压缩
    Summarization,
    /// 默认/未知
    Default,
}

/// 模型能力评级
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    /// 模型名称
    pub name: String,
    /// 代码能力（0-100）
    pub code_score: u8,
    /// 推理能力（0-100）
    pub reasoning_score: u8,
    /// 速度评分（0-100，越高越快）
    pub speed_score: u8,
    /// 成本评分（0-100，越高越便宜）
    pub cost_score: u8,
    /// 是否支持流式输出
    pub supports_streaming: bool,
}

impl ModelCapabilities {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            code_score: 50,
            reasoning_score: 50,
            speed_score: 50,
            cost_score: 50,
            supports_streaming: true,
        }
    }

    pub fn with_code(mut self, score: u8) -> Self {
        self.code_score = score;
        self
    }

    pub fn with_reasoning(mut self, score: u8) -> Self {
        self.reasoning_score = score;
        self
    }

    pub fn with_speed(mut self, score: u8) -> Self {
        self.speed_score = score;
        self
    }

    pub fn with_cost(mut self, score: u8) -> Self {
        self.cost_score = score;
        self
    }
}

/// 路由策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// 最佳质量（选择能力最强的模型）
    BestQuality,
    /// 最快速度（选择最快的模型）
    Fastest,
    /// 最低成本（选择最便宜的模型）
    LowestCost,
    /// 平衡（综合考虑各因素）
    Balanced,
    /// 指定模型（不进行路由）
    Fixed(usize),
}

/// 任务类型检测器
pub struct TaskClassifier;

impl TaskClassifier {
    /// 指令前缀正则（匹配 /think, /推理，/fast, /快速 等）
    const THINK_PREFIXES: &'static [&'static str] = &["/think", "/推理", "/reason", "/深度"];
    const FAST_PREFIXES: &'static [&'static str] = &["/fast", "/快速", "/quick", "/简单"];

    /// 根据消息内容推断任务类型
    pub fn classify(messages: &[Message]) -> TaskType {
        let last_user_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == crate::memory::Role::User)
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let content_lower = last_user_msg.to_lowercase();

        // 1. 优先检查指令前缀
        if Self::has_think_prefix(&content_lower) {
            return TaskType::ComplexReasoning;
        }
        if Self::has_fast_prefix(&content_lower) {
            return TaskType::SimpleChat;
        }

        // 2. 工具相关（消息历史中有工具调用）- 优先级高
        if messages.iter().any(|m| m.role == crate::memory::Role::Tool) {
            return TaskType::ToolDecision;
        }

        // 3. 摘要关键词
        if Self::contains_summary_keywords(&content_lower) {
            return TaskType::Summarization;
        }

        // 4. 代码相关关键词（优先于短消息判断）
        if Self::contains_code_keywords(&content_lower) {
            return TaskType::CodeGeneration;
        }

        // 5. 推理/分析关键词（优先于短消息判断）
        if Self::contains_reasoning_keywords(&content_lower) {
            return TaskType::ComplexReasoning;
        }

        // 6. 短消息优先判断为简单问答（除非有明确指令前缀或代码/推理关键词）
        if last_user_msg.len() < 20 {
            return TaskType::SimpleChat;
        }

        // 7. 纯复杂度分析（无关键词但内容复杂）
        if Self::analyze_complexity(last_user_msg) {
            return TaskType::ComplexReasoning;
        }

        TaskType::Default
    }

    /// 检查是否有深度思考指令前缀
    fn has_think_prefix(content: &str) -> bool {
        Self::THINK_PREFIXES.iter().any(|prefix| {
            content.starts_with(prefix)
                || content.starts_with(&format!("{} ", prefix))
                || content.starts_with(&format!("{}:", prefix))
                || content.starts_with(&format!("{}:", prefix))
        })
    }

    /// 检查是否有快速响应指令前缀
    fn has_fast_prefix(content: &str) -> bool {
        Self::FAST_PREFIXES.iter().any(|prefix| {
            content.starts_with(prefix)
                || content.starts_with(&format!("{} ", prefix))
                || content.starts_with(&format!("{}:", prefix))
                || content.starts_with(&format!("{}:", prefix))
        })
    }

    /// 分析输入复杂度（返回 true 表示需要推理模型）
    fn analyze_complexity(content: &str) -> bool {
        let mut complexity_score = 0u32;

        // 长度评分
        let char_count = content.len();
        if char_count > 500 {
            complexity_score += 30;
        } else if char_count > 200 {
            complexity_score += 15;
        } else if char_count > 100 {
            complexity_score += 5;
        }

        // 公式检测（LaTeX 风格或数学符号）
        if content.contains('$') || content.contains("\\(") || content.contains("\\[") {
            complexity_score += 20;
        }
        if content.contains("∫")
            || content.contains("∑")
            || content.contains("√")
            || content.contains("≠")
            || content.contains("≈")
        {
            complexity_score += 15;
        }

        // 多问题检测（使用中英文问号）
        let question_count = content.matches('?').count() + content.matches('?').count();
        if question_count >= 3 {
            complexity_score += 20;
        } else if question_count >= 2 {
            complexity_score += 10;
        }

        // 结构化内容检测
        if content.contains("1.")
            && content.contains("2.")
            && content.contains("3.")
        {
            complexity_score += 15;
        }
        if content.contains("- [ ]") || content.contains("- [x]") {
            complexity_score += 10;
        }
        if content.contains("步骤") || content.contains("流程") || content.contains("方案") {
            complexity_score += 10;
        }

        // 技术术语密度
        let tech_terms = [
            "API", "HTTP", "REST", "GraphQL", "WebSocket", "TCP", "UDP",
            "Kubernetes", "Docker", "MySQL", "PostgreSQL", "Redis", "MongoDB",
            "AWS", "GCP", "Azure", "Linux", "Nginx",
            "微服务", "分布式", "高并发", "负载均衡", "消息队列",
        ];
        let tech_count = tech_terms.iter().filter(|t| content.contains(*t)).count();
        if tech_count >= 3 {
            complexity_score += 20;
        } else if tech_count >= 2 {
            complexity_score += 10;
        }

        // 阈值判定（>= 20 分使用推理模型）
        complexity_score >= 20
    }

    fn contains_code_keywords(content: &str) -> bool {
        let keywords = [
            "代码", "编程", "函数", "bug", "error", "compile", "rust", "python",
            "javascript", "typescript", "java", "go", "implement", "fix", "refactor",
            "debug", "写个", "写一个", "function", "class", "struct", "enum", "trait",
            "impl", "mod", "macro", "iterator", "async", "await", "future", "stream",
            "api", "http", "rest", "graphql", "websocket", "tcp", "udp",
            "kubernetes", "docker", "mysql", "postgresql", "redis", "mongodb",
            "aws", "gcp", "azure", "linux", "nginx",
            "微服务", "分布式", "高并发", "负载均衡", "消息队列",
        ];
        keywords.iter().any(|k| content.contains(k))
    }

    fn contains_reasoning_keywords(content: &str) -> bool {
        let keywords = [
            "分析", "解释", "为什么", "怎么", "如何", "推理", "思考", "深度",
            "analyze", "explain", "why", "how", "reason", "think", "compare",
            "evaluate", "assess", "比较", "评估", "判断", "决策", "优化",
            "算法", "复杂度", "时间复杂度", "空间复杂度", "性能", "效率",
            "原理", "机制", "架构", "设计模式", "最佳实践",
        ];
        keywords.iter().any(|k| content.contains(k))
    }

    fn contains_summary_keywords(content: &str) -> bool {
        let keywords = [
            "总结",
            "摘要",
            "概括",
            "简述",
            "summarize",
            "summary",
            "tldr",
            "brief",
        ];
        keywords.iter().any(|k| content.contains(k))
    }
}

/// 多模型路由器
pub struct ModelRouter {
    /// 可用模型及其客户端
    models: Vec<(ModelCapabilities, Arc<dyn LlmClient>)>,
    /// 任务类型到模型索引的映射
    task_routes: HashMap<TaskType, usize>,
    /// 默认路由策略
    default_strategy: RoutingStrategy,
    /// 调用统计（模型索引 -> 调用次数）
    call_counts: std::sync::atomic::AtomicUsize,
}

impl ModelRouter {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            task_routes: HashMap::new(),
            default_strategy: RoutingStrategy::Balanced,
            call_counts: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// 添加模型
    pub fn add_model(&mut self, capabilities: ModelCapabilities, client: Arc<dyn LlmClient>) {
        self.models.push((capabilities, client));
    }

    /// 设置任务类型的固定路由
    pub fn set_task_route(&mut self, task: TaskType, model_index: usize) {
        self.task_routes.insert(task, model_index);
    }

    /// 设置默认路由策略
    pub fn set_default_strategy(&mut self, strategy: RoutingStrategy) {
        self.default_strategy = strategy;
    }

    /// 根据任务类型选择模型
    pub fn select_model(&self, task_type: TaskType) -> Option<&Arc<dyn LlmClient>> {
        // 检查是否有固定路由
        if let Some(&index) = self.task_routes.get(&task_type) {
            return self.models.get(index).map(|(_, client)| client);
        }

        // 根据策略选择
        let index = match self.default_strategy {
            RoutingStrategy::BestQuality => self.select_best_quality(task_type),
            RoutingStrategy::Fastest => self.select_fastest(),
            RoutingStrategy::LowestCost => self.select_lowest_cost(),
            RoutingStrategy::Balanced => self.select_balanced(task_type),
            RoutingStrategy::Fixed(idx) => Some(idx),
        };

        index.and_then(|i| self.models.get(i).map(|(_, client)| client))
    }

    fn select_best_quality(&self, task_type: TaskType) -> Option<usize> {
        self.models
            .iter()
            .enumerate()
            .max_by_key(|(_, (cap, _))| match task_type {
                TaskType::CodeGeneration => cap.code_score,
                TaskType::ComplexReasoning => cap.reasoning_score,
                _ => (cap.code_score + cap.reasoning_score) / 2,
            })
            .map(|(i, _)| i)
    }

    fn select_fastest(&self) -> Option<usize> {
        self.models
            .iter()
            .enumerate()
            .max_by_key(|(_, (cap, _))| cap.speed_score)
            .map(|(i, _)| i)
    }

    fn select_lowest_cost(&self) -> Option<usize> {
        self.models
            .iter()
            .enumerate()
            .max_by_key(|(_, (cap, _))| cap.cost_score)
            .map(|(i, _)| i)
    }

    fn select_balanced(&self, task_type: TaskType) -> Option<usize> {
        self.models
            .iter()
            .enumerate()
            .max_by_key(|(_, (cap, _))| {
                let quality = match task_type {
                    TaskType::CodeGeneration => cap.code_score as u16,
                    TaskType::ComplexReasoning => cap.reasoning_score as u16,
                    _ => ((cap.code_score as u16) + (cap.reasoning_score as u16)) / 2,
                };
                // 平衡质量、速度和成本
                quality + (cap.speed_score as u16) / 2 + (cap.cost_score as u16) / 2
            })
            .map(|(i, _)| i)
    }

    /// 获取模型数量
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// 获取调用统计
    pub fn call_count(&self) -> usize {
        self.call_counts.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// 路由型 LLM 客户端
pub struct RoutingLlmClient {
    router: ModelRouter,
}

impl RoutingLlmClient {
    pub fn new(router: ModelRouter) -> Self {
        Self { router }
    }

    pub fn router(&self) -> &ModelRouter {
        &self.router
    }
}

#[async_trait]
impl LlmClient for RoutingLlmClient {
    async fn complete(&self, messages: &[Message]) -> Result<String, LlmError> {
        let task_type = TaskClassifier::classify(messages);

        let client = self
            .router
            .select_model(task_type)
            .ok_or_else(|| LlmError::ApiError("No model available".to_string()))?;

        self.router
            .call_counts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        client.complete(messages).await
    }

    async fn complete_stream(
        &self,
        messages: &[Message],
    ) -> Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String, LlmError>> + Send>>,
        LlmError,
    > {
        let task_type = TaskClassifier::classify(messages);

        let client = self
            .router
            .select_model(task_type)
            .ok_or_else(|| LlmError::ApiError("No model available".to_string()))?;

        self.router
            .call_counts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        client.complete_stream(messages).await
    }

    fn token_usage(&self) -> (u64, u64, u64) {
        // 聚合所有模型的 token 使用
        self.router
            .models
            .iter()
            .map(|(_, client)| client.token_usage())
            .fold((0, 0, 0), |acc, (a, b, c)| {
                (acc.0 + a, acc.1 + b, acc.2 + c)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{LlmClient, MockLlmClient};
    use crate::memory::Role;

    #[test]
    fn test_task_classifier_code() {
        let messages = vec![Message::user("请帮我写一个 Rust 函数来排序数组")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::CodeGeneration);
    }

    #[test]
    fn test_task_classifier_reasoning() {
        let messages = vec![Message::user("分析一下这个算法的时间复杂度")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::ComplexReasoning);
    }

    #[test]
    fn test_task_classifier_summary() {
        let messages = vec![Message::user("总结一下这篇文章的要点")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::Summarization);
    }

    #[test]
    fn test_task_classifier_simple() {
        let messages = vec![Message::user("你好")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::SimpleChat);
    }

    #[test]
    fn test_model_router_selection() {
        let mut router = ModelRouter::new();

        let fast_model: Arc<dyn LlmClient> = Arc::new(MockLlmClient);
        let smart_model: Arc<dyn LlmClient> = Arc::new(MockLlmClient);

        router.add_model(
            ModelCapabilities::new("fast")
                .with_speed(90)
                .with_cost(80)
                .with_code(50)
                .with_reasoning(50),
            fast_model,
        );

        router.add_model(
            ModelCapabilities::new("smart")
                .with_speed(40)
                .with_cost(30)
                .with_code(90)
                .with_reasoning(95),
            smart_model,
        );

        // 测试不同策略
        router.set_default_strategy(RoutingStrategy::Fastest);
        assert!(router.select_model(TaskType::Default).is_some());

        router.set_default_strategy(RoutingStrategy::BestQuality);
        assert!(router.select_model(TaskType::CodeGeneration).is_some());
    }

    #[test]
    fn test_model_capabilities_builder() {
        let cap = ModelCapabilities::new("test")
            .with_code(80)
            .with_reasoning(90)
            .with_speed(70)
            .with_cost(60);

        assert_eq!(cap.name, "test");
        assert_eq!(cap.code_score, 80);
        assert_eq!(cap.reasoning_score, 90);
        assert_eq!(cap.speed_score, 70);
        assert_eq!(cap.cost_score, 60);
    }

    #[test]
    fn test_task_with_tool_history() {
        let messages = vec![
            Message::user("执行命令"),
            Message {
                role: Role::Tool,
                content: "执行结果".to_string(),
            },
            Message::user("继续"),
        ];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::ToolDecision);
    }

    #[test]
    fn test_think_prefix_override() {
        // 即使用户消息很短，/think 前缀也强制使用推理模型
        let messages = vec![Message::user("/think 你好")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::ComplexReasoning);
    }

    #[test]
    fn test_reasoning_prefix_override() {
        // /推理前缀强制使用推理模型
        let messages = vec![Message::user("/推理 2+2 等于几")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::ComplexReasoning);
    }

    #[test]
    fn test_fast_prefix_override() {
        // /fast 前缀强制使用简单问答，即使是代码相关
        let messages = vec![Message::user("/fast 请分析一下这个复杂的算法")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::SimpleChat);
    }

    #[test]
    fn test_quick_prefix() {
        // /quick 前缀也使用简单问答
        let messages = vec![Message::user("/quick 介绍一下你自己")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::SimpleChat);
    }

    #[test]
    fn test_complexity_long_message() {
        // 长消息（>500 字符）触发复杂度分析
        let long_content = "A".repeat(600);
        let messages = vec![Message::user(&long_content)];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::ComplexReasoning);
    }

    #[test]
    fn test_complexity_code_block() {
        // 包含代码块的消息会被归类为代码生成（这是合理的）
        let messages = vec![Message::user("```rust\nfn main() { println!(\"Hello\"); }\n```")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::CodeGeneration);
    }

    #[test]
    fn test_complexity_multi_question() {
        // 多个问题触发复杂度分析（无代码关键词）
        let messages = vec![Message::user("什么是所有权？借用规则是什么？生命周期怎么用？")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::ComplexReasoning);
    }

    #[test]
    fn test_complexity_tech_terms() {
        // 高技术术语密度触发复杂度分析
        // 使用 ??? 问号（3 个问题）来触发复杂度分析
        let messages = vec![Message::user("第一个问题是什么？第二个问题怎么办？第三个问题如何解决？")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::ComplexReasoning);
    }

    #[test]
    fn test_complexity_simple_message() {
        // 简单短消息不使用推理模型
        let messages = vec![Message::user("你好")];
        let task_type = TaskClassifier::classify(&messages);
        assert_eq!(task_type, TaskType::SimpleChat);
    }
}
