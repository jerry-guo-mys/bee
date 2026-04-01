//! 全链路追踪数据模型
//!
//! 提供请求级和 Span 级的追踪数据结构，用于记录和分析每次用户请求
//! 从进入到退出的完整执行过程。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 请求 ID 类型别名
pub type RequestId = String;

/// Span ID 类型别名
pub type SpanId = String;

/// 追踪状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    /// 请求正在进行中
    Running,
    /// 请求成功完成
    Success,
    /// 请求失败
    Failure,
    /// 请求被取消
    Cancelled,
}

/// 追踪元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraceMetadata {
    /// 用户 ID（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// 会话 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// 请求来源（TUI/Web/WhatsApp/Lark/Gateway）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// 请求类型（chat/command/tool_call/evolution 等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_type: Option<String>,
    /// 自定义标签
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
}

impl TraceMetadata {
    /// 创建新的元数据
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置用户 ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// 设置会话 ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置请求来源
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// 设置请求类型
    pub fn with_request_type(mut self, request_type: impl Into<String>) -> Self {
        self.request_type = Some(request_type.into());
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// 操作类型枚举 - 标识 ReAct 循环中的各种操作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    /// Orchestrator 协调器操作
    Orchestrator,
    /// Planner 规划器操作
    Planner,
    /// Critic 评估器操作
    Critic,
    /// LLM 调用
    LlmCall,
    /// 工具执行
    ToolExecution,
    /// 内存操作（读取/写入）
    Memory,
    /// RAG 检索
    RagRetrieval,
    /// 响应流式输出
    ResponseStream,
    /// 技能选择
    SkillSelection,
    /// 自我进化分析
    EvolutionAnalysis,
    /// 自定义操作
    Custom(String),
}

impl OperationKind {
    /// 获取操作类型的字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            OperationKind::Orchestrator => "orchestrator",
            OperationKind::Planner => "planner",
            OperationKind::Critic => "critic",
            OperationKind::LlmCall => "llm_call",
            OperationKind::ToolExecution => "tool_execution",
            OperationKind::Memory => "memory",
            OperationKind::RagRetrieval => "rag_retrieval",
            OperationKind::ResponseStream => "response_stream",
            OperationKind::SkillSelection => "skill_selection",
            OperationKind::EvolutionAnalysis => "evolution_analysis",
            OperationKind::Custom(s) => s.as_str(),
        }
    }
}

/// Span 状态枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    /// Span 正在进行中
    Running,
    /// Span 成功完成
    Success,
    /// Span 失败
    Failure,
}

/// 属性值类型 - 支持多种值类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    /// 字符串值
    String(String),
    /// 整数值
    Integer(i64),
    /// 浮点数值
    Float(f64),
    /// 布尔值
    Bool(bool),
    /// 字符串数组
    Array(Vec<String>),
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        AttributeValue::String(s.to_string())
    }
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        AttributeValue::String(s)
    }
}

impl From<i64> for AttributeValue {
    fn from(n: i64) -> Self {
        AttributeValue::Integer(n)
    }
}

impl From<f64> for AttributeValue {
    fn from(n: f64) -> Self {
        AttributeValue::Float(n)
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> Self {
        AttributeValue::Bool(b)
    }
}

/// Span 级追踪 - 记录单个操作的详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanTrace {
    /// Span 唯一标识
    pub span_id: SpanId,
    /// 所属请求 ID
    pub request_id: RequestId,
    /// 父 Span ID（如果有，用于构建调用链树）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<SpanId>,
    /// 操作类型
    pub operation_kind: OperationKind,
    /// 操作名称/描述
    pub operation_name: String,
    /// Span 开始时间戳（Unix 毫秒）
    pub start_timestamp_ms: u64,
    /// 持续时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Span 状态
    pub status: SpanStatus,
    /// 属性集合
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, AttributeValue>,
    /// 错误信息（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// ReAct 步骤编号（如果在 ReAct 循环中）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub react_step: Option<u32>,
}

