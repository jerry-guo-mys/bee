//! Planner：意图规划与 Tool Call 解析

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::core::AgentError;
use crate::llm::LlmClient;
use crate::memory::Message;

/// LLM 返回的 Tool Call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Planner 输出
#[derive(Debug, Clone)]
pub enum PlannerOutput {
    /// 直接回复用户
    Response(String),
    /// 需要执行工具
    ToolCall(ToolCall),
}

/// Planner：持有 LLM 与 system prompt
pub struct Planner {
    llm: Arc<dyn LlmClient>,
    system_prompt: String,
}

impl Planner {
    pub fn new(llm: Arc<dyn LlmClient>, system_prompt: impl Into<String>) -> Self {
        Self {
            llm,
            system_prompt: system_prompt.into(),
        }
    }

    /// 获取基础 system prompt
    pub fn base_system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// 获取 token 使用统计
    pub fn token_usage(&self) -> (u64, u64, u64) {
        self.llm.token_usage()
    }

    /// 执行规划
    pub async fn plan(&self, messages: &[Message]) -> Result<String, AgentError> {
        self.plan_with_system(messages, &self.system_prompt).await
    }

    /// 使用动态 system prompt 执行规划
    pub async fn plan_with_system(
        &self,
        messages: &[Message],
        system: &str,
    ) -> Result<String, AgentError> {
        let mut full_messages = vec![Message::system(system.to_string())];
        full_messages.extend(messages.to_vec());
        self.llm
            .complete(&full_messages)
            .await
            .map_err(AgentError::LlmError)
    }

    /// 将对话历史压缩为摘要
    pub async fn summarize(&self, messages: &[Message]) -> Result<String, AgentError> {
        if messages.is_empty() {
            return Ok(String::new());
        }
        let system =
            "You are a summarizer. Summarize the following conversation in one short paragraph.";
        let mut full = vec![Message::system(system.to_string())];
        full.extend(messages.to_vec());
        self.llm.complete(&full).await.map_err(AgentError::LlmError)
    }
}

/// 解析 LLM 输出
pub fn parse_llm_output(output: &str) -> Result<PlannerOutput, AgentError> {
    let trimmed = output.trim();

    // 尝试提取 JSON
    if let Some(json_str) = extract_json(trimmed) {
        if let Ok(tc) = serde_json::from_str::<ToolCall>(&json_str) {
            if tc.tool.is_empty() {
                return Ok(PlannerOutput::Response(trimmed.to_string()));
            }
            return Ok(PlannerOutput::ToolCall(tc));
        }
    }

    Ok(PlannerOutput::Response(trimmed.to_string()))
}

fn extract_json(s: &str) -> Option<&str> {
    // 尝试找 ```json 块
    if let Some(start) = s.find("```json") {
        let rest = &s[start + 7..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim());
        }
    }

    // 尝试找 { 开头的 JSON
    if let Some(start) = s.find('{') {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, ch) in s[start..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&s[start..=start + i]);
                    }
                }
                _ => {}
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_llm_output_response() {
        let output = "Hello, how can I help you?";
        let result = parse_llm_output(output).unwrap();
        match result {
            PlannerOutput::Response(s) => assert_eq!(s, output),
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn test_parse_llm_output_tool_call() {
        let output = r#"{"tool": "cat", "args": {"path": "test.txt"}}"#;
        let result = parse_llm_output(output).unwrap();
        match result {
            PlannerOutput::ToolCall(tc) => {
                assert_eq!(tc.tool, "cat");
            }
            _ => panic!("Expected ToolCall"),
        }
    }
}
