use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::Value;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::tools::{
    Tool, ToolCriticMode, ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk, ToolScope,
    ToolUseCase,
};

pub struct TestRunTool {
    project_root: PathBuf,
    timeout_secs: u64,
}

impl TestRunTool {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            timeout_secs: 300,
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

#[async_trait]
impl Tool for TestRunTool {
    fn name(&self) -> &str {
        "test_run"
    }

    fn description(&self) -> &str {
        r#"运行 Rust 测试套件。

参数:
- package: 要测试的包（可选，默认当前包）
- test_name: 特定测试名（可选）
- features: 启用的特性（可选，如 "web,whatsapp"）

返回: 测试结果摘要

示例:
{"package": "bee", "test_name": "test_agent", "features": ""}"#
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(
            ToolScope::LocalWorkspace,
            vec![ToolIntent::RunCommand, ToolIntent::ExecuteSideEffect],
        )
        .with_risk(ToolRisk::High)
        .with_output_shape(ToolOutputShape::PlainText)
        .with_side_effects(true)
        .with_preferred_use_cases(vec![ToolUseCase::Testing])
        .with_disallowed_use_cases(vec![
            ToolUseCase::DirectExplanation,
            ToolUseCase::TimeSensitiveCurrent,
            ToolUseCase::ExternalGitHubRepo,
        ])
        .with_requires_explicit_user_request(true)
        .with_critic_mode(ToolCriticMode::Always)
    }

    fn timeout_secs(&self) -> Option<u64> {
        Some(self.timeout_secs)
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        self.execute_with_cancel(args, CancellationToken::new())
            .await
    }

    async fn execute_with_cancel(
        &self,
        args: Value,
        cancel_token: CancellationToken,
    ) -> Result<String, String> {
        let package = args.get("package").and_then(|v| v.as_str());
        let test_name = args.get("test_name").and_then(|v| v.as_str());
        let features = args.get("features").and_then(|v| v.as_str());

        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        cmd.current_dir(&self.project_root);

        if let Some(pkg) = package {
            cmd.arg("-p").arg(pkg);
        }

        if let Some(name) = test_name {
            cmd.arg(name);
        }

        if let Some(feat) = features {
            if !feat.is_empty() {
                cmd.arg("--features").arg(feat);
            }
        }

        cmd.arg("--");
        cmd.arg("--nocapture");
        cmd.kill_on_drop(true);

        let output = tokio::select! {
            _ = cancel_token.cancelled() => {
                return Err("Cancelled by user".to_string());
            }
            result = tokio::time::timeout(
                tokio::time::Duration::from_secs(self.timeout_secs),
                cmd.output(),
            ) => result
                .map_err(|_| "Test execution timed out")?
                .map_err(|e| format!("Failed to run tests: {}", e))?,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        let mut result = format!(
            "Test Result: {}\nExit Code: {}\n\n",
            if success { "✓ PASSED" } else { "✗ FAILED" },
            exit_code
        );

        if !stdout.is_empty() {
            result.push_str("STDOUT:\n");
            result.push_str(&stdout);
            result.push('\n');
        }

        if !stderr.is_empty() {
            result.push_str("STDERR:\n");
            result.push_str(&stderr);
        }

        if success {
            Ok(result)
        } else {
            Err(result)
        }
    }
}
