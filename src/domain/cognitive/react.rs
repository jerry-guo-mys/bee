//! ReAct 主循环相关类型

use crate::memory::Message;

/// ReAct 循环结果
#[derive(Debug)]
pub struct ReactResult {
    pub response: String,
    pub messages: Vec<Message>,
}

/// ReAct 会话配置
pub struct ReactSession<'a> {
    pub planner: &'a crate::domain::cognitive::planner::Planner,
    pub executor: &'a crate::domain::ToolExecutor,
    pub recovery: &'a crate::core::RecoveryEngine,
    pub cancel_token: tokio_util::sync::CancellationToken,
    pub critic: Option<&'a crate::domain::cognitive::critic::Critic>,
}

impl<'a> ReactSession<'a> {
    /// 创建新的 ReactSession
    pub fn new(
        planner: &'a crate::domain::cognitive::planner::Planner,
        executor: &'a crate::domain::ToolExecutor,
        recovery: &'a crate::core::RecoveryEngine,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self {
            planner,
            executor,
            recovery,
            cancel_token,
            critic: None,
        }
    }

    /// 设置 Critic
    pub fn with_critic(mut self, critic: &'a crate::domain::cognitive::critic::Critic) -> Self {
        self.critic = Some(critic);
        self
    }
}
