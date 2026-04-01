//! 会话监管：生命周期、中断管理
//!
//! 持有 CancellationToken，用户 Ctrl+C 时取消当前 ReAct 步；支持暂停与子 token（单任务取消）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use tokio_util::sync::CancellationToken;

/// 会话级生命周期管理：取消令牌与暂停状态
///
/// 解决问题 1.4：每次 Submit 重建 CancellationToken
#[derive(Debug)]
pub struct SessionSupervisor {
    /// 用户 Cancel 时触发，使用 RwLock 支持重建（解决问题 1.4）
    cancel_token: Arc<RwLock<CancellationToken>>,
    /// 是否已暂停（使用 AtomicBool 提升性能）
    paused: Arc<AtomicBool>,
}

impl SessionSupervisor {
    pub fn new() -> Self {
        Self {
            cancel_token: Arc::new(RwLock::new(CancellationToken::new())),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 获取当前 cancel token 的克隆
    ///
    /// # Panics
    /// 如果锁被 poison（极低概率），会 panic。由于 CancellationToken 本身无 panic 路径，
    /// 锁中毒通常意味着其他严重错误。
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// 触发取消（用户 Ctrl+C）
    pub fn cancel(&self) {
        let _ = self.cancel_token.read().map(|g| g.cancel());
    }

    /// 重建 cancel token（每次 Submit 前调用，解决问题 1.4）
    ///
    /// 当前 token 取消后，新请求需要新的 token
    /// **返回新的 CancellationToken，可直接用于后续操作**
    pub fn reset_cancel_token(&self) -> CancellationToken {
        let mut guard = self.cancel_token.write().unwrap();
        *guard = CancellationToken::new();
        guard.clone()
    }

    /// 检查是否已取消
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token
            .read()
            .map(|g| g.is_cancelled())
            .unwrap_or(true)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    /// 创建子 token（用于单个任务）
    ///
    /// **注意**：如果父 token 已取消，返回的子 token 也会立即取消。
    /// 调用方应检查 `token.is_cancelled()` 或使用 `token.run_until_cancelled()`。
    pub fn child_token(&self) -> CancellationToken {
        self.cancel_token
            .read()
            .map(|g| g.child_token())
            .unwrap_or_else(|e| {
                // 锁中毒时返回一个未取消的 token，避免 panic
                let guard = e.into_inner();
                guard.child_token()
            })
    }
}

impl Default for SessionSupervisor {
    fn default() -> Self {
        Self::new()
    }
}
