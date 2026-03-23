//! 文件存储实现
//!
//! 基于文件系统的记忆存储

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::domain::memory::store::MemoryStore;
use crate::memory::Message;

/// 文件存储实现
pub struct FileStore {
    base_path: PathBuf,
}

impl FileStore {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    fn conversation_path(&self, conversation_id: &str) -> PathBuf {
        // 将 conversation_id 转换为安全的文件名
        let safe_id = conversation_id.replace('/', "_").replace('\\', "_");
        self.base_path.join(format!("{}.jsonl", safe_id))
    }
}

#[async_trait]
impl MemoryStore for FileStore {
    async fn append(&self, conversation_id: &str, message: &Message) -> Result<(), String> {
        let path = self.conversation_path(conversation_id);

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // 序列化消息为 JSON
        let json = serde_json::to_string(message)
            .map_err(|e| format!("Failed to serialize message: {}", e))?;

        // 追加到文件
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| format!("Failed to write message: {}", e))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| format!("Failed to write newline: {}", e))?;

        Ok(())
    }

    async fn load(&self, conversation_id: &str, limit: usize) -> Result<Vec<Message>, String> {
        let path = self.conversation_path(conversation_id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&path)
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut messages = Vec::new();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let message: Message = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to deserialize message: {}", e))?;
            messages.push(message);
        }

        // 应用限制
        if limit > 0 && limit < messages.len() {
            messages = messages.into_iter().rev().take(limit).rev().collect();
        }

        Ok(messages)
    }

    async fn delete(&self, conversation_id: &str) -> Result<(), String> {
        let path = self.conversation_path(conversation_id);

        if path.exists() {
            fs::remove_file(&path)
                .await
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_store() -> (FileStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path());
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_append_and_load() {
        let (store, _temp) = create_test_store();

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
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi");
    }

    #[tokio::test]
    async fn test_delete() {
        let (store, _temp) = create_test_store();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store.delete("conv1").await.unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert!(messages.is_empty());
    }
}
