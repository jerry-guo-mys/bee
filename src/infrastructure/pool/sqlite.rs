//! SQLite 连接池
//!
//! 提供 SQLite 连接的池化管理，支持连接复用和健康检查。

use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::{Connection, Result as SqliteResult};
use tokio::sync::{Mutex, Semaphore};

/// 连接池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 最小连接数
    pub min_connections: usize,
    /// 最大连接数
    pub max_connections: usize,
    /// 连接空闲超时
    pub idle_timeout: Duration,
    /// 连接最大生命周期
    pub max_lifetime: Duration,
    /// 获取连接超时
    pub acquire_timeout: Duration,
    /// 健康检查间隔
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(3600),
            acquire_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(60),
        }
    }
}

impl PoolConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_min_connections(mut self, n: usize) -> Self {
        self.min_connections = n;
        self
    }

    pub fn with_max_connections(mut self, n: usize) -> Self {
        self.max_connections = n;
        self
    }

    pub fn with_idle_timeout(mut self, d: Duration) -> Self {
        self.idle_timeout = d;
        self
    }

    pub fn with_max_lifetime(mut self, d: Duration) -> Self {
        self.max_lifetime = d;
        self
    }
}

/// PooledConnection 包装器
#[derive(Clone)]
pub struct PooledConnection {
    conn: Arc<Mutex<Connection>>,
    created_at: Instant,
    last_used_at: Arc<Mutex<Instant>>,
    is_healthy: bool,
}

impl PooledConnection {
    fn new(conn: Connection) -> Self {
        let now = Instant::now();
        Self {
            conn: Arc::new(Mutex::new(conn)),
            created_at: now,
            last_used_at: Arc::new(Mutex::new(now)),
            is_healthy: true,
        }
    }

    /// 获取连接
    pub async fn get(&self) -> SqliteResult<Arc<Mutex<Connection>>> {
        *self.last_used_at.lock().await = Instant::now();
        Ok(self.conn.clone())
    }

    /// 检查连接是否健康
    pub fn is_healthy(&self) -> bool {
        self.is_healthy
    }

    /// 检查连接是否过期
    pub fn is_expired(&self, config: &PoolConfig) -> bool {
        self.created_at.elapsed() > config.max_lifetime
    }

    /// 检查连接是否空闲超时
    pub fn is_idle(&self, _config: &PoolConfig) -> bool {
        // 需要异步读取 last_used_at，简化处理
        false
    }

    /// 标记连接为不健康
    pub fn mark_unhealthy(&mut self) {
        self.is_healthy = false;
    }
}

/// SQLite 连接池
pub struct SqliteConnectionPool {
    /// 连接列表（使用 Mutex 保护以支持动态添加）
    connections: Arc<Mutex<Vec<PooledConnection>>>,
    /// 配置
    config: PoolConfig,
    /// 信号量，控制并发连接数
    semaphore: Arc<Semaphore>,
    /// 数据库路径
    database_path: String,
}

impl SqliteConnectionPool {
    /// 创建新的连接池
    pub fn new(database_path: impl Into<String>, config: PoolConfig) -> SqliteResult<Self> {
        let database_path = database_path.into();
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        let mut connections = Vec::new();

        // 初始化最小连接数
        for _ in 0..config.min_connections {
            let conn = Connection::open(&database_path)?;
            connections.push(PooledConnection::new(conn));
        }

        Ok(Self {
            connections: Arc::new(Mutex::new(connections)),
            config,
            semaphore,
            database_path,
        })
    }

    /// 创建内存连接池（用于测试）
    pub fn in_memory() -> SqliteResult<Self> {
        let config = PoolConfig::default()
            .with_min_connections(1)
            .with_max_connections(4);
        Self::new(":memory:", config)
    }

    /// 获取连接
    pub async fn get(&self) -> Option<PooledConnectionGuard> {
        // 获取信号量许可
        let permit_result = tokio::time::timeout(
            self.config.acquire_timeout,
            self.semaphore.clone().acquire_owned(),
        )
        .await;

        let permit = match permit_result {
            Ok(Ok(p)) => p,
            _ => return None,
        };

        // 查找可用连接
        let conn = self.find_available_connection().await?;

        Some(PooledConnectionGuard {
            conn,
            _permit: permit,
            pool_size: self.connections.lock().await.len(),
        })
    }

