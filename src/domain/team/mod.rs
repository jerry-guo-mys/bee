//! Team 领域模块
//!
//! 团队 (Team) 是 SaaS 多租户系统的第三级聚合根，
//! 隶属于组织 (Organization)，管理团队成员和具体任务。

pub mod entity;
pub mod event;
pub mod repository;
pub mod value_object;

// 重新导出常用类型
pub use entity::Team;
pub use event::{TeamCreated, TeamDeleted, TeamEvent, TeamUpdated};
pub use repository::{InMemoryTeamRepository, TeamRepository};
pub use crate::domain::service::TeamDomainService;
pub use value_object::{TeamCode, TeamError, TeamId, TeamName};
