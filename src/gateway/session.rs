//! 会话管理
//!
//! 统一管理所有平台的会话状态，支持跨平台上下文连贯

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::message::{ClientInfo, SessionStatus, SpokeType};
use crate::react::ContextManager;

/// 会话 ID（用户维度，跨平台共享）
pub type SessionId = String;

/// 会话租户范围
#[derive(Debug, Clone, Default)]
pub struct SessionScope {
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub team_id: Option<String>,
    pub agent_instance_id: Option<String>,
    pub user_id: Option<String>,
}

impl SessionScope {
    pub fn from_client_metadata(user_id: &str, metadata: Option<&Value>) -> Self {
        let mut scope = Self {
            user_id: Some(user_id.to_string()),
            ..Self::default()
        };
        let Some(Value::Object(map)) = metadata else {
            return scope;
        };
        scope.tenant_id = scope_value(map.get("tenant_id"));
        scope.organization_id = scope_value(map.get("organization_id"));
        scope.team_id = scope_value(map.get("team_id"));
        scope.agent_instance_id = scope_value(map.get("agent_instance_id"));
        scope.user_id = scope_value(map.get("user_id")).or(scope.user_id);
        scope
    }
}

fn scope_value(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// 单个会话
pub struct Session {
    /// 会话 ID
    pub id: SessionId,
    /// 关联的用户 ID（跨平台统一）
    pub user_id: String,
    /// 当前连接的客户端信息
    pub clients: HashMap<SpokeType, ClientInfo>,
    /// 对话上下文（跨平台共享）
    pub context: ContextManager,
    /// 会话状态
    pub status: SessionStatus,
    /// 当前请求的取消令牌
    pub cancel_token: Option<CancellationToken>,
    /// 最后活跃时间
    pub last_active: Instant,
    /// 创建时间
    pub created_at: Instant,
    /// 助手 ID（可选）
    pub assistant_id: Option<String>,
    /// 模型 ID（可选）
    pub model_id: Option<String>,
    /// 多租户上下文范围
    pub scope: SessionScope,
}

impl Session {
    pub fn new(user_id: String, max_context_turns: usize) -> Self {
        let id = format!("session_{}", uuid::Uuid::new_v4());
        let scope_user_id = user_id.clone();
        Self {
            id,
            user_id,
            clients: HashMap::new(),
            context: ContextManager::new(max_context_turns),
            status: SessionStatus::Idle,
            cancel_token: None,
            last_active: Instant::now(),
            created_at: Instant::now(),
            assistant_id: None,
            model_id: None,
            scope: SessionScope {
                user_id: Some(scope_user_id),
                ..SessionScope::default()
            },
        }
    }

    pub fn apply_client_scope(&mut self, client: &ClientInfo) {
        let scope = SessionScope::from_client_metadata(&self.user_id, client.metadata.as_ref());
        self.scope = scope;
    }

    /// 添加客户端连接
    pub fn add_client(&mut self, client: ClientInfo) {
        self.clients.insert(client.platform, client);
        self.last_active = Instant::now();
    }

    /// 移除客户端连接
    pub fn remove_client(&mut self, platform: SpokeType) {
        self.clients.remove(&platform);
    }

    /// 检查会话是否还有活跃连接
    pub fn has_active_clients(&self) -> bool {
        !self.clients.is_empty()
    }

    /// 更新状态
    pub fn set_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.last_active = Instant::now();
    }

    /// 取消当前请求
    pub fn cancel(&mut self) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
        self.status = SessionStatus::Idle;
    }

    /// 创建新的取消令牌
    pub fn new_cancel_token(&mut self) -> CancellationToken {
        self.cancel();
        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());
        token
    }

    /// 会话是否过期
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_active.elapsed() > timeout && !self.has_active_clients()
    }
}

/// 会话管理器
pub struct SessionManager {
    /// 所有会话（session_id -> Arc<RwLock<Session>>）
    sessions: RwLock<HashMap<SessionId, Arc<RwLock<Session>>>>,
    /// 用户到会话的映射（user_id -> session_id）
    user_sessions: RwLock<HashMap<String, SessionId>>,
    /// 最大上下文轮数
    max_context_turns: usize,
    /// 会话过期时间
    session_timeout: Duration,
}