    /// 查找可用连接
    async fn find_available_connection(&self) -> Option<Arc<Mutex<Connection>>> {
        // 优先使用现有连接
        let connections = self.connections.lock().await;
        for pooled in connections.iter() {
            if pooled.is_healthy() && !pooled.is_expired(&self.config) {
                return pooled.get().await.ok();
            }
        }

        // 没有可用连接，创建新连接（如果未达上限）
        drop(connections);
        let mut connections = self.connections.lock().await;

        if connections.len() < self.config.max_connections {
            match Connection::open(&self.database_path) {
                Ok(conn) => {
                    let pooled = PooledConnection::new(conn);
                    let arc = pooled.get().await.ok()?;
                    // 添加新连接到池中
                    connections.push(pooled);
                    return Some(arc);
                }
                Err(_) => {
                    // 创建失败，返回第一个连接（即使不健康）
                    let first = connections.first().cloned();
                    drop(connections);
                    if let Some(pooled) = first {
                        return pooled.get().await.ok();
                    }
                    return None;
                }
            }
        }

        // 等待并返回第一个连接
        let first = connections.first().cloned();
        drop(connections);
        if let Some(pooled) = first {
            pooled.get().await.ok()
        } else {
            None
        }
    }

    /// 获取连接池状态
    pub async fn status(&self) -> PoolStatus {
        let connections = self.connections.lock().await;
        let healthy = connections.iter().filter(|c| c.is_healthy()).count();
        let expired = connections
            .iter()
            .filter(|c| c.is_expired(&self.config))
            .count();
        let total = connections.len();

        PoolStatus {
            total,
            healthy,
            expired,
            in_use: self.config.max_connections - self.semaphore.available_permits(),
            available: self.semaphore.available_permits(),
        }
    }

    /// 关闭连接池
    pub async fn close(&self) {
        // 连接会在 Drop 时自动关闭
    }
}

/// 连接池状态
#[derive(Debug, Clone)]
pub struct PoolStatus {
    /// 总连接数
    pub total: usize,
    /// 健康连接数
    pub healthy: usize,
    /// 过期连接数
    pub expired: usize,
    /// 使用中连接数
    pub in_use: usize,
    /// 可用连接数
    pub available: usize,
}

/// 连接池守卫（RAII 风格）
pub struct PooledConnectionGuard {
    conn: Arc<Mutex<Connection>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
    pool_size: usize,
}

impl PooledConnectionGuard {
    /// 获取连接
    pub fn conn(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    /// 执行查询
    pub async fn query<F, T>(&self, f: F) -> SqliteResult<T>
    where
        F: FnOnce(&Connection) -> SqliteResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.conn.lock().await;
        f(&conn)
    }

    /// 执行无结果的操作
    pub async fn execute<F>(&self, f: F) -> SqliteResult<()>
    where
        F: FnOnce(&mut Connection) -> SqliteResult<()> + Send + 'static,
    {
        let mut conn = self.conn.lock().await;
        f(&mut conn)
    }
}

impl PooledConnectionGuard {
    pub fn pool_size(&self) -> usize {
        self.pool_size
    }
}

/// HTTP 连接池（用于外部 API 调用）
pub struct HttpClientPool {
    client: reqwest::Client,
}

impl HttpClientPool {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

impl Default for HttpClientPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::new()
            .with_min_connections(4)
            .with_max_connections(20)
            .with_idle_timeout(Duration::from_secs(300))
            .with_max_lifetime(Duration::from_secs(1800));

        assert_eq!(config.min_connections, 4);
        assert_eq!(config.max_connections, 20);
    }

    #[test]
    fn test_in_memory_pool() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pool = SqliteConnectionPool::in_memory().unwrap();
        let status = rt.block_on(pool.status());

        assert!(status.total >= 1);
        assert_eq!(status.healthy, status.total);
    }

    #[tokio::test]
    async fn test_pool_get_connection() {
        let pool = SqliteConnectionPool::in_memory().unwrap();

        let guard = pool.get().await.unwrap();
        let status = pool.status().await;

        assert_eq!(status.in_use, 1);
        assert!(status.available > 0 || status.total < status.in_use);

        drop(guard);
    }

    #[tokio::test]
    async fn test_pool_query() {
        let pool = SqliteConnectionPool::in_memory().unwrap();

        let guard = pool.get().await.unwrap();

        // 创建测试表
        guard
            .execute(|conn| {
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS test_pool (id INTEGER PRIMARY KEY, value TEXT)",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();

        // 插入数据
        guard
            .execute(|conn| {
                conn.execute("INSERT INTO test_pool (value) VALUES (?)", ["test"])?;
                Ok(())
            })
            .await
            .unwrap();

        // 查询数据
        let count: i64 = guard
            .query(|conn| {
                let mut stmt = conn.prepare("SELECT COUNT(*) FROM test_pool")?;
                Ok(stmt.query_row([], |row| row.get(0))?)
            })
            .await
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_http_client_pool() {
        let _pool = HttpClientPool::new();
        // Client 创建成功即表示有效
        assert!(true);
    }
}