impl SpanTrace {
    /// 创建新的 SpanTrace
    pub fn new(
        request_id: RequestId,
        operation_kind: OperationKind,
        operation_name: impl Into<String>,
    ) -> Self {
        Self {
            span_id: Uuid::new_v4().to_string(),
            request_id,
            parent_span_id: None,
            operation_kind,
            operation_name: operation_name.into(),
            start_timestamp_ms: current_timestamp_ms(),
            duration_ms: None,
            status: SpanStatus::Running,
            attributes: HashMap::new(),
            error_message: None,
            react_step: None,
        }
    }

    /// 设置父 Span ID
    pub fn with_parent(mut self, parent_span_id: impl Into<SpanId>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }

    /// 设置 ReAct 步骤编号
    pub fn with_react_step(mut self, step: u32) -> Self {
        self.react_step = Some(step);
        self
    }

    /// 添加属性
    pub fn with_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<AttributeValue>,
    ) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// 标记为成功
    pub fn mark_success(&mut self, duration: Duration) {
        self.status = SpanStatus::Success;
        self.duration_ms = Some(duration.as_millis() as u64);
    }

    /// 标记为失败
    pub fn mark_failure(&mut self, duration: Duration, error_message: impl Into<String>) {
        self.status = SpanStatus::Failure;
        self.duration_ms = Some(duration.as_millis() as u64);
        self.error_message = Some(error_message.into());
    }

    /// 完成 Span 并返回实例
    pub fn complete(mut self, success: bool, duration: Duration) -> Self {
        if success {
            self.mark_success(duration);
        }
        self
    }

    /// 完成 Span 并返回实例（带错误信息）
    pub fn complete_with_error(mut self, duration: Duration, error: impl Into<String>) -> Self {
        self.mark_failure(duration, error);
        self
    }
}

