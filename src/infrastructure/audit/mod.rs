//! 审计基础设施

pub mod logger;
pub mod repository;

pub use logger::{AuditError, AuditLog, AuditLogger};
pub use repository::AuditLogRepository;

#[cfg(feature = "postgres")]
pub use repository::postgres::PostgresAuditLogRepository;
