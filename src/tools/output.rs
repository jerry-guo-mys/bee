//! 统一的结构化工具输出封装。

use serde_json::{json, Value};

pub fn structured(
    tool: &str,
    summary: impl Into<String>,
    sufficient_to_answer: bool,
    data: Value,
) -> Result<String, String> {
    serde_json::to_string_pretty(&json!({
        "tool": tool,
        "summary": summary.into(),
        "sufficient_to_answer": sufficient_to_answer,
        "data": data,
    }))
    .map_err(|e| format!("Serialize failed: {}", e))
}