/// 请求级追踪 - 记录整个请求的生命周期
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTrace {
    /// 请求唯一标识
    pub request_id: RequestId,
    /// 会话 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// 请求开始时间戳（Unix 毫秒）
    pub start_timestamp_ms: u64,
    /// 请求结束时间戳（Unix 毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_timestamp_ms: Option<u64>,
    /// 总持续时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// 请求状态
    pub status: TraceStatus,
    /// 请求输入内容摘要
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_summary: Option<String>,
    /// 请求输出内容摘要
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_summary: Option<String>,
    /// 追踪元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TraceMetadata>,
    /// 包含的 Spans
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spans: Vec<SpanTrace>,
    /// 错误信息（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// ReAct 循环步数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub react_steps_total: Option<u32>,
    /// LLM 调用次数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_calls_count: Option<u32>,
    /// 工具执行次数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_executions_count: Option<u32>,
    /// 总 token 消耗（prompt + completion）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

impl RequestTrace {
    /// 创建新的 RequestTrace
    pub fn new(request_id: RequestId) -> Self {
        Self {
            request_id,
            session_id: None,
            start_timestamp_ms: current_timestamp_ms(),
            end_timestamp_ms: None,
            duration_ms: None,
            status: TraceStatus::Running,
            input_summary: None,
            output_summary: None,
            metadata: None,
            spans: Vec::new(),
            error_message: None,
            react_steps_total: None,
            llm_calls_count: None,
            tool_executions_count: None,
            total_tokens: None,
        }
    }

    /// 创建新的 RequestTrace（带会话 ID）
    pub fn with_session(request_id: RequestId, session_id: impl Into<String>) -> Self {
        let mut trace = Self::new(request_id);
        trace.session_id = Some(session_id.into());
        trace
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: TraceMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// 设置输入摘要
    pub fn with_input_summary(mut self, summary: impl Into<String>) -> Self {
        self.input_summary = Some(summary.into());
        self
    }

    /// 设置输出摘要
    pub fn with_output_summary(mut self, summary: impl Into<String>) -> Self {
        self.output_summary = Some(summary.into());
        self
    }

    /// 添加 Span
    pub fn add_span(&mut self, span: SpanTrace) {
        self.spans.push(span);
    }

    /// 记录 LLM 调用
    pub fn record_llm_call(&mut self) {
        self.llm_calls_count = Some(self.llm_calls_count.unwrap_or(0) + 1);
    }

    /// 记录工具执行
    pub fn record_tool_execution(&mut self) {
        self.tool_executions_count = Some(self.tool_executions_count.unwrap_or(0) + 1);
    }

    /// 记录 token 消耗
    pub fn record_tokens(&mut self, tokens: u64) {
        self.total_tokens = Some(self.total_tokens.unwrap_or(0) + tokens);
    }

    /// 标记请求成功完成
    pub fn mark_success(&mut self, duration: Duration) {
        self.status = TraceStatus::Success;
        self.end_timestamp_ms = Some(current_timestamp_ms());
        self.duration_ms = Some(duration.as_millis() as u64);
    }

    /// 标记请求失败
    pub fn mark_failure(&mut self, duration: Duration, error_message: impl Into<String>) {
        self.status = TraceStatus::Failure;
        self.end_timestamp_ms = Some(current_timestamp_ms());
        self.duration_ms = Some(duration.as_millis() as u64);
        self.error_message = Some(error_message.into());
    }

    /// 标记请求被取消
    pub fn mark_cancelled(&mut self, duration: Duration) {
        self.status = TraceStatus::Cancelled;
        self.end_timestamp_ms = Some(current_timestamp_ms());
        self.duration_ms = Some(duration.as_millis() as u64);
    }

    /// 完成请求并返回实例
    pub fn complete(mut self, success: bool, duration: Duration) -> Self {
        if success {
            self.mark_success(duration);
        } else {
            self.mark_failure(duration, "Request failed");
        }
        self
    }

    /// 获取 Span 数量
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// 获取特定操作类型的 Spans
    pub fn spans_by_kind(&self, kind: &OperationKind) -> Vec<&SpanTrace> {
        self.spans
            .iter()
            .filter(|span| span.operation_kind == *kind)
            .collect()
    }
}

/// 获取当前时间戳（Unix 毫秒）
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Span 构建器 - 用于链式构建 SpanTrace
#[derive(Debug)]
pub struct SpanBuilder {
    request_id: RequestId,
    operation_kind: OperationKind,
    operation_name: String,
    parent_span_id: Option<SpanId>,
    react_step: Option<u32>,
    attributes: HashMap<String, AttributeValue>,
    start_time: Option<Instant>,
}

impl SpanBuilder {
    /// 创建新的 Span 构建器
    pub fn new(
        request_id: RequestId,
        operation_kind: OperationKind,
        operation_name: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            operation_kind,
            operation_name: operation_name.into(),
            parent_span_id: None,
            react_step: None,
            attributes: HashMap::new(),
            start_time: None,
        }
    }

    /// 设置父 Span
    pub fn parent(mut self, parent_span_id: impl Into<SpanId>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }

    /// 设置 ReAct 步骤
    pub fn react_step(mut self, step: u32) -> Self {
        self.react_step = Some(step);
        self
    }

    /// 添加属性
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<AttributeValue>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// 设置开始时间
    pub fn start_time(mut self, start: Instant) -> Self {
        self.start_time = Some(start);
        self
    }

    /// 构建 SpanTrace
    pub fn build(self) -> SpanTrace {
        let mut span = SpanTrace::new(self.request_id, self.operation_kind, self.operation_name);
        span.parent_span_id = self.parent_span_id;
        span.react_step = self.react_step;
        span.attributes = self.attributes;
        if let Some(start) = self.start_time {
            let elapsed = Instant::now().duration_since(start);
            span.start_timestamp_ms = current_timestamp_ms() - elapsed.as_millis() as u64;
        }
        span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_trace_creation() {
        let trace = RequestTrace::new("req-123".to_string());

        assert_eq!(trace.request_id, "req-123");
        assert_eq!(trace.status, TraceStatus::Running);
        assert!(trace.spans.is_empty());
        assert!(trace.duration_ms.is_none());
    }

    #[test]
    fn test_request_trace_with_metadata() {
        let metadata = TraceMetadata::new()
            .with_user_id("user-456")
            .with_session_id("session-789")
            .with_source("TUI")
            .with_request_type("chat")
            .with_tag("feature", "test");

        let trace = RequestTrace::new("req-123".to_string()).with_metadata(metadata.clone());

        assert_eq!(
            trace.metadata.as_ref().unwrap().user_id,
            Some("user-456".to_string())
        );
        assert_eq!(
            trace.metadata.as_ref().unwrap().session_id,
            Some("session-789".to_string())
        );
        assert_eq!(
            trace.metadata.as_ref().unwrap().source,
            Some("TUI".to_string())
        );
        assert_eq!(
            trace.metadata.as_ref().unwrap().request_type,
            Some("chat".to_string())
        );
        assert_eq!(
            trace.metadata.as_ref().unwrap().tags.get("feature"),
            Some(&"test".to_string())
        );
    }

    #[test]
    fn test_span_trace_lifecycle() {
        let request_id = "req-123".to_string();
        let mut span = SpanTrace::new(request_id.clone(), OperationKind::Planner, "Generate plan")
            .with_attribute("model", "deepseek-chat")
            .with_attribute("step", 1i64);

        assert_eq!(span.request_id, request_id);
        assert_eq!(span.operation_kind, OperationKind::Planner);
        assert_eq!(span.status, SpanStatus::Running);
        assert!(span.duration_ms.is_none());

        // 模拟完成
        let duration = Duration::from_millis(150);
        span.mark_success(duration);

        assert_eq!(span.status, SpanStatus::Success);
        assert!(span.duration_ms.is_some());
        assert!(span.error_message.is_none());
    }

    #[test]
    fn test_span_trace_failure() {
        let span = SpanTrace::new(
            "req-123".to_string(),
            OperationKind::ToolExecution,
            "shell_command",
        )
        .with_attribute("command", "invalid_command")
        .complete_with_error(Duration::from_millis(50), "Command not found");

        assert_eq!(span.status, SpanStatus::Failure);
        assert_eq!(span.duration_ms, Some(50));
        assert_eq!(span.error_message, Some("Command not found".to_string()));
    }

    #[test]
    fn test_request_trace_complete_with_spans() {
        let mut trace = RequestTrace::with_session("req-123".to_string(), "session-456")
            .with_input_summary("Test input")
            .with_output_summary("Test output");

        // 添加多个 spans
        trace.add_span(SpanTrace::new(
            "req-123".to_string(),
            OperationKind::Planner,
            "Planning",
        ));
        trace.add_span(SpanTrace::new(
            "req-123".to_string(),
            OperationKind::LlmCall,
            "LLM Call 1",
        ));
        trace.add_span(SpanTrace::new(
            "req-123".to_string(),
            OperationKind::ToolExecution,
            "Tool execution",
        ));

        // 记录一些统计
        trace.record_llm_call();
        trace.record_llm_call();
        trace.record_tool_execution();
        trace.record_tokens(1500);

        // 完成请求
        let duration = Duration::from_millis(500);
        trace.mark_success(duration);

        assert_eq!(trace.status, TraceStatus::Success);
        assert_eq!(trace.span_count(), 3);
        assert_eq!(trace.llm_calls_count, Some(2));
        assert_eq!(trace.tool_executions_count, Some(1));
        assert_eq!(trace.total_tokens, Some(1500));
        assert!(trace.duration_ms.is_some());

        // 验证 spans_by_kind
        let llm_spans = trace.spans_by_kind(&OperationKind::LlmCall);
        assert_eq!(llm_spans.len(), 1);
    }
}
