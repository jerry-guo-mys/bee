//! Organization 领域模块
//!
//! 组织 (Organization) 是 SaaS 多租户系统的第二级聚合根，
//! 隶属于租户 (Tenant)，管理团队和成员。

pub mod entity;
pub mod event;
pub mod repository;
pub mod value_object;

// 重新导出常用类型
pub use entity::Organization;
pub use event::{
    OrganizationCreated, OrganizationDeleted, OrganizationEvent, OrganizationUpdated,
};
pub use repository::{InMemoryOrganizationRepository, OrganizationRepository};
pub use value_object::{
    OrganizationError, OrganizationId, OrganizationName, OrganizationSlug,
};
