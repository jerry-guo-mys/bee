//! Critic：结果反思与校验

use std::collections::HashSet;
use std::sync::Arc;

use crate::config::CriticSection;
use crate::llm::LlmClient;
use crate::memory::Message;

/// Critic 评估结果
#[derive(Debug, Clone)]
pub enum CriticResult {
    /// 通过
    Approved { score: f32 },
    /// 需要修正
    Review(CriticReview),
    /// 跳过评估
    Skipped,
}

/// Critic 审查详情
#[derive(Debug, Clone)]
pub struct CriticReview {
    pub score: f32,
    pub reason: String,
    pub retry_recommended: bool,
    pub blocking_risk: bool,
}

/// Critic：持有 LLM 与 prompt 模板
pub struct Critic {
    llm: Arc<dyn LlmClient>,
    prompt_template: String,
    evaluate_all_tools: bool,
    evaluate_tools: HashSet<String>,
    score_threshold: f32,
    max_self_corrections: usize,
}

impl Critic {
    /// 从配置创建 Critic
    pub fn from_config(llm: Arc<dyn LlmClient>, config: &CriticSection) -> Self {
        Self {
            llm,
            prompt_template: config.prompt_template.clone(),
            evaluate_all_tools: config.evaluate_all_tools,
            evaluate_tools: config.evaluate_tools.iter().cloned().collect(),
            score_threshold: config.score_threshold,
            max_self_corrections: config.max_self_corrections,
        }
    }

    /// 直接创建 Critic
    pub fn new(llm: Arc<dyn LlmClient>, prompt_template: impl Into<String>) -> Self {
        Self {
            llm,
            prompt_template: prompt_template.into(),
            evaluate_all_tools: true,
            evaluate_tools: HashSet::new(),
            score_threshold: 0.45,
            max_self_corrections: 2,
        }
    }

    /// 获取最大自修正次数
    pub fn max_self_corrections(&self) -> usize {
        self.max_self_corrections
    }

    /// 评估工具执行结果（桩实现，待完善）
    pub async fn evaluate(
        &self,
        goal: &str,
        tool: &str,
        observation: &str,
    ) -> Result<CriticResult, String> {
        // 检查是否应该评估此工具
        if !self.should_evaluate(tool) {
            return Ok(CriticResult::Skipped);
        }

        // 构建评估 prompt
        let prompt = self
            .prompt_template
            .replace("{goal}", goal)
            .replace("{tool}", tool)
            .replace("{observation}", observation);

        let messages = vec![Message::user(prompt)];
        let response = self.llm.complete(&messages).await.map_err(|e| e.to_string())?;
        let trimmed = response.trim();

        // 解析响应
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("OK") {
            return Ok(CriticResult::Approved { score: 1.0 });
        }

        // 尝试解析 JSON
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let score = value.get("score").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            let reason = value
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let retry_recommended = value
                .get("retry_recommended")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let blocking_risk = value
                .get("blocking_risk")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if score >= self.score_threshold && !blocking_risk {
                return Ok(CriticResult::Approved { score });
            }

            return Ok(CriticResult::Review(CriticReview {
                score,
                reason,
                retry_recommended,
                blocking_risk,
            }));
        }

        Ok(CriticResult::Review(CriticReview {
            score: 0.2,
            reason: trimmed.to_string(),
            retry_recommended: true,
            blocking_risk: false,
        }))
    }

    fn should_evaluate(&self, tool: &str) -> bool {
        if self.evaluate_all_tools {
            return true;
        }
        if self.evaluate_tools.is_empty() {
            return true;
        }
        self.evaluate_tools.contains(tool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmClient;

    #[test]
    fn test_critic_new() {
        let critic = Critic::new(Arc::new(MockLlmClient::default()), "test prompt");
        assert!(critic.evaluate_all_tools);
        assert_eq!(critic.max_self_corrections(), 2);
    }
}
