//! Critic：结果反思与校验（解决问题 4.3：配置化与模型分离）
//!
//! 在将工具 Observation 喂回 Planner 前，可选一次轻量 LLM 调用判断「是否符合预期」，
//! 若不符合则返回 Correction 作为下一轮上下文，减少重复犯错。
//!
//! 通过配置可以：
//! - 启用/禁用 Critic
//! - 使用与 Planner 不同的模型（避免自我认同）
//! - 仅评估特定工具（减少 token 开销）

use std::collections::HashSet;
use std::sync::Arc;

use serde_json::Value;

use crate::config::CriticSection;
use crate::llm::LlmClient;
use crate::memory::Message;
use crate::tools::{ToolCriticMode, ToolMetadata, ToolOutputShape, ToolRisk};

/// Critic 评估结果：通过或需修正
#[derive(Debug, Clone)]
pub enum CriticResult {
    /// 通过
    Approved { score: f32 },
    /// 需要修正
    Review(CriticReview),
    /// 跳过评估（该工具不在评估列表中）
    Skipped,
}

#[derive(Debug, Clone)]
pub struct CriticReview {
    pub score: f32,
    pub reason: String,
    pub retry_recommended: bool,
    pub blocking_risk: bool,
}

/// Critic：持有 LLM 与 prompt 模板，evaluate(goal, tool, observation) 返回 Approved / Correction / Skipped
pub struct Critic {
    llm: Arc<dyn LlmClient>,
    prompt_template: String,
    /// 是否评估所有工具
    evaluate_all_tools: bool,
    /// 仅评估的工具集合（evaluate_all_tools=false 时生效）
    evaluate_tools: HashSet<String>,
    score_threshold: f32,
    max_self_corrections: usize,
}

impl Critic {
    /// 从配置创建 Critic（需要外部传入 LLM 实例）
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

    /// 直接创建 Critic（向后兼容）
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

    /// 设置仅评估特定工具
    pub fn with_evaluate_tools(mut self, tools: Vec<String>) -> Self {
        self.evaluate_all_tools = false;
        self.evaluate_tools = tools.into_iter().collect();
        self
    }

    /// 设置评估所有工具
    pub fn with_evaluate_all(mut self) -> Self {
        self.evaluate_all_tools = true;
        self
    }

    pub fn score_threshold(&self) -> f32 {
        self.score_threshold
    }

    pub fn max_self_corrections(&self) -> usize {
        self.max_self_corrections
    }

    /// 检查是否应该评估此工具
    fn should_evaluate(&self, tool: &str) -> bool {
        if self.evaluate_all_tools {
            return true;
        }
        if self.evaluate_tools.is_empty() {
            return true;
        }
        self.evaluate_tools.contains(tool)
    }

    fn observation_is_sufficient_structured(observation: &str) -> bool {
        serde_json::from_str::<Value>(observation)
            .ok()
            .and_then(|value| value.get("sufficient_to_answer").and_then(|v| v.as_bool()))
            .unwrap_or(false)
    }

    pub fn should_evaluate_with_metadata(
        &self,
        tool: &str,
        metadata: Option<&ToolMetadata>,
        observation: &str,
    ) -> bool {
        if !self.should_evaluate(tool) {
            return false;
        }

        let Some(metadata) = metadata else {
            return true;
        };

        match metadata.critic_mode {
            ToolCriticMode::Skip => return false,
            ToolCriticMode::Always => return true,
            ToolCriticMode::Conservative => {
                if Self::observation_is_sufficient_structured(observation) {
                    return false;
                }
                if metadata.risk == ToolRisk::Low
                    && metadata.output_shape == ToolOutputShape::StructuredJson
                {
                    return false;
                }
            }
            ToolCriticMode::Normal => {
                if metadata.risk == ToolRisk::Low
                    && metadata.output_shape == ToolOutputShape::StructuredJson
                    && Self::observation_is_sufficient_structured(observation)
                {
                    return false;
                }
            }
        }

        true
    }

