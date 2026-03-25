//! 连接池模块
//!
//! 提供数据库和 HTTP 客户端的池化管理

pub mod http;
pub mod sqlite;

pub use http::{HttpClientPool, HttpClientPoolConfig, HttpClientPoolStatus, HttpClientPoolError};
pub use sqlite::{
    PoolConfig, PoolStatus, PooledConnection, PooledConnectionGuard, SqliteConnectionPool,
};
