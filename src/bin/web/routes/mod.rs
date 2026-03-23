//! Web 路由模块
//!
//! 定义应用的所有 HTTP 路由

pub mod health;
pub mod chat;
pub mod tools;
pub mod agents;
pub mod sessions;

use axum::Router;
use std::sync::Arc;

use crate::config::AppConfig;

/// Web 应用状态
#[derive(Debug, Clone)]
pub struct WebAppState {
    pub config: Arc<AppConfig>,
    pub version: String,
}

impl WebAppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(config),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// 创建路由
pub fn create_router(state: WebAppState) -> Router {
    Router::new()
        // 健康检查
        .merge(health::router())
        // Chat API
        .merge(chat::router(state.clone()))
        // Tools API
        .merge(tools::router(state.clone()))
        // Agents API
        .merge(agents::router(state.clone()))
        // Sessions API
        .merge(sessions::router(state.clone()))
        .with_state(state)
}
