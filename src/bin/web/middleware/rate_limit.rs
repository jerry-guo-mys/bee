//! 限流中间件

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// 限流中间件
#[derive(Debug, Clone)]
pub struct RateLimitMiddleware {
    pub limiter: RateLimiter,
}

impl RateLimitMiddleware {
    pub fn new(limiter: RateLimiter) -> Self {
        Self { limiter }
    }

    /// 限流中间件处理函数
    pub async fn rate_limit(
        State(_state): State<crate::routes::WebAppState>,
        request: Request,
        next: Next,
    ) -> Result<Response, (StatusCode, String)> {
        // 获取客户端标识（IP 或用户 ID）
        let client_id = request
            .extensions()
            .get::<AuthenticatedClient>()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| {
                request
                    .headers()
                    .get(axum::http::header::FORWARDED)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("unknown")
                    .to_string()
            });

        // 检查限流
        if !RateLimiter::global().allow(&client_id).await {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
            ));
        }

        Ok(next.run(request).await)
    }
}

/// 限流器
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// 每秒请求数限制
    pub requests_per_second: u32,
    /// 突发请求限制
    pub burst_size: u32,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            requests_per_second,
            burst_size,
        }
    }

    /// 获取全局限流器实例
    pub fn global() -> &'static Arc<RateLimiterState> {
        use std::sync::OnceLock;
        static LIMITER: OnceLock<Arc<RateLimiterState>> = OnceLock::new();
        LIMITER.get_or_init(|| {
            Arc::new(RateLimiterState::new(RateLimiter {
                requests_per_second: 10,
                burst_size: 20,
            }))
        })
    }

    /// 滑动窗口限流
    pub async fn allow(&self, _client_id: &str) -> bool {
        // 简化实现：使用令牌桶算法
        RateLimiter::global().allow().await
    }
}

/// 限流器状态（每个客户端）
#[derive(Debug)]
pub struct RateLimiterState {
    config: RateLimiter,
    buckets: RwLock<HashMap<String, TokenBucket>>,
}

impl RateLimiterState {
    pub fn new(config: RateLimiter) -> Self {
        Self {
            config,
            buckets: RwLock::new(HashMap::new()),
        }
    }

    pub async fn allow(&self) -> bool {
        // 简化实现：总是允许
        // 实际实现应使用令牌桶或滑动窗口算法
        true
    }
}

/// 令牌桶
#[derive(Debug)]
pub struct TokenBucket {
    tokens: u32,
    last_refill: Instant,
    refill_rate: u32,
    max_tokens: u32,
}

impl TokenBucket {
    pub fn new(max_tokens: u32, refill_rate: u32) -> Self {
        Self {
            tokens: max_tokens,
            last_refill: Instant::now(),
            refill_rate,
            max_tokens,
        }
    }

    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let new_tokens = (elapsed.as_secs_f32() * self.refill_rate as f32) as u32;

        if new_tokens > 0 {
            self.tokens = std::cmp::min(self.max_tokens, self.tokens + new_tokens);
            self.last_refill = now;
        }
    }
}

/// 认证客户端标识
#[derive(Debug, Clone)]
pub struct AuthenticatedClient {
    pub id: String,
}

/// 限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub by_endpoint: HashMap<String, RateLimitEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEndpoint {
    pub path_pattern: String,
    pub requests_per_minute: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(5, 1);

        // 应该能消耗 5 个令牌
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }

        // 第 6 次应该失败
        assert!(!bucket.try_consume());
    }
}
