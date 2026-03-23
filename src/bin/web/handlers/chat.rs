//! Chat 处理器

use axum::{
    extract::State,
    response::sse::{Event, Sse},
    Json,
};
use serde::{Deserialize, Serialize};
use futures_util::stream::Stream;
use std::convert::Infallible;

use crate::config::AppConfig;

/// Chat 处理器集合
#[derive(Debug, Clone)]
pub struct ChatHandlers {
    config: std::sync::Arc<AppConfig>,
}

impl ChatHandlers {
    pub fn new(config: std::sync::Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// 处理聊天请求
    pub async fn chat(&self, request: ChatRequest) -> ChatResponse {
        // TODO: 实现聊天处理逻辑
        ChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: request.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            message: format!("Echo: {}", request.message),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 处理流式聊天请求
    pub async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> impl Stream<Item = Result<Event, Infallible>> {
        use tokio::time::Duration;
        use futures_util::stream;

        let session_id = request.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        stream::unfold(0, move |state| async move {
            if state < 3 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let event = Event::default().data(format!(
                    r#"{{"session_id":"{}","content":"Chunk {}","done":false}}"#,
                    session_id, state
                ));
                Some((Ok(event), state + 1))
            } else {
                let event = Event::default().data(format!(
                    r#"{{"session_id":"{}","content":"","done":true}}"#,
                    session_id
                ));
                Some((Ok(event), state + 1))
            }
        })
    }
}

/// 聊天请求
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub stream: Option<bool>,
    pub config: Option<ChatConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChatConfig {
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub model: Option<String>,
}

/// 聊天响应
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub session_id: String,
    pub message: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat_handler() {
        let config = std::sync::Arc::new(AppConfig::default());
        let handlers = ChatHandlers::new(config);

        let request = ChatRequest {
            message: "Hello".to_string(),
            session_id: None,
            stream: None,
            config: None,
        };

        let response = handlers.chat(request).await;
        assert!(response.id.len() > 0);
        assert!(response.message.contains("Echo"));
    }
}
