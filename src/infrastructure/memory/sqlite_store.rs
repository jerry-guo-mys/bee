//! SQLite 存储实现
//!
//! 基于 SQLite 的记忆存储，支持持久化和并发访问

use async_trait::async_trait;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::spawn_blocking;

use crate::domain::memory::store::MemoryStore;
use crate::memory::{Message, Role};

/// SQLite 存储实现
pub struct SqliteMemoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMemoryStore {
    /// 创建新的 SQLite 存储
    pub fn new(path: impl AsRef<Path>) -> SqliteResult<Self> {
        let conn = Connection::open(path.as_ref())?;

        // 初始化表结构
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;

        // 创建索引以提高查询性能
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_conversation_id ON messages(conversation_id)",
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
            "CREATE TABLE messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX idx_conversation_id ON messages(conversation_id)",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn message_to_row(message: &Message) -> &str {
        match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        }
    }

    fn row_to_message(role: &str, content: &str) -> Message {
        match role {
            "user" => Message::user(content),
            "assistant" => Message::assistant(content),
            "system" => Message::system(content),
            "tool" => Message::tool(content),
            _ => Message::assistant(content),
        }
    }
}

#[async_trait]
impl MemoryStore for SqliteMemoryStore {
    async fn append(&self, conversation_id: &str, message: &Message) -> Result<(), String> {
        let conn = self.conn.clone();
        let conversation_id = conversation_id.to_string();
        let role = SqliteMemoryStore::message_to_row(message).to_string();
        let content = message.content.clone();

        spawn_blocking(move || {
            let conn = conn.blocking_lock();
            conn.execute(
                "INSERT INTO messages (conversation_id, role, content) VALUES (?1, ?2, ?3)",
                params![conversation_id, role, content],
            )
            .map_err(|e| format!("Failed to insert message: {}", e))?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))??;
        Ok(())
    }

    async fn load(&self, conversation_id: &str, limit: usize) -> Result<Vec<Message>, String> {
        let conn = self.conn.clone();
        let conversation_id = conversation_id.to_string();

        let messages = spawn_blocking(move || {
            let conn = conn.blocking_lock();
            let mut stmt = conn
                .prepare(
                    "SELECT role, content FROM messages WHERE conversation_id = ?1 ORDER BY id",
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            let message_iter = stmt
                .query_map(params![conversation_id], |row| {
                    let role: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    Ok(SqliteMemoryStore::row_to_message(&role, &content))
                })
                .map_err(|e| format!("Failed to query messages: {}", e))?;

            let messages: Vec<Message> = message_iter.filter_map(|r| r.ok()).collect();
            Ok::<Vec<Message>, String>(messages)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))??;

        // 应用限制
        let mut messages = messages;
        if limit > 0 && limit < messages.len() {
            messages = messages.into_iter().rev().take(limit).rev().collect();
        }

        Ok(messages)
    }

    async fn delete(&self, conversation_id: &str) -> Result<(), String> {
        let conn = self.conn.clone();
        let conversation_id = conversation_id.to_string();

        spawn_blocking(move || {
            let conn = conn.blocking_lock();
            conn.execute(
                "DELETE FROM messages WHERE conversation_id = ?1",
                params![conversation_id],
            )
            .map_err(|e| format!("Failed to delete messages: {}", e))?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))??;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_and_load() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store
            .append("conv1", &Message::assistant("Hi"))
            .await
            .unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_load_with_limit() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        for i in 0..10 {
            store
                .append("conv1", &Message::user(&format!("msg{}", i)))
                .await
                .unwrap();
        }

        let messages = store.load("conv1", 3).await.unwrap();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store.delete("conv1").await.unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert!(messages.is_empty());
    }
}
