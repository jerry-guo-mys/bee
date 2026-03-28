//! 持久化存储模块
//!
//! 提供细粒度锁的存储实现和 PostgreSQL 数据库连接

pub mod locking;

#[cfg(feature = "async-sqlite")]
pub mod postgres;

pub use locking::{
    FineGrainedLockStore, FineGrainedReadGuard, FineGrainedWriteGuard,
    LockError, LockStats, ShardedMap,
};

#[cfg(feature = "async-sqlite")]
pub use postgres::{load_database_connection, PostgresConnection};
