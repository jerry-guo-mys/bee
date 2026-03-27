//! HTTP 连接池
//!
//! 提供 HTTP 客户端的池化管理，支持连接复用和超时控制。

use std::time::Duration;

use reqwest::{Client, ClientBuilder};

/// HTTP 连接池配置
#[derive(Debug, Clone)]
pub struct HttpClientPoolConfig {
    /// 每个主机的最大空闲连接数
    pub pool_max_idle_per_host: usize,
    /// 连接空闲超时
    pub pool_idle_timeout: Duration,
    /// 请求超时
    pub timeout: Duration,
    /// 连接超时
    pub connect_timeout: Duration,
    /// 重试次数
    pub max_retries: u32,
}

impl Default for HttpClientPoolConfig {
    fn default() -> Self {
        Self {
            pool_max_idle_per_host: 10,
            pool_idle_timeout: Duration::from_secs(90),
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            max_retries: 3,
        }
    }
}

impl HttpClientPoolConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pool_size(mut self, n: usize) -> Self {
        self.pool_max_idle_per_host = n;
        self
    }

    pub fn with_timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }
}

/// HTTP 连接池
pub struct HttpClientPool {
    client: Client,
    config: HttpClientPoolConfig,
}

impl HttpClientPool {
    /// 创建新的 HTTP 连接池
    pub fn new(config: HttpClientPoolConfig) -> Result<Self, reqwest::Error> {
        let client = ClientBuilder::new()
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(config.pool_idle_timeout)
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .build()?;

        Ok(Self { client, config })
    }

    /// 创建默认配置的 HTTP 连接池
    pub fn with_default_config() -> Result<Self, reqwest::Error> {
        Self::new(HttpClientPoolConfig::default())
    }

    /// 获取 HTTP 客户端
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// 获取配置
    pub fn config(&self) -> &HttpClientPoolConfig {
        &self.config
    }

    /// 执行 GET 请求（带重试）
    pub async fn get_with_retry(
        &self,
        url: &str,
    ) -> Result<reqwest::Response, HttpClientPoolError> {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            match self.client.get(url).send().await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    tracing::warn!(
                        "HTTP GET {} failed (attempt {}/{}): {}",
                        url,
                        attempt + 1,
                        self.config.max_retries,
                        last_error.as_ref().expect("logic error: last_error should be set")
                    );

                    if attempt < self.config.max_retries - 1 {
                        tokio::time::sleep(Duration::from_millis(100 * (1 << attempt))).await;
                    }
                }
            }
        }

        Err(HttpClientPoolError::RequestFailed(
            last_error.expect("retry loop should always set last_error")
        ))
    }

    /// 执行 POST 请求（带重试）
    pub async fn post_with_retry(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, HttpClientPoolError> {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            match self
                .client
                .post(url)
                .json(body)
                .send()
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    tracing::warn!(
                        "HTTP POST {} failed (attempt {}/{}): {}",
                        url,
                        attempt + 1,
                        self.config.max_retries,
                        last_error.as_ref().expect("logic error: last_error should be set")
                    );

                    if attempt < self.config.max_retries - 1 {
                        tokio::time::sleep(Duration::from_millis(100 * (1 << attempt))).await;
                    }
                }
            }
        }

        Err(HttpClientPoolError::RequestFailed(
            last_error.expect("retry loop should always set last_error")
        ))
    }

    /// 获取连接池状态
    pub fn status(&self) -> HttpClientPoolStatus {
        HttpClientPoolStatus {
            pool_max_idle_per_host: self.config.pool_max_idle_per_host,
            pool_idle_timeout: self.config.pool_idle_timeout,
            timeout: self.config.timeout,
        }
    }
}

impl Default for HttpClientPool {
    fn default() -> Self {
        Self::with_default_config().expect("Failed to create HTTP client pool with default config")
    }
}

/// HTTP 连接池状态
#[derive(Debug, Clone)]
pub struct HttpClientPoolStatus {
    pub pool_max_idle_per_host: usize,
    pub pool_idle_timeout: Duration,
    pub timeout: Duration,
}

/// HTTP 连接池错误
#[derive(Debug, thiserror::Error)]
pub enum HttpClientPoolError {
    #[error("Request failed: {0}")]
    RequestFailed(reqwest::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_pool_config() {
        let config = HttpClientPoolConfig::new()
            .with_pool_size(20)
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.pool_max_idle_per_host, 20);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_http_client_pool_creation() {
        let pool = HttpClientPool::with_default_config().expect("Failed to create HTTP client pool");
        // Client 创建成功即表示有效
        assert!(true);
    }

    #[test]
    fn test_http_client_pool_status() {
        let _pool = HttpClientPool::with_default_config().expect("Failed to create HTTP client pool");
        let _status = _pool.status();

        assert_eq!(_pool.config().pool_max_idle_per_host, 10);
        assert_eq!(_pool.config().pool_idle_timeout, Duration::from_secs(90));
        assert_eq!(_pool.config().timeout, Duration::from_secs(30));
    }
}
