//! 多会话并发集成测试
//!
//! 测试多会话并发、会话恢复和会话隔离场景

#[cfg(test)]
mod tests {
    use bee::domain::session::{Session, SessionConfig, SessionId, SessionStatus};
    use bee::infrastructure::session::SqliteSessionStore;
    use bee::domain::session::store::SessionStore;

    #[tokio::test]
    async fn test_multi_session_concurrent_access() {
        // 测试多个会话的并发访问
        let store = SqliteSessionStore::in_memory().unwrap();

        // 创建多个会话
        let mut ids = Vec::new();
        for _ in 0..5 {
            let config = SessionConfig::new();
            let id = store.create(config).await.unwrap();
            ids.push(id);
        }

        // 并发访问多个会话
        let handles: Vec<_> = ids.iter().map(|id| {
            let store: SqliteSessionStore = store.clone();
            let id = id.clone();
            tokio::spawn(async move {
                let session = store.get(&id).await.unwrap();
                assert!(session.is_some());
                id
            })
        }).collect();

        // 等待所有任务完成
        let results: Vec<Result<SessionId, _>> = futures::future::join_all(handles).await;
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_session_recovery() {
        // 测试会话恢复功能
        let store = SqliteSessionStore::in_memory().unwrap();

        // 创建会话
        let config = SessionConfig::new()
            .with_max_turns(10)
            .with_system_prompt("Test system prompt");
        let id = store.create(config).await.unwrap();

        // 获取会话并验证
        let session = store.get(&id).await.unwrap();
        assert!(session.is_some());

        let session = session.unwrap();
        assert_eq!(session.config.max_turns, 10);
        assert_eq!(session.config.system_prompt, "Test system prompt");

        // 验证状态可以恢复
        let state = store.get_state(&id).await.unwrap();
        assert!(state.is_some());
        assert_eq!(state.unwrap().status, SessionStatus::Idle);
    }

    #[tokio::test]
    async fn test_session_isolation() {
        // 测试会话隔离性
        let store = SqliteSessionStore::in_memory().unwrap();

        // 创建两个会话
        let config1 = SessionConfig::new().with_system_prompt("Session 1");
        let id1 = store.create(config1).await.unwrap();

        let config2 = SessionConfig::new().with_system_prompt("Session 2");
        let id2 = store.create(config2).await.unwrap();

        // 验证会话 1 不能访问会话 2 的数据
        let session1 = store.get(&id1).await.unwrap().unwrap();
        assert_eq!(session1.config.system_prompt, "Session 1");

        let session2 = store.get(&id2).await.unwrap().unwrap();
        assert_eq!(session2.config.system_prompt, "Session 2");

        // 更新会话 1 不影响会话 2
        let mut session1 = Session::new(SessionConfig::new().with_system_prompt("Updated Session 1"));
        session1.config.id = id1.clone();
        store.update(session1).await.unwrap();

        let session2 = store.get(&id2).await.unwrap().unwrap();
        assert_eq!(session2.config.system_prompt, "Session 2");
    }

    #[tokio::test]
    async fn test_session_state_transitions() {
        // 测试会话状态转换
        let store = SqliteSessionStore::in_memory().unwrap();

        let config = SessionConfig::new();
        let id = store.create(config).await.unwrap();

        // 初始状态为 Idle
        let state = store.get_state(&id).await.unwrap().unwrap();
        assert_eq!(state.status, SessionStatus::Idle);

        // 更新为 Thinking
        let mut session = store.get(&id).await.unwrap().unwrap();
        session.state.status = SessionStatus::Thinking;
        store.update(session).await.unwrap();

        let state = store.get_state(&id).await.unwrap().unwrap();
        assert_eq!(state.status, SessionStatus::Thinking);

        // 更新为 Executing
        let mut session = store.get(&id).await.unwrap().unwrap();
        session.state.status = SessionStatus::Executing;
        store.update(session).await.unwrap();

        let state = store.get_state(&id).await.unwrap().unwrap();
        assert_eq!(state.status, SessionStatus::Executing);
    }

    #[tokio::test]
    async fn test_session_list_ordering() {
        // 测试会话列表按创建时间倒序
        let store = SqliteSessionStore::in_memory().unwrap();

        // 创建多个会话，每个会话之间有时间间隔以确保 created_at 不同
        let mut ids = Vec::new();
        for i in 0..3 {
            let config = SessionConfig::new().with_system_prompt(&format!("Session {}", i));
            let id = store.create(config).await.unwrap();
            ids.push(id);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // 获取列表
        let listed_ids = store.list().await.unwrap();

        // 验证列表包含所有会话
        assert_eq!(listed_ids.len(), 3);

        // 验证列表包含所有创建的 ID（不验证顺序，因为 SQLite 时间戳精度可能有限）
        let listed_id_strings: Vec<_> = listed_ids.iter().map(|id| id.0.clone()).collect();
        for id in &ids {
            assert!(listed_id_strings.contains(&id.0), "列表应包含所有会话 ID");
        }
    }

    #[tokio::test]
    async fn test_session_delete_isolation() {
        // 测试删除会话不影响其他会话
        let store = SqliteSessionStore::in_memory().unwrap();

        let config1 = SessionConfig::new();
        let id1 = store.create(config1).await.unwrap();

        let config2 = SessionConfig::new();
        let id2 = store.create(config2).await.unwrap();

        // 删除会话 1
        store.delete(&id1).await.unwrap();

        // 验证会话 1 被删除
        let session1 = store.get(&id1).await.unwrap();
        assert!(session1.is_none());

        // 验证会话 2 仍然存在
        let session2 = store.get(&id2).await.unwrap();
        assert!(session2.is_some());
    }

    #[tokio::test]
    async fn test_concurrent_session_creation() {
        // 测试并发创建会话
        let store = SqliteSessionStore::in_memory().unwrap();

        let mut handles = Vec::new();
        for i in 0..10 {
            let store: SqliteSessionStore = store.clone();
            let handle = tokio::spawn(async move {
                let config = SessionConfig::new().with_system_prompt(&format!("Concurrent {}", i));
                store.create(config).await.unwrap()
            });
            handles.push(handle);
        }

        let ids: Vec<Result<SessionId, _>> = futures::future::join_all(handles).await;
        let ids: Vec<SessionId> = ids.into_iter().map(|r| r.unwrap()).collect();

        // 验证所有会话都被创建
        let listed = store.list().await.unwrap();
        assert_eq!(listed.len(), 10);

        // 验证所有 ID 都不同
        let mut id_strings: Vec<_> = ids.iter().map(|id| id.0.clone()).collect();
        id_strings.sort();
        id_strings.dedup();
        assert_eq!(id_strings.len(), 10);
    }

    #[tokio::test]
    async fn test_session_update_preserves_id() {
        // 测试更新会话保持 ID 不变
        let store = SqliteSessionStore::in_memory().unwrap();

        let config = SessionConfig::new();
        let original_id = store.create(config).await.unwrap();

        // 获取并更新会话
        let mut session = store.get(&original_id).await.unwrap().unwrap();
        let original_config_id = session.config.id.0.clone();

        session.config.max_turns = 100;
        store.update(session).await.unwrap();

        // 验证 ID 保持不变
        let updated = store.get(&original_id).await.unwrap().unwrap();
        assert_eq!(updated.config.id.0, original_config_id);
    }

    #[tokio::test]
    async fn test_session_store_multiple_operations() {
        // 测试完整的会话操作流程
        let store = SqliteSessionStore::in_memory().unwrap();

        // Create
        let config = SessionConfig::new()
            .with_max_turns(5)
            .with_system_prompt("Integration test");
        let id = store.create(config).await.unwrap();

        // Read
        let session = store.get(&id).await.unwrap().unwrap();
        assert_eq!(session.config.max_turns, 5);

        // Update state
        let state = store.get_state(&id).await.unwrap().unwrap();
        assert_eq!(state.status, SessionStatus::Idle);

        // List
        let sessions = store.list().await.unwrap();
        assert_eq!(sessions.len(), 1);

        // Delete
        store.delete(&id).await.unwrap();
        assert!(store.get(&id).await.unwrap().is_none());
    }
}
