//! 工具执行器

use std::time::{Duration, Instant};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::core::AgentError;
use crate::domain::tool::trait_::Tool;
use crate::domain::tool::ToolRegistry;

/// 工具执行器
pub struct ToolExecutor {
    registry: ToolRegistry,
    default_timeout: Duration,
}

impl ToolExecutor {
    pub fn new(registry: ToolRegistry, timeout_secs: u64) -> Self {
        Self {
            registry,
            default_timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// 执行工具
    pub async fn execute(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<String, AgentError> {
        self.execute_cancellable(tool_name, args, CancellationToken::new())
            .await
    }

    /// 执行可取消的工具
    pub async fn execute_cancellable(
        &self,
        tool_name: &str,
        args: serde_json::Value,
        cancel_token: CancellationToken,
    ) -> Result<String, AgentError> {
        let start = Instant::now();

        // 获取工具超时
        let tool_timeout = self
            .registry
            .get(tool_name)
            .and_then(|tool| tool.timeout_secs())
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let result = tokio::select! {
            _ = cancel_token.cancelled() => return Err(AgentError::Cancelled),
            result = timeout(tool_timeout, self.registry.execute(tool_name, args)) => {
                result.map_err(|_| AgentError::ToolTimeout(tool_name.to_string()))?
            }
        };

        let duration = start.elapsed();

        tracing::debug!(
            tool = tool_name,
            duration_ms = duration.as_millis() as u64,
            success = result.is_ok(),
            "Tool execution completed"
        );

        match result {
            Ok(content) => Ok(content),
            Err(e) => Err(AgentError::ToolExecutionFailed(e)),
        }
    }

    /// 获取工具
    pub fn get_tool(&self, name: &str) -> Option<std::sync::Arc<dyn Tool>> {
        self.registry.get(name)
    }

    /// 获取工具名称列表
    pub fn tool_names(&self) -> Vec<String> {
        self.registry.tool_names()
    }
}
