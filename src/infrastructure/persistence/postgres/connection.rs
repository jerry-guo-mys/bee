#![cfg(any(feature = "async-sqlite", feature = "postgres"))]

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use std::time::Duration;

/// PostgreSQL 连接封装
pub struct PostgresConnection {
    pool: Arc<PgPool>,
}

impl PostgresConnection {
    /// 创建新连接池（异步）
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// 获取连接池
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run database migrations using compile-time validated SQLx migrate
    /// Returns sqlx::Error which preserves migration failure information
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        // 使用 sqlx::migrate! 宏在编译时加载迁移文件
        sqlx::migrate!("./migrations")
            .run(&*self.pool)
            .await
            .map_err(|e| sqlx::Error::Protocol(format!("Migration error: {}", e).into()))
    }
}

/// 从环境变量加载数据库连接
pub async fn load_database_connection() -> Result<PostgresConnection, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL environment variable not set")?;

    let conn = PostgresConnection::new(&database_url).await?;
    Ok(conn)
}
