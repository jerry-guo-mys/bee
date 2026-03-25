//! 健康检查路由

use axum::{
    routing::get,
    Router,
    Json,
    extract::State,
};
use serde::{Deserialize, Serialize};

use super::WebAppState;

/// 健康状态响应
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: String,
}

/// 详细健康状态
#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedHealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: String,
    pub components: HealthComponents,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthComponents {
    pub database: HealthStatus,
    pub llm: HealthStatus,
    pub memory: HealthStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub message: Option<String>,
}

/// 创建健康检查路由
pub fn router() -> Router<WebAppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/live", get(liveness_probe))
        .route("/health/ready", get(readiness_probe))
        .route("/health/detailed", get(detailed_health))
}

/// 基础健康检查
async fn health_check(State(state): State<WebAppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// 存活探针（Kubernetes）
async fn liveness_probe() -> &'static str {
    "OK"
}

/// 就绪探针（Kubernetes）
async fn readiness_probe(State(state): State<WebAppState>) -> &'static str {
    // 这里可以检查依赖服务是否可用
    if state.config.database.path.is_empty() {
        "Service Unavailable"
    } else {
        "OK"
    }
}

/// 详细健康检查
async fn detailed_health(State(state): State<WebAppState>) -> Json<DetailedHealthResponse> {
    Json(DetailedHealthResponse {
        status: "healthy".to_string(),
        version: state.version.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        components: HealthComponents {
            database: HealthStatus {
                status: "healthy".to_string(),
                message: None,
            },
            llm: HealthStatus {
                status: "healthy".to_string(),
                message: None,
            },
            memory: HealthStatus {
                status: "healthy".to_string(),
                message: None,
            },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));
    }
}
