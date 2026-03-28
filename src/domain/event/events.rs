//! 领域事件定义

/// 领域事件
#[derive(Debug, Clone)]
pub enum DomainEvent {
    /// 会话创建
    SessionCreated(String),
    /// 会话完成
    SessionCompleted(String),
    /// 工具执行
    ToolExecuted { name: String, success: bool },
    /// 记忆更新
    MemoryUpdated(String),
    /// 错误发生
    Error(String),
    /// 自定义事件
    Custom(String),
}
