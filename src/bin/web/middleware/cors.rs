//! CORS 中间件

use axum::{
    http::{header, HeaderValue, Method},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

/// 设置 CORS
pub fn setup_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any)
        .expose_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
        ])
        .max_age(std::time::Duration::from_secs(3600))
}

/// CORS 配置
#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<Method>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub max_age: std::time::Duration,
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ],
            allowed_headers: vec!["*".to_string()],
            expose_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
            ],
            max_age: std::time::Duration::from_secs(3600),
            allow_credentials: false,
        }
    }
}

impl CorsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    pub fn with_methods(mut self, methods: Vec<Method>) -> Self {
        self.allowed_methods = methods;
        self
    }

    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.allowed_headers = headers;
        self
    }

    pub fn to_layer(&self) -> CorsLayer {
        let mut layer = CorsLayer::new();

        // 设置允许的来源
        if self.allowed_origins.contains(&"*".to_string()) {
            layer = layer.allow_origin(Any);
        } else {
            for origin in &self.allowed_origins {
                if let Ok(header_value) = HeaderValue::from_str(origin) {
                    layer = layer.allow_origin(header_value);
                }
            }
        }

        // 设置允许的方法
        layer = layer.allow_methods(self.allowed_methods.clone());

        // 设置允许的头部
        if self.allowed_headers.contains(&"*".to_string()) {
            layer = layer.allow_headers(Any);
        } else {
            for header in &self.allowed_headers {
                if let Ok(header_value) = HeaderValue::from_str(header) {
                    layer = layer.allow_headers(header_value);
                }
            }
        }

        // 设置暴露的头部
        for header in &self.expose_headers {
            if let Ok(header_value) = HeaderValue::from_str(header) {
                layer = layer.expose_headers(header_value);
            }
        }

        // 设置 max age
        layer = layer.max_age(self.max_age);

        // 设置 credentials
        if self.allow_credentials {
            layer = layer.allow_credentials(true);
        }

        layer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::new();
        assert!(config.allowed_origins.contains(&"*".to_string()));
        assert_eq!(config.allowed_methods.len(), 5);
    }

    #[test]
    fn test_cors_config_builder() {
        let config = CorsConfig::new()
            .with_origins(vec!["https://example.com".to_string()])
            .with_methods(vec![Method::GET, Method::POST]);

        assert_eq!(config.allowed_origins.len(), 1);
        assert_eq!(config.allowed_methods.len(), 2);
    }
}
