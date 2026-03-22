//! 会话存储 trait

use async_trait::async_trait;
use crate::domain::session::{Session, SessionConfig, SessionId, SessionState};

/// 会话存储 trait
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// 创建会话
    async fn create(&self, config: SessionConfig) -> Result<SessionId, String>;

    /// 获取会话
    async fn get(&self, id: &SessionId) -> Result<Option<Session>, String>;

    /// 获取会话状态
    async fn get_state(&self, id: &SessionId) -> Result<Option<SessionState>, String>;

    /// 更新会话
    async fn update(&self, session: Session) -> Result<(), String>;

    /// 删除会话
    async fn delete(&self, id: &SessionId) -> Result<(), String>;

    /// 列出所有会话
    async fn list(&self) -> Result<Vec<SessionId>, String>;
}

/// 内存会话存储实现
pub struct InMemorySessionStore {
    sessions: tokio::sync::RwLock<std::collections::HashMap<SessionId, Session>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create(&self, config: SessionConfig) -> Result<SessionId, String> {
        let id = config.id.clone();
        let session = Session::new(config);
        let mut sessions = self.sessions.write().await;
        sessions.insert(id.clone(), session);
        Ok(id)
    }

    async fn get(&self, id: &SessionId) -> Result<Option<Session>, String> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id).cloned())
    }

    async fn get_state(&self, id: &SessionId) -> Result<Option<SessionState>, String> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id).map(|s| s.state.clone()))
    }

    async fn update(&self, session: Session) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.config.id.clone(), session);
        Ok(())
    }

    async fn delete(&self, id: &SessionId) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<SessionId>, String> {
        let sessions = self.sessions.read().await;
        Ok(sessions.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_store_crud() {
        let store = InMemorySessionStore::new();

        // Create
        let config = SessionConfig::new();
        let id = store.create(config).await.unwrap();

        // Get
        let session = store.get(&id).await.unwrap();
        assert!(session.is_some());

        // List
        let sessions = store.list().await.unwrap();
        assert_eq!(sessions.len(), 1);

        // Delete
        store.delete(&id).await.unwrap();
        let session = store.get(&id).await.unwrap();
        assert!(session.is_none());
    }
}
