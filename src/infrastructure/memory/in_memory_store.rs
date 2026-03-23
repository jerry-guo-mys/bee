//! 内存存储实现
//!
//! 基于 tokio RwLock 的线程安全内存存储

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::domain::memory::store::MemoryStore;
use crate::memory::Message;

/// 内存存储实现
pub struct InMemoryStore {
    conversations: RwLock<HashMap<String, Vec<Message>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            conversations: RwLock::new(HashMap::new()),
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

    async fn load(&self, conversation_id: &str, limit: usize) -> Result<Vec<Message>, String> {
        let conversations = self.conversations.read().await;
        Ok(conversations
            .get(conversation_id)
            .map(|msgs| {
                if limit == 0 || limit >= msgs.len() {
                    msgs.clone()
                } else {
                    msgs.iter().rev().take(limit).rev().cloned().collect()
                }
            })
            .unwrap_or_default())
    }

    async fn delete(&self, conversation_id: &str) -> Result<(), String> {
        let mut conversations = self.conversations.write().await;
        conversations.remove(conversation_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_and_load() {
        let store = InMemoryStore::new();

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
        let store = InMemoryStore::new();

        for i in 0..10 {
            store
                .append("conv1", &Message::user(&format!("msg{}", i)))
                .await
                .unwrap();
        }

        let messages = store.load("conv1", 3).await.unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "msg7");
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemoryStore::new();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store.delete("conv1").await.unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert!(messages.is_empty());
    }
}
