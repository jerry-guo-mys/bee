//! 领域服务模块
//!
//! 领域服务协调多个聚合根或 Repository 完成复杂的业务操作。

pub mod tenant_service;

pub use tenant_service::TenantDomainService;
