//! Mock 记忆存储实现

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::domain::memory::{Message, store::MemoryStore};

/// Mock 记忆存储，用于测试
pub struct MockMemoryStore {
    conversations: Arc<RwLock<HashMap<String, Vec<Message>>>>,
    should_fail: Arc<RwLock<bool>>,
}

impl Default for MockMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMemoryStore {
    /// 创建新的 Mock 记忆存储
    pub fn new() -> Self {
        Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
            should_fail: Arc::new(RwLock::new(false)),
        }
    }

    /// 设置为失败模式
    pub fn with_failure(self) -> Self {
        *self.should_fail.write().unwrap() = true;
        self
    }


}

#[async_trait]
impl MemoryStore for MockMemoryStore {
    async fn append(&self, conversation_id: &str, message: &Message) -> Result<(), String> {
        if *self.should_fail.read().unwrap() {
            return Err("Mock storage failure".to_string());
        }

        let mut conversations = self.conversations.write().unwrap();
        conversations
            .entry(conversation_id.to_string())
            .or_insert_with(Vec::new)
            .push(message.clone());

        Ok(())
    }

    async fn load(&self, conversation_id: &str, _limit: usize) -> Result<Vec<Message>, String> {
        if *self.should_fail.read().unwrap() {
            return Err("Mock storage failure".to_string());
        }

        let conversations = self.conversations.read().unwrap();
        Ok(conversations
            .get(conversation_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete(&self, conversation_id: &str) -> Result<(), String> {
        if *self.should_fail.read().unwrap() {
            return Err("Mock storage failure".to_string());
        }

        let mut conversations = self.conversations.write().unwrap();
        conversations.remove(conversation_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Message;

    #[tokio::test]
    async fn test_mock_memory_append_and_load() {
        let store = MockMemoryStore::new();
        let conversation_id = "test_conv";

        store
            .append(conversation_id, &Message::user("Hello"))
            .await
            .unwrap();
        store
            .append(conversation_id, &Message::assistant("Hi"))
            .await
            .unwrap();

        let messages = store.load(conversation_id, 10).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi");
    }

    #[tokio::test]
    async fn test_mock_memory_delete() {
        let store = MockMemoryStore::new();
        let conversation_id = "test_conv";

        store
            .append(conversation_id, &Message::user("Hello"))
            .await
            .unwrap();

        store.delete(conversation_id).await.unwrap();

        let messages = store.load(conversation_id, 10).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_mock_memory_failure() {
        let store = MockMemoryStore::new().with_failure();

        let result = store.append("test", &Message::user("Hello")).await;
        assert!(result.is_err());
    }
}