    pub async fn evaluate(
        &self,
        goal: &str,
        tool: &str,
        observation: &str,
    ) -> Result<CriticResult, String> {
        if !self.should_evaluate(tool) {
            return Ok(CriticResult::Skipped);
        }

        let prompt = self
            .prompt_template
            .replace("{goal}", goal)
            .replace("{tool}", tool)
            .replace("{observation}", observation);

        let messages = vec![Message::user(prompt)];
        let response = self
            .llm
            .complete(&messages)
            .await
            .map_err(|e| e.to_string())?;
        let trimmed = response.trim();

        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("OK") {
            return Ok(CriticResult::Approved { score: 1.0 });
        }

        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmClient;
    use crate::tools::{ToolFreshness, ToolIntent, ToolScope};
    use async_trait::async_trait;

    struct StaticLlm(&'static str);

    #[async_trait]
    impl LlmClient for StaticLlm {
        async fn complete(&self, _messages: &[Message]) -> Result<String, crate::llm::LlmError> {
            Ok(self.0.to_string())
        }

        async fn complete_stream(
            &self,
            _messages: &[Message],
        ) -> Result<
            std::pin::Pin<
                Box<dyn futures_util::Stream<Item = Result<String, crate::llm::LlmError>> + Send>,
            >,
            crate::llm::LlmError,
        > {
            Ok(Box::pin(futures_util::stream::iter(vec![Ok(self
                .0
                .to_string())])))
        }
    }

    #[test]
    fn test_should_evaluate_all() {
        let critic = Critic::new(Arc::new(MockLlmClient), "test");
        assert!(critic.should_evaluate("any_tool"));
    }

    #[test]
    fn test_should_evaluate_specific() {
        let critic = Critic::new(Arc::new(MockLlmClient), "test")
            .with_evaluate_tools(vec!["shell".to_string(), "code_edit".to_string()]);
        assert!(critic.should_evaluate("shell"));
        assert!(critic.should_evaluate("code_edit"));
        assert!(!critic.should_evaluate("cat"));
    }

    #[test]
    fn test_should_skip_low_risk_structured_tool_when_sufficient() {
        let critic = Critic::new(Arc::new(MockLlmClient), "test");
        let metadata = ToolMetadata::new(ToolScope::RemoteWeb, vec![ToolIntent::FetchWebPage])
            .with_risk(ToolRisk::Low)
            .with_output_shape(ToolOutputShape::StructuredJson)
            .with_freshness(ToolFreshness::Live)
            .with_critic_mode(ToolCriticMode::Conservative);

        assert!(!critic.should_evaluate_with_metadata(
            "weather",
            Some(&metadata),
            r#"{"tool":"weather","summary":"ok","sufficient_to_answer":true,"data":{}}"#,
        ));
    }

    #[test]
    fn test_should_keep_evaluating_high_risk_tool() {
        let critic = Critic::new(Arc::new(MockLlmClient), "test");
        let metadata = ToolMetadata::new(
            ToolScope::System,
            vec![ToolIntent::RunCommand, ToolIntent::ExecuteSideEffect],
        )
        .with_risk(ToolRisk::High)
        .with_critic_mode(ToolCriticMode::Always);

        assert!(critic.should_evaluate_with_metadata("shell", Some(&metadata), "command output",));
    }

    #[test]
    fn test_critic_parses_json_review() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let critic = Critic::new(
                Arc::new(StaticLlm(
                    r#"{"score":0.2,"reason":"stale result","retry_recommended":true,"blocking_risk":false}"#,
                )),
                "test",
            );
            let result = critic.evaluate("goal", "search", "obs").await.unwrap();
            match result {
                CriticResult::Review(review) => {
                    assert_eq!(review.reason, "stale result");
                    assert!(review.retry_recommended);
                }
                _ => panic!("expected review"),
            }
        });
    }
}
