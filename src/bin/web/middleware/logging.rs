//! 日志中间件

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn, error};

/// 日志中间件
#[derive(Debug, Clone)]
pub struct LoggingMiddleware {
    pub enabled: bool,
    pub log_headers: bool,
    pub log_body: bool,
}

impl LoggingMiddleware {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            log_headers: false,
            log_body: false,
        }
    }

    pub fn with_headers(mut self, log: bool) -> Self {
        self.log_headers = log;
        self
    }

    pub fn with_body(mut self, log: bool) -> Self {
        self.log_body = log;
        self
    }

    /// 日志中间件处理函数
    pub async fn log_request(
        State(_state): State<crate::routes::WebAppState>,
        request: Request,
        next: Next,
    ) -> Result<Response, (StatusCode, String)> {
        let start = Instant::now();

        // 记录请求信息
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();

        // 添加请求 ID
        let request_id = uuid::Uuid::new_v4().to_string();
        let mut request = request;
        request.headers_mut().insert(
            "X-Request-ID",
            request_id.parse().unwrap(),
        );

        // 执行请求
        let response = next.run(request).await;
        let duration = start.elapsed();

        // 记录响应信息
        let status = response.status();
        let response_headers = response.headers();

        // 根据状态码选择日志级别
        let log_fn = match status.as_u16() {
            0..=399 => info,
            400..=499 => warn,
            _ => error,
        };

        log_fn!(
            target: "http_log",
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration.as_millis(),
            request_id = %request_id,
            "HTTP request"
        );

        Ok(response)
    }
}

/// 结构化日志条目
#[derive(Debug, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub method: String,
    pub uri: String,
    pub status: u16,
    pub duration_ms: u128,
    pub request_id: String,
    pub client_ip: Option<String>,
}

impl LogEntry {
    pub fn new(
        method: &str,
        uri: &str,
        status: StatusCode,
        duration: std::time::Duration,
        request_id: &str,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            method: method.to_string(),
            uri: uri.to_string(),
            status: status.as_u16(),
            duration_ms: duration.as_millis(),
            request_id: request_id.to_string(),
            client_ip: None,
        }
    }

    pub fn with_client_ip(mut self, ip: String) -> Self {
        self.client_ip = Some(ip);
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// 请求跟踪扩展
#[derive(Debug, Clone)]
pub struct RequestTrace {
    pub request_id: String,
    pub span_id: String,
    pub trace_id: String,
}

impl RequestTrace {
    pub fn new(request_id: &str) -> Self {
        Self {
            request_id: request_id.to_string(),
            span_id: uuid::Uuid::new_v4().to_string(),
            trace_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry::new(
            "GET",
            "/api/health",
            StatusCode::OK,
            std::time::Duration::from_millis(10),
            "test-request-id",
        );

        let json = entry.to_json();
        assert!(json.contains("GET"));
        assert!(json.contains("/api/health"));
        assert!(json.contains("200"));
    }
}
