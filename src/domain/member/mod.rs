//! 成员领域模块
//!
//! 包含成员聚合的所有组件：
//! - entity: 聚合根（Membership）
//! - value_object: 值对象（UserEmail, ToolId, ToolRiskLevel, ToolPolicy）
//! - repository: 数据访问接口
//! - event: 领域事件

pub mod entity;
pub mod event;
pub mod repository;
pub mod value_object;

// 重新导出主要类型
pub use entity::{MemberDomainError, Membership};
pub use event::MemberEvent;
#[cfg(feature = "postgres")]
pub use repository::PostgresMembershipRepository;
pub use repository::{InMemoryMembershipRepository, MembershipFilter, MembershipRepository};
pub use crate::domain::service::MemberDomainService;
pub use value_object::{ToolId, ToolPolicy, ToolRiskLevel, UserEmail};
