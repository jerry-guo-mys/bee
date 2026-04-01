//! 记忆持久化集成测试
//!
//! 测试记忆存储的持久化功能

#[cfg(test)]
mod tests {
    use bee::domain::memory::store::MemoryStore;
    use bee::infrastructure::memory::{InMemoryStore, SqliteMemoryStore};
    use bee::memory::Message;

    #[tokio::test]
    async fn test_memory_store_append_and_load() {
        let store = InMemoryStore::new();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store
            .append("conv1", &Message::assistant("Hi there!"))
            .await
            .unwrap();

        let messages = store.load("conv1", 0).await.unwrap();

        assert_eq!(messages.len(), 2);
        assert!(matches!(messages[0].role, bee::memory::Role::User));
        assert!(matches!(messages[1].role, bee::memory::Role::Assistant));
    }

    #[tokio::test]
    async fn test_memory_store_load_with_limit() {
        let store = InMemoryStore::new();

        for i in 0..10 {
            store
                .append("conv1", &Message::user(&format!("Message {}", i)))
                .await
                .unwrap();
        }

        // Load with limit
        let messages = store.load("conv1", 5).await.unwrap();
        assert_eq!(messages.len(), 5);

        // Should return the most recent messages
        assert!(messages[0].content.contains("Message 5"));
        assert!(messages[4].content.contains("Message 9"));
    }

    #[tokio::test]
    async fn test_memory_store_delete() {
        let store = InMemoryStore::new();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store.delete("conv1").await.unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_memory_store_multiple_conversations() {
        let store = InMemoryStore::new();

        // Conversation 1
        store
            .append("conv1", &Message::user("Hello from conv1"))
            .await
            .unwrap();
        store
            .append("conv1", &Message::assistant("Hi from conv1"))
            .await
            .unwrap();

        // Conversation 2
        store
            .append("conv2", &Message::user("Hello from conv2"))
            .await
            .unwrap();
        store
            .append("conv2", &Message::assistant("Hi from conv2"))
            .await
            .unwrap();

        // Load conv1
        let conv1_messages = store.load("conv1", 0).await.unwrap();
        assert_eq!(conv1_messages.len(), 2);
        assert!(conv1_messages[0].content.contains("conv1"));

        // Load conv2
        let conv2_messages = store.load("conv2", 0).await.unwrap();
        assert_eq!(conv2_messages.len(), 2);
        assert!(conv2_messages[0].content.contains("conv2"));
    }

    #[tokio::test]
    async fn test_sqlite_memory_store_persistence() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store
            .append("conv1", &Message::assistant("Hi!"))
            .await
            .unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_memory_store_load_with_limit() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        for i in 0..10 {
            store
                .append("conv1", &Message::user(&format!("Msg {}", i)))
                .await
                .unwrap();
        }

        let messages = store.load("conv1", 3).await.unwrap();
        assert_eq!(messages.len(), 3);
        // Should return the most recent messages
        assert!(messages[0].content.contains("Msg 7"));
        assert!(messages[2].content.contains("Msg 9"));
    }

    #[tokio::test]
    async fn test_sqlite_memory_store_delete() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store.delete("conv1").await.unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_memory_store_system_message() {
        let store = InMemoryStore::new();

        store
            .append("conv1", &Message::system("You are a helpful assistant."))
            .await
            .unwrap();
        store
            .append("conv1", &Message::user("Hello"))
            .await
            .unwrap();
        store
            .append("conv1", &Message::assistant("Hi!"))
            .await
            .unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert_eq!(messages.len(), 3);
        assert!(matches!(messages[0].role, bee::memory::Role::System));
    }

    #[tokio::test]
    async fn test_memory_store_tool_message() {
        let store = InMemoryStore::new();

        store
            .append("conv1", &Message::user("Calculate 2+2"))
            .await
            .unwrap();
        store
            .append("conv1", &Message::assistant("Let me use the calculator."))
            .await
            .unwrap();
        store.append("conv1", &Message::tool("4")).await.unwrap();
        store
            .append("conv1", &Message::assistant("The answer is 4."))
            .await
            .unwrap();

        let messages = store.load("conv1", 0).await.unwrap();
        assert_eq!(messages.len(), 4);

        let tool_messages: Vec<_> = messages
            .iter()
            .filter(|m| matches!(m.role, bee::memory::Role::Tool))
            .collect();
        assert_eq!(tool_messages.len(), 1);
    }
}
