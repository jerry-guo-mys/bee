//! SQLite 会话存储实现

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::session::store::SessionStore;
use crate::domain::session::{Session, SessionConfig, SessionId, SessionState, SessionStatus};

/// SQLite 会话存储实现
#[derive(Clone)]
pub struct SqliteSessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSessionStore {
    /// 创建新的 SQLite 会话存储
    pub fn new(path: impl AsRef<Path>) -> SqliteResult<Self> {
        let conn = Connection::open(path.as_ref())?;

        // 初始化表结构
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                max_turns INTEGER NOT NULL,
                system_prompt TEXT NOT NULL,
                status TEXT NOT NULL,
                message_count INTEGER DEFAULT 0,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 创建内存中 SQLite 存储（用于测试）
    pub fn in_memory() -> SqliteResult<Self> {
        let conn = Connection::open(":memory:")?;

        conn.execute(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                max_turns INTEGER NOT NULL,
                system_prompt TEXT NOT NULL,
                status TEXT NOT NULL,
                message_count INTEGER DEFAULT 0,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn row_to_session(id: &str, max_turns: i64, system_prompt: String, message_count: i64, status: String) -> Session {
        let mut config = SessionConfig::new();
        config.id = SessionId(id.to_string());
        config.max_turns = max_turns as usize;
        config.system_prompt = system_prompt;

        let mut session = Session::new(config);
        session.state.message_count = message_count as usize;
        session.state.status = match status.as_str() {
            "idle" => SessionStatus::Idle,
            "thinking" => SessionStatus::Thinking,
            "executing" => SessionStatus::Executing,
            "responding" => SessionStatus::Responding,
            "error" => SessionStatus::Error("Unknown error".to_string()),
            _ => SessionStatus::Idle,
        };
        session
    }
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    async fn create(&self, config: SessionConfig) -> Result<SessionId, String> {
        let conn = self.conn.lock().await;
        let id = config.id.0.clone();
        let max_turns = config.max_turns as i64;
        let system_prompt = &config.system_prompt;

        conn.execute(
            "INSERT INTO sessions (id, max_turns, system_prompt, status, message_count) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, max_turns, system_prompt, "idle", 0],
        )
        .map_err(|e| format!("Failed to create session: {}", e))?;
        Ok(SessionId(id))
    }

    async fn get(&self, id: &SessionId) -> Result<Option<Session>, String> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT id, max_turns, system_prompt, message_count, status FROM sessions WHERE id = ?1")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let result: Option<(String, i64, String, i64, String)> = stmt
            .query_row(params![&id.0], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .optional()
            .map_err(|e| format!("Failed to query session: {}", e))?;

        Ok(result.map(|(id, max_turns, system_prompt, message_count, status)| {
            SqliteSessionStore::row_to_session(&id, max_turns, system_prompt, message_count, status)
        }))
    }

    async fn get_state(&self, id: &SessionId) -> Result<Option<SessionState>, String> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT status, message_count FROM sessions WHERE id = ?1")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let result: Option<(String, i64)> = stmt
            .query_row(params![&id.0], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .map_err(|e| format!("Failed to query state: {}", e))?;

        Ok(result.map(|(status, message_count)| {
            SessionState {
                id: id.clone(),
                status: match status.as_str() {
                    "idle" => SessionStatus::Idle,
                    "thinking" => SessionStatus::Thinking,
                    "executing" => SessionStatus::Executing,
                    "responding" => SessionStatus::Responding,
                    "error" => SessionStatus::Error("Unknown error".to_string()),
                    _ => SessionStatus::Idle,
                },
                message_count: message_count as usize,
            }
        }))
    }

    async fn update(&self, session: Session) -> Result<(), String> {
        let conn = self.conn.lock().await;
        let id = &session.config.id.0;
        let max_turns = session.config.max_turns as i64;
        let system_prompt = &session.config.system_prompt;
        let message_count = session.state.message_count as i64;
        let status = match &session.state.status {
            SessionStatus::Idle => "idle",
            SessionStatus::Thinking => "thinking",
            SessionStatus::Executing => "executing",
            SessionStatus::Responding => "responding",
            SessionStatus::Error(_) => "error",
        };

        conn.execute(
            "UPDATE sessions SET max_turns = ?2, system_prompt = ?3, message_count = ?4, status = ?5, updated_at = (strftime('%s', 'now')) WHERE id = ?1",
            params![id, max_turns, system_prompt, message_count, status],
        )
        .map_err(|e| format!("Failed to update session: {}", e))?;
        Ok(())
    }

    async fn delete(&self, id: &SessionId) -> Result<(), String> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            params![&id.0],
        )
        .map_err(|e| format!("Failed to delete session: {}", e))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<SessionId>, String> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT id FROM sessions ORDER BY created_at DESC")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let ids = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| format!("Failed to query sessions: {}", e))?;

        let mut result = Vec::new();
        for id_result in ids {
            if let Ok(id) = id_result {
                result.push(SessionId(id));
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_store_crud() {
        let store = SqliteSessionStore::in_memory().unwrap();

        // Create
        let config = SessionConfig::new();
        let id = store.create(config).await.unwrap();

        // Get
        let session = store.get(&id).await.unwrap();
        assert!(session.is_some());

        // Get state
        let state = store.get_state(&id).await.unwrap();
        assert!(state.is_some());
        assert_eq!(state.unwrap().status, SessionStatus::Idle);

        // List
        let sessions = store.list().await.unwrap();
        assert_eq!(sessions.len(), 1);

        // Delete
        store.delete(&id).await.unwrap();
        let session = store.get(&id).await.unwrap();
        assert!(session.is_none());
    }
}
