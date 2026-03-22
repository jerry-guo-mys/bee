//! 会话模型

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 会话 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 会话配置
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub id: SessionId,
    pub max_turns: usize,
    pub system_prompt: String,
}

impl SessionConfig {
    pub fn new() -> Self {
        Self {
            id: SessionId::new(),
            max_turns: 20,
            system_prompt: "You are a helpful assistant.".to_string(),
        }
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// 会话状态
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub id: SessionId,
    pub status: SessionStatus,
    pub message_count: usize,
}

/// 会话状态枚举
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SessionStatus {
    #[default]
    Idle,
    Thinking,
    Executing,
    Responding,
    Error(String),
}

/// 会话
#[derive(Clone)]
pub struct Session {
    pub config: SessionConfig,
    pub state: SessionState,
    pub messages: Vec<crate::memory::Message>,
}

impl Session {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config: config.clone(),
            state: SessionState {
                id: config.id.clone(),
                status: SessionStatus::Idle,
                message_count: 0,
            },
            messages: vec![],
        }
    }

    /// 添加消息
    pub fn add_message(&mut self, message: crate::memory::Message) {
        self.messages.push(message);
        self.state.message_count += 1;
    }

    /// 设置状态
    pub fn set_status(&mut self, status: SessionStatus) {
        self.state.status = status;
    }
}
