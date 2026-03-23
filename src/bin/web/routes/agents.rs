//! Agents 路由模块

use axum::{
    routing::{get, post, put, delete},
    Router,
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use super::WebAppState;

/// 创建 Agents 路由
pub fn router(state: WebAppState) -> Router<WebAppState> {
    Router::new()
        .route("/api/agents", get(list_agents))
        .route("/api/agents", post(create_agent))
        .route("/api/agents/:agent_id", get(get_agent))
        .route("/api/agents/:agent_id", put(update_agent))
        .route("/api/agents/:agent_id", delete(delete_agent))
        .route("/api/agents/:agent_id/status", get(get_agent_status))
        .route("/api/agents/:agent_id/start", post(start_agent))
        .route("/api/agents/:agent_id/stop", post(stop_agent))
        .with_state(state)
}

/// Agent 信息
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config: AgentConfig,
    pub status: AgentStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// Agent 配置
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    pub temperature: f32,
    pub max_iterations: usize,
    pub tools: Vec<String>,
    pub memory_enabled: bool,
}

/// Agent 状态
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AgentStatus {
    Created,
    Running,
    Paused,
    Stopped,
    Error(String),
}

/// 创建 Agent 请求
#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub description: String,
    pub config: AgentConfig,
}

/// 更新 Agent 请求
#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<AgentConfig>,
}

/// Agent 状态响应
#[derive(Debug, Serialize)]
pub struct AgentStatusResponse {
    pub agent_id: String,
    pub status: AgentStatus,
    pub current_task: Option<String>,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub uptime_seconds: Option<u64>,
}

/// 列出所有 Agent
async fn list_agents(
    State(_state): State<WebAppState>,
) -> Json<Vec<AgentInfo>> {
    // TODO: 实现 Agent 列表
    Json(vec![])
}

/// 创建 Agent
async fn create_agent(
    State(_state): State<WebAppState>,
    Json(request): Json<CreateAgentRequest>,
) -> Json<AgentInfo> {
    // TODO: 实现创建 Agent 逻辑
    AgentInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        config: request.config,
        status: AgentStatus::Created,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    }.into()
}

/// 获取 Agent 详情
async fn get_agent(
    State(_state): State<WebAppState>,
    Path(agent_id): Path<String>,
) -> Json<AgentInfo> {
    // TODO: 实现获取 Agent 详情
    AgentInfo {
        id: agent_id,
        name: "Agent".to_string(),
        description: "Agent description".to_string(),
        config: AgentConfig {
            model: "default".to_string(),
            temperature: 0.7,
            max_iterations: 10,
            tools: vec![],
            memory_enabled: true,
        },
        status: AgentStatus::Created,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    }.into()
}

/// 更新 Agent
async fn update_agent(
    State(_state): State<WebAppState>,
    Path(_agent_id): Path<String>,
    Json(_request): Json<UpdateAgentRequest>,
) -> Json<AgentInfo> {
    // TODO: 实现更新 Agent 逻辑
    AgentInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Updated Agent".to_string(),
        description: "Description".to_string(),
        config: AgentConfig {
            model: "default".to_string(),
            temperature: 0.7,
            max_iterations: 10,
            tools: vec![],
            memory_enabled: true,
        },
        status: AgentStatus::Created,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    }.into()
}

/// 删除 Agent
async fn delete_agent(
    State(_state): State<WebAppState>,
    Path(agent_id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: 实现删除 Agent 逻辑
    serde_json::json!({
        "success": true,
        "message": format!("Agent {} deleted", agent_id)
    }).into()
}

/// 获取 Agent 状态
async fn get_agent_status(
    State(_state): State<WebAppState>,
    Path(agent_id): Path<String>,
) -> Json<AgentStatusResponse> {
    // TODO: 实现获取状态
    AgentStatusResponse {
        agent_id,
        status: AgentStatus::Stopped,
        current_task: None,
        tasks_completed: 0,
        tasks_failed: 0,
        uptime_seconds: None,
    }.into()
}

/// 启动 Agent
async fn start_agent(
    State(_state): State<WebAppState>,
    Path(agent_id): Path<String>,
) -> Json<AgentStatusResponse> {
    // TODO: 实现启动逻辑
    AgentStatusResponse {
        agent_id,
        status: AgentStatus::Running,
        current_task: None,
        tasks_completed: 0,
        tasks_failed: 0,
        uptime_seconds: Some(0),
    }.into()
}

/// 停止 Agent
async fn stop_agent(
    State(_state): State<WebAppState>,
    Path(agent_id): Path<String>,
) -> Json<AgentStatusResponse> {
    // TODO: 实现停止逻辑
    AgentStatusResponse {
        agent_id,
        status: AgentStatus::Stopped,
        current_task: None,
        tasks_completed: 0,
        tasks_failed: 0,
        uptime_seconds: Some(100),
    }.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_serialization() {
        let config = AgentConfig {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_iterations: 10,
            tools: vec!["search".to_string()],
            memory_enabled: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_create_agent_request_deserialize() {
        let json = r#"{
            "name": "Test Agent",
            "description": "A test agent",
            "config": {
                "model": "gpt-4",
                "temperature": 0.5,
                "max_iterations": 5,
                "tools": [],
                "memory_enabled": true
            }
        }"#;
        let request: CreateAgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Test Agent");
        assert_eq!(request.config.model, "gpt-4");
    }
}
