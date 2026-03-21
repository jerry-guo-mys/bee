//! 工具执行器
//!
//! 持有 ToolRegistry 与全局超时，execute(tool_name, args) 在超时内调用 registry.execute，
//! 超时或失败时转为 AgentError（ToolTimeout / ToolExecutionFailed）；每次调用输出结构化审计日志（JSON）。

use std::time::{Duration, Instant};

use tokio::time::timeout as tokio_timeout;
use tokio_util::sync::CancellationToken;

use crate::core::AgentError;
use crate::observability::Metrics;
use crate::tools::{ToolMetadata, ToolRegistry};

/// 工具执行器：对每次调用施加超时，并将结果映射为 AgentError
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

    /// 执行指定工具；超时返回 ToolTimeout，工具返回 Err 则转为 ToolExecutionFailed；输出 JSON 审计日志
    pub async fn execute(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<String, AgentError> {
        self.execute_cancellable(tool_name, args, CancellationToken::new())
            .await
    }

    pub async fn execute_cancellable(
        &self,
        tool_name: &str,
        args: serde_json::Value,
        cancel_token: CancellationToken,
    ) -> Result<String, AgentError> {
        let start = Instant::now();
        let args_preview = args_preview(&args);
        let metrics = Metrics::global();
        let tool_timeout = self
            .registry
            .get(tool_name)
            .and_then(|tool| tool.timeout_secs())
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let result = tokio::select! {
            _ = cancel_token.cancelled() => return Err(AgentError::Cancelled),
            result = tokio_timeout(
                tool_timeout,
                self.registry.execute_cancellable(tool_name, args, cancel_token.clone())
            ) => result,
        };

        let (ok, outcome, success): (bool, &str, bool) = match &result {
            Ok(Ok(_)) => (true, "ok", true),
            Ok(Err(_)) => (false, "error", false),
            Err(_) => (false, "timeout", false),
        };
        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as u64;

        // 记录工具执行 metrics
        metrics.tools.record_execution(success, duration);

        let audit = serde_json::json!({
            "event": "tool_audit",
            "tool": tool_name,
            "ok": ok,
            "outcome": outcome,
            "duration_ms": duration_ms,
            "args_preview": args_preview,
        });
        tracing::info!(audit = %audit.to_string(), "tool");
        tracing::debug!(
            target: "bee::metrics",
            tool = tool_name,
            success = success,
            duration_ms = duration_ms,
            "tool_execution"
        );

        if cancel_token.is_cancelled() {
            return Err(AgentError::Cancelled);
        }

        match result {
            Ok(Ok(content)) => Ok(content),
            Ok(Err(e)) => Err(AgentError::ToolExecutionFailed(e)),
            Err(_) => Err(AgentError::ToolTimeout(tool_name.to_string())),
        }
    }

    pub fn get_tool(&self, name: &str) -> Option<std::sync::Arc<dyn crate::tools::Tool>> {
        self.registry.get(name)
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.registry.tool_names()
    }

    pub fn tool_metadata(&self, name: &str) -> Option<ToolMetadata> {
        self.registry.tool_metadata(name)
    }

    pub fn tool_metadata_for_names(&self, names: &[String]) -> Vec<(String, ToolMetadata)> {
        self.registry.tool_metadata_for_names(names)
    }

    /// 返回 (name, description) 列表，用于按智能体技能过滤后生成 prompt
    pub fn tool_descriptions(&self) -> Vec<(String, String)> {
        self.registry.tool_descriptions()
    }
}

fn args_preview(args: &serde_json::Value) -> String {
    let s = args.to_string();
    if s.len() > 200 {
        format!("{}...", s.chars().take(200).collect::<String>())
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_trait::async_trait;
    use serde_json::json;

    use super::*;
    use crate::tools::{Tool, ToolRegistry};

    struct SlowTool {
        timeout_secs: Option<u64>,
        sleep_ms: u64,
    }

    #[async_trait]
    impl Tool for SlowTool {
        fn name(&self) -> &str {
            "slow"
        }

        fn description(&self) -> &str {
            "slow tool"
        }

        fn timeout_secs(&self) -> Option<u64> {
            self.timeout_secs
        }

        async fn execute(&self, _args: serde_json::Value) -> Result<String, String> {
            tokio::time::sleep(Duration::from_millis(self.sleep_ms)).await;
            Ok("ok".to_string())
        }
    }

    #[tokio::test]
    async fn test_executor_uses_tool_specific_timeout_override() {
        let mut registry = ToolRegistry::new();
        registry.register(SlowTool {
            timeout_secs: Some(1),
            sleep_ms: 150,
        });

        let executor = ToolExecutor::new(registry, 0);
        let result = executor.execute("slow", json!({})).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_executor_uses_default_timeout_when_tool_has_no_override() {
        let mut registry = ToolRegistry::new();
        registry.register(SlowTool {
            timeout_secs: None,
            sleep_ms: 150,
        });

        let executor = ToolExecutor::new(registry, 0);
        let result = executor.execute("slow", json!({})).await;

        assert!(matches!(result, Err(AgentError::ToolTimeout(name)) if name == "slow"));
    }

    #[tokio::test]
    async fn test_executor_respects_cancellation() {
        let mut registry = ToolRegistry::new();
        registry.register(SlowTool {
            timeout_secs: Some(5),
            sleep_ms: 500,
        });

        let executor = ToolExecutor::new(registry, 5);
        let cancel_token = CancellationToken::new();
        cancel_token.cancel();
        let result = executor
            .execute_cancellable("slow", json!({}), cancel_token)
            .await;

        assert!(matches!(result, Err(AgentError::Cancelled)));
    }
}
