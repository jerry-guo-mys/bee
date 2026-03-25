//! 领域层：核心业务逻辑
//!
//! 包含三个主要子域：
//! - **cognitive**: 认知领域（Planner, Critic, ReAct, Context）
//! - **tool**: 工具领域（Tool, Registry, Executor, Policy）
//! - **memory**: 记忆领域（Conversation, Working, LongTerm, Store）

pub mod cognitive;
pub mod event;
pub mod memory;
pub mod session;
pub mod tool;

// 重新导出常用类型
pub use cognitive::{
    context::ContextManager,
    critic::{Critic, CriticResult, CriticReview},
    planner::{Planner, PlannerOutput, ToolCall},
    react::{ReactResult, ReactSession},
};
pub use memory::{
    conversation::ConversationMemory,
    Message,
    working::WorkingMemory,
};
pub use tool::{
    executor::ToolExecutor,
    metadata::ToolMetadata,
    registry::ToolRegistry,
    trait_::Tool,
};
