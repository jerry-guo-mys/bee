//! 记忆存储抽象

use async_trait::async_trait;
use crate::memory::Message;

/// 记忆存储 trait
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// 追加消息
    async fn append(&self, conversation_id: &str, message: &Message) -> Result<(), String>;

    /// 加载消息
    async fn load(&self, conversation_id: &str, limit: usize) -> Result<Vec<Message>, String>;

    /// 删除对话
    async fn delete(&self, conversation_id: &str) -> Result<(), String>;
}

/// 工厂函数：创建记忆存储
pub fn create_memory_store(
    backend: MemoryBackend,
    path: Option<&str>,
) -> Result<Box<dyn MemoryStore>, String> {
    match backend {
        MemoryBackend::InMemory => Ok(Box::new(InMemoryStore::new())),
        MemoryBackend::File => {
            let path = path.ok_or("File backend requires a path")?;
            Ok(Box::new(FileStore::new(path)))
        }
        // SQLite 需要异步运行时，留给 infrastructure 层实现
        MemoryBackend::Sqlite => Err("Sqlite backend should be created via infrastructure layer".to_string()),
    }
}

/// 存储后端类型
#[derive(Debug, Clone, Copy)]
pub enum MemoryBackend {
    InMemory,
    File,
    Sqlite,
}

/// 内存存储实现
pub struct InMemoryStore {
    conversations: tokio::sync::RwLock<std::collections::HashMap<String, Vec<Message>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            conversations: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    async fn append(&self, conversation_id: &str, message: &Message) -> Result<(), String> {
        let mut conversations = self.conversations.write().await;
        conversations
            .entry(conversation_id.to_string())
            .or_insert_with(Vec::new)
            .push(message.clone());
        Ok(())
    }

    async fn load(&self, conversation_id: &str, _limit: usize) -> Result<Vec<Message>, String> {
        let conversations = self.conversations.read().await;
        Ok(conversations
            .get(conversation_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete(&self, conversation_id: &str) -> Result<(), String> {
        let mut conversations = self.conversations.write().await;
        conversations.remove(conversation_id);
        Ok(())
    }
}

/// 文件存储实现（简化版）
pub struct FileStore {
    #[allow(dead_code)]
    base_path: String,
}

impl FileStore {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
        }
    }
}

#[async_trait]
impl MemoryStore for FileStore {
    async fn append(&self, _conversation_id: &str, _message: &Message) -> Result<(), String> {
        // 简化实现，实际应写入文件
        Ok(())
    }

    async fn load(&self, _conversation_id: &str, _limit: usize) -> Result<Vec<Message>, String> {
        // 简化实现，实际应从文件读取
        Ok(vec![])
    }

    async fn delete(&self, _conversation_id: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_store() {
        let store = InMemoryStore::new();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store
            .append("conv1", &Message::assistant("Hi"))
            .await
            .unwrap();

        let messages = store.load("conv1", 10).await.unwrap();
        assert_eq!(messages.len(), 2);

        store.delete("conv1").await.unwrap();
        let messages = store.load("conv1", 10).await.unwrap();
        assert!(messages.is_empty());
    }
}
