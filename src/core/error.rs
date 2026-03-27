//! Agent 错误类型与恢复动作
//!
//! 与 RecoveryEngine 配合：根据 AgentError 决定 RetryWithPrompt / SummarizeAndPrune / AskUser / Abort 等。

use thiserror::Error;

use crate::llm::LlmError;

/// Agent 运行过程中可能出现的错误（网络、解析、工具、路径逃逸等）
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Cancelled by user")]
    Cancelled,

    #[error("Network timeout: {0}")]
    NetworkTimeout(String),

    #[error("Context window exceeded")]
    ContextWindowExceeded,

    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Tool timeout: {0}")]
    ToolTimeout(String),

    /// LLM 幻觉出不存在的工具名
    #[error("Hallucinated tool: {0} (model made up this tool name)")]
    HallucinatedTool(String),

    /// 工具应该存在但未找到（如技能文件缺失）
    #[error("Tool not found: {0} (tool exists but not loaded)")]
    ToolNotFound(String),

    #[error("LLM error: {0}")]
    LlmError(#[from] LlmError),

    /// 保留用于未来扩展：当 Agent 直接建议降级模型时（非 LlmError 触发）
    #[error("Suggest downgrade model: {0}")]
    SuggestDowngradeModel(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Path escape attempt: {0}")]
    PathEscape(String),

    #[error("Orchestration failed: {0}")]
    OrchestrationFailed(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),
}

/// 恢复引擎根据错误类型给出的建议动作
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// 将提示注入下一轮，让 LLM 重试（如 JSON 格式错误）
    RetryWithPrompt(String),
    /// 压缩上下文后继续（如超长上下文）
    SummarizeAndPrune,
    /// 需要用户决策（如幻觉工具、超时）
    AskUser(String),
    /// 降级到更轻量模型（如 LLM 持续失败时）
    DowngradeModel,
    /// 终止当前任务（如用户取消或不可恢复错误）
    Abort,
}