impl SessionManager {
    pub fn new(max_context_turns: usize, session_timeout_secs: u64) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            user_sessions: RwLock::new(HashMap::new()),
            max_context_turns,
            session_timeout: Duration::from_secs(session_timeout_secs),
        }
    }

    /// 获取或创建用户的会话
    pub async fn get_or_create(&self, user_id: &str, client: ClientInfo) -> SessionId {
        // 先检查是否已存在会话
        {
            let user_sessions = self.user_sessions.read().await;
            if let Some(session_id) = user_sessions.get(user_id) {
                let sessions = self.sessions.read().await;
                if let Some(session_arc) = sessions.get(session_id) {
                    let mut session = session_arc.write().await;
                    session.add_client(client);
                    return session_id.clone();
                }
            }
        }

        // 创建新会话
        let mut session = Session::new(user_id.to_string(), self.max_context_turns);
        session.apply_client_scope(&client);
        session.add_client(client);
        let session_id = session.id.clone();
        let session_arc = Arc::new(RwLock::new(session));

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), Arc::clone(&session_arc));
        self.user_sessions
            .write()
            .await
            .insert(user_id.to_string(), session_id.clone());

        session_id
    }

    /// 获取会话
    pub async fn get(&self, session_id: &str) -> Option<Arc<RwLock<Session>>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// 获取会话（可变引用）
    pub async fn with_session<F, R>(&self, session_id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut Session) -> R,
    {
        let sessions = self.sessions.read().await;
        if let Some(session_arc) = sessions.get(session_id) {
            let mut session = session_arc.write().await;
            Some(f(&mut *session))
        } else {
            None
        }
    }

    /// 移除客户端连接
    pub async fn remove_client(&self, session_id: &str, platform: SpokeType) {
        let sessions = self.sessions.read().await;
        if let Some(session_arc) = sessions.get(session_id) {
            let mut session = session_arc.write().await;
            session.remove_client(platform);
        }
    }

    /// 清理过期会话
    pub async fn cleanup_expired(&self) -> usize {
        let sessions = self.sessions.read().await;

        let expired: Vec<_> = sessions
            .iter()
            .filter(|(_, s)| s.try_read().map(|s| s.is_expired(self.session_timeout)).unwrap_or(false))
            .map(|(id, _)| id.clone())
            .collect();

        drop(sessions);

        let mut sessions = self.sessions.write().await;
        let mut user_sessions = self.user_sessions.write().await;

        for session_id in &expired {
            sessions.remove(session_id);
        }

        // 清理 user_sessions 中指向已删除会话的映射
        user_sessions.retain(|_, sid| sessions.contains_key(sid));

        expired.len()
    }

    /// 获取活跃会话数
    pub async fn active_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// 获取用户的会话 ID
    pub async fn get_user_session(&self, user_id: &str) -> Option<SessionId> {
        self.user_sessions.read().await.get(user_id).cloned()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(20, 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::message::ClientInfo;

    #[tokio::test]
    async fn test_get_does_not_remove_session() {
        // 验证 get 方法不会从管理器中移除会话（修复问题 1）
        let manager = SessionManager::new(10, 3600);
        let client = ClientInfo {
            client_id: "test_client".to_string(),
            platform: SpokeType::Web,
            display_name: Some("Test".to_string()),
            metadata: None,
        };

        // 创建会话
        let session_id = manager.get_or_create("user1", client.clone()).await;

        // 第一次获取会话
        let session1 = manager.get(&session_id).await;
        assert!(session1.is_some(), "Should get session on first call");

        // 第二次获取会话 - 应该仍然能获取到（不会被 remove）
        let session2 = manager.get(&session_id).await;
        assert!(session2.is_some(), "Should still get session on second call");

        // 验证两次获取的是同一个 Arc
        assert!(Arc::ptr_eq(&session1.unwrap(), &session2.unwrap()));
    }

    #[tokio::test]
    async fn test_get_or_create_concurrent_safety() {
        // 验证并发调用 get_or_create 的安全性（修复问题 5）
        let manager = Arc::new(SessionManager::new(10, 3600));
        let client = ClientInfo {
            client_id: "test_client".to_string(),
            platform: SpokeType::Web,
            display_name: Some("Test".to_string()),
            metadata: None,
        };

        // 并发创建多个会话
        let mut handles = vec![];
        for i in 0..5 {
            let mgr = Arc::clone(&manager);
            let cli = client.clone();
            handles.push(tokio::spawn(async move {
                mgr.get_or_create(&format!("user{}", i), cli).await
            }));
        }

        // 等待所有任务完成
        let session_ids: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // 验证每个会话 ID 都不同
        let unique_count = session_ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 5, "All session IDs should be unique");
    }

    #[tokio::test]
    async fn test_with_session_modifies_original() {
        // 验证 with_session 能正确修改原始会话
        let manager = SessionManager::new(10, 3600);
        let client = ClientInfo {
            client_id: "test_client".to_string(),
            platform: SpokeType::Web,
            display_name: Some("Test".to_string()),
            metadata: None,
        };

        let session_id = manager.get_or_create("user1", client.clone()).await;

        // 使用 with_session 修改会话状态
        manager
            .with_session(&session_id, |session| {
                session.set_status(super::SessionStatus::Processing);
            })
            .await;

        // 验证修改已生效
        let session = manager.get(&session_id).await.unwrap();
        let session = session.read().await;
        assert_eq!(session.status, super::SessionStatus::Processing);
    }

    #[tokio::test]
    async fn test_cleanup_expired_removes_from_both_maps() {
        // 验证清理过期会话时同时清理两个映射
        let manager = SessionManager::new(10, 0); // 0 秒过期时间，便于测试
        let client = ClientInfo {
            client_id: "test_client".to_string(),
            platform: SpokeType::Web,
            display_name: Some("Test".to_string()),
            metadata: None,
        };

        let session_id = manager.get_or_create("user1", client.clone()).await;

        // 先移除客户端连接，使会话可以被清理
        manager.remove_client(&session_id, SpokeType::Web).await;

        // 等待一小段时间让会话过期
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // 清理过期会话
        let removed = manager.cleanup_expired().await;
        assert_eq!(removed, 1, "Should remove 1 expired session");

        // 验证会话已从两个映射中删除
        assert!(manager.get(&session_id).await.is_none());
        assert!(manager.get_user_session("user1").await.is_none());
    }
}
