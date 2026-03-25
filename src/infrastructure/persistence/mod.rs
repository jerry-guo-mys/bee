//! 持久化存储模块
//!
//! 提供细粒度锁的存储实现

pub mod locking;

pub use locking::{
    FineGrainedLockStore, FineGrainedReadGuard, FineGrainedWriteGuard,
    LockError, LockStats, ShardedMap,
};
