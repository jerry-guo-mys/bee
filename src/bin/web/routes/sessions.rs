//! Sessions 路由模块

use axum::{
    routing::{get, post, delete},
    Router,
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use super::WebAppState;

/// 创建 Sessions 路由
pub fn router(state: WebAppState) -> Router<WebAppState> {
    Router::new()
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions", post(create_session))
        .route("/api/sessions/:session_id", get(get_session))
        .route("/api/sessions/:session_id", delete(delete_session))
        .route("/api/sessions/:session_id/messages", get(get_messages))
        .route("/api/sessions/:session_id/messages", post(add_message))
        .with_state(state)
}

/// Session 信息
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub agent_id: Option<String>,
    pub title: String,
    pub message_count: usize,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: serde_json::Value,
}

/// 消息
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// 创建 Session 请求
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub agent_id: Option<String>,
    pub title: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// 添加消息请求
#[derive(Debug, Deserialize)]
pub struct AddMessageRequest {
    pub role: MessageRole,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

/// 列出所有 Session
async fn list_sessions(
    State(_state): State<WebAppState>,
) -> Json<Vec<SessionInfo>> {
    // TODO: 实现 Session 列表
    Json(vec![])
}

/// 创建 Session
async fn create_session(
    State(_state): State<WebAppState>,
    Json(request): Json<CreateSessionRequest>,
) -> Json<SessionInfo> {
    // TODO: 实现创建 Session 逻辑
    SessionInfo {
        id: uuid::Uuid::new_v4().to_string(),
        agent_id: request.agent_id,
        title: request.title.unwrap_or_else(|| "New Session".to_string()),
        message_count: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        metadata: request.metadata.unwrap_or(serde_json::Value::Null),
    }.into()
}

/// 获取 Session 详情
async fn get_session(
    State(_state): State<WebAppState>,
    Path(session_id): Path<String>,
) -> Json<SessionInfo> {
    // TODO: 实现获取 Session 详情
    SessionInfo {
        id: session_id,
        agent_id: None,
        title: "Session".to_string(),
        message_count: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        metadata: serde_json::Value::Null,
    }.into()
}

/// 删除 Session
async fn delete_session(
    State(_state): State<WebAppState>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: 实现删除 Session 逻辑
    serde_json::json!({
        "success": true,
        "message": format!("Session {} deleted", session_id)
    }).into()
}

/// 获取消息列表
async fn get_messages(
    State(_state): State<WebAppState>,
    Path(session_id): Path<String>,
) -> Json<Vec<Message>> {
    // TODO: 实现获取消息列表
    Json(vec![])
}

/// 添加消息
async fn add_message(
    State(_state): State<WebAppState>,
    Path(session_id): Path<String>,
    Json(request): Json<AddMessageRequest>,
) -> Json<Message> {
    // TODO: 实现添加消息逻辑
    Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id,
        role: request.role,
        content: request.content,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata: request.metadata,
    }.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let message = Message {
            id: "msg-123".to_string(),
            session_id: "session-123".to_string(),
            role: MessageRole::User,
            content: "Hello".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            metadata: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_message_role_deserialize() {
        let json = r#""assistant""#;
        let role: MessageRole = serde_json::from_str(json).unwrap();
        assert!(matches!(role, MessageRole::Assistant));
    }

    #[test]
    fn test_create_session_request() {
        let json = r#"{
            "agent_id": "agent-123",
            "title": "Test Session",
            "metadata": {"key": "value"}
        }"#;
        let request: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.agent_id, Some("agent-123".to_string()));
        assert_eq!(request.title, Some("Test Session".to_string()));
    }
}
