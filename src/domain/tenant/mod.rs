//! 租户领域模块
//!
//! 租户 (Tenant) 是 SaaS 多租户系统的最高层级聚合根，
//! 管理组织、团队、成员和 Agent 的生命周期。

pub mod entity;
pub mod event;
pub mod repository;
pub mod value_object;

// 重新导出常用类型
pub use entity::Tenant;
pub use event::{
    DomainEvent,
    DomainEventPublisher,
    InMemoryEventPublisher,
    TenantArchived,
    TenantCreated,
    TenantDeleted,
    TenantEvent,
    TenantRestored,
    TenantSuspended,
};
pub use repository::{InMemoryTenantRepository, TenantRepository};
pub use value_object::{
    AgentId,
    MembershipId,
    OrganizationId,
    TeamId,
    TenantError,
    TenantId,
    TenantName,
    TenantSlug,
    UserId,
};
