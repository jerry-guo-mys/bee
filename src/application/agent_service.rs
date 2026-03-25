//! Agent 服务 trait 与实现

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::AppConfig;
use crate::core::{AgentComponents, AgentError};
use crate::domain::session::{Session, SessionConfig, SessionStatus};
use crate::memory::{Message, SqlitePersistence};
use crate::react::{react_loop, ContextManager};

/// Agent 响应
#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub success: bool,
    pub message: String,
    pub messages: Vec<Message>,
}

/// 会话状态
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub id: String,
    pub status: SessionStatusEnum,
    pub message_count: usize,
}

/// 会话状态枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatusEnum {
    Idle,
    Thinking,
    Executing,
    Responding,
    Error(String),
}

impl Default for SessionStatusEnum {
    fn default() -> Self {
        Self::Idle
    }
}

/// Agent 服务 trait
#[async_trait::async_trait]
pub trait AgentService: Send + Sync {
    /// 处理用户消息
    async fn process_message(
        &self,
        session_id: &str,
        input: &str,
    ) -> Result<AgentResponse, AgentError>;

    /// 取消当前操作
    async fn cancel(&self, session_id: &str) -> Result<(), AgentError>;

    /// 清空会话
    async fn clear(&self, session_id: &str) -> Result<(), AgentError>;

    /// 获取会话状态
    async fn get_session(&self, session_id: &str) -> Result<SessionState, AgentError>;
}

/// Agent 服务实现
pub struct AgentServiceImpl {
    config: AppConfig,
    components: Arc<AgentComponents>,
    sqlite_persistence: Arc<Mutex<Option<SqlitePersistence>>>,
    sessions: Arc<Mutex<std::collections::HashMap<String, Session>>>,
}

impl AgentServiceImpl {
    pub fn new(
        config: AppConfig,
        components: AgentComponents,
        sqlite_persistence: Arc<Mutex<Option<SqlitePersistence>>>,
    ) -> Self {
        Self {
            config,
            components: Arc::new(components),
            sqlite_persistence,
            sessions: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// 获取或创建会话
    async fn get_or_create_session(&self, session_id: &str) -> Session {
        let mut sessions = self.sessions.lock().await;
        sessions
            .entry(session_id.to_string())
            .or_insert_with(|| {
                let config = SessionConfig::new()
                    .with_max_turns(20)
                    .with_system_prompt("You are a helpful AI assistant.");
                Session::new(config)
            })
            .clone()
    }

    /// 更新会话
    async fn update_session(&self, session_id: &str, session: Session) {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.to_string(), session);
    }
}

#[async_trait::async_trait]
impl AgentService for AgentServiceImpl {
    async fn process_message(
        &self,
        session_id: &str,
        input: &str,
    ) -> Result<AgentResponse, AgentError> {
        use tokio_util::sync::CancellationToken;

        let mut session = self.get_or_create_session(session_id).await;
        let mut context = ContextManager::new(self.config.app.max_context_turns);
        
        // 从 SQLite 加载历史消息
        {
            let persistence = self.sqlite_persistence.lock().await;
            if let Some(ref p) = *persistence {
                if let Ok(messages) = p.load_messages(session_id) {
                    for msg in messages {
                        context.conversation.push(msg);
                    }
                }
            }
        }

        // 保存用户消息
        {
            let persistence = self.sqlite_persistence.lock().await;
            if let Some(ref p) = *persistence {
                let _ = p.save_message(
                    session_id,
                    &Message {
                        role: crate::memory::Role::User,
                        content: input.to_string(),
                    },
                );
            }
        }

        // 运行 ReAct 循环
        let cancel_token = CancellationToken::new();
        let result = react_loop(
            &self.components.planner,
            &self.components.executor,
            &self.components.recovery,
            &mut context,
            input,
            None,
            None,
            cancel_token,
            self.components.critic.as_ref(),
            Some(&self.components.task_scheduler),
            None,
            None,
        )
        .await;

        match result {
            Ok(react_result) => {
                // 保存助手消息
                if let Some(last_msg) = react_result.messages.last() {
                    if last_msg.role == crate::memory::Role::Assistant {
                        let persistence = self.sqlite_persistence.lock().await;
                        if let Some(ref p) = *persistence {
                            let _ = p.save_message(session_id, last_msg);
                        }
                    }

                    // 更新会话状态
                    session.add_message(last_msg.clone());
                }
                self.update_session(session_id, session).await;

                Ok(AgentResponse {
                    success: true,
                    message: String::new(),
                    messages: react_result.messages,
                })
            }
            Err(e) => Err(AgentError::OrchestrationFailed(e.to_string())),
        }
    }

    async fn cancel(&self, session_id: &str) -> Result<(), AgentError> {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.set_status(SessionStatus::Idle);
        }
        Ok(())
    }

    async fn clear(&self, session_id: &str) -> Result<(), AgentError> {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.messages.clear();
            session.state.message_count = 0;
        }
        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> Result<SessionState, AgentError> {
        let sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(session_id) {
            Ok(SessionState {
                id: session_id.to_string(),
                status: match &session.state.status {
                    SessionStatus::Idle => SessionStatusEnum::Idle,
                    SessionStatus::Thinking => SessionStatusEnum::Thinking,
                    SessionStatus::Executing => SessionStatusEnum::Executing,
                    SessionStatus::Responding => SessionStatusEnum::Responding,
                    SessionStatus::Error(msg) => SessionStatusEnum::Error(msg.clone()),
                },
                message_count: session.state.message_count,
            })
        } else {
            Ok(SessionState::default())
        }
    }
}
