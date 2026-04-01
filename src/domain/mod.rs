//! 领域层：核心业务逻辑
//!
//! 包含三个主要子域：
//! - **cognitive**: 认知领域（Planner, Critic, ReAct, Context）
//! - **tool**: 工具领域（Tool, Registry, Executor, Policy）
//! - **memory**: 记忆领域（Conversation, Working, LongTerm, Store）

pub mod cognitive;
pub mod common;
pub mod event;
pub mod member;
pub mod memory;
pub mod organization;
pub mod service;
pub mod session;
pub mod team;
pub mod tenant;
pub mod tool;

// 重新导出常用类型
pub use cognitive::{
    context::ContextManager,
    critic::{Critic, CriticResult, CriticReview},
    planner::{Planner, PlannerOutput, ToolCall},
    react::{ReactResult, ReactSession},
};
pub use common::*;
pub use member::{
    MemberDomainError, MemberEvent, Membership, MembershipFilter, MembershipRepository, ToolId,
    ToolPolicy, ToolRiskLevel, UserEmail,
};
pub use memory::{conversation::ConversationMemory, working::WorkingMemory, Message};
pub use organization::{Organization, OrganizationError, OrganizationId, OrganizationRepository};
pub use service::{
    MemberDomainService, PermissionCheckService, PermissionError, TenantDomainService,
    ToolPolicyBuilder, ToolPolicyError, ToolPolicyService,
};
pub use team::{Team, TeamError, TeamId, TeamRepository};
pub use tool::{
    executor::ToolExecutor, metadata::ToolMetadata, registry::ToolRegistry, trait_::Tool,
};
