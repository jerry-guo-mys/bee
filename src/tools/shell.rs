//! Shell 执行器：白名单命令，禁止危险操作
//!
//! 仅允许配置中的命令名（首词，如 ls、grep、cargo）；禁止 rm -rf、wget、chmod 777 等子串；
//! 执行通过 sh -c / cmd /C，带超时与 tracing 审计。

use std::collections::HashSet;

use async_trait::async_trait;
use serde_json::Value;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::tools::{
    Tool, ToolCapabilityGroup, ToolCapabilitySubgroup, ToolCostClass, ToolCriticMode, ToolIntent,
    ToolMetadata, ToolOutputShape, ToolRisk, ToolScope, ToolUseCase,
};

/// 禁止的命令子串（简单快速检查）
const FORBIDDEN_SUBSTR: &[&str] = &[
    "mkfs",
    "dd if=",
    "> /dev/sd",
    ":(){ :|:& };:", // fork bomb
];

/// 禁止的命令/参数模式（使用正则匹配）
fn is_forbidden_command(raw: &str) -> Result<(), String> {
    let raw_lower = raw.to_lowercase();

    // 检查禁止的子串
    for forbidden in FORBIDDEN_SUBSTR {
        if raw_lower.contains(forbidden) {
            return Err(format!("Forbidden pattern: {}", forbidden));
        }
    }

    // 解析命令和参数，检查变体绕过
    let parts: Vec<&str> = raw_lower.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(());
    }

    let cmd = parts[0];
    let args = &parts[1..];

    // 检查 rm 命令的危险参数组合
    if cmd == "rm" || cmd.ends_with("/rm") {
        let has_recursive = args.iter().any(|a| a.starts_with("-r") || *a == "--recursive");
        let has_force = args.iter().any(|a| a.starts_with("-f") || *a == "--force");
        let has_root = args.iter().any(|a| *a == "/" || a.starts_with("/*"));
        let has_home = args.iter().any(|a| a.starts_with("~") || a.starts_with("$HOME"));

        if has_recursive && has_force {
            return Err("Forbidden pattern: rm with -r and -f flags".to_string());
        }
        if has_recursive && (has_root || has_home) {
            return Err("Forbidden pattern: rm -r on root or home directory".to_string());
        }
    }

    // 检查 chmod 危险模式
    if cmd == "chmod" || cmd.ends_with("/chmod") {
        if args.iter().any(|a| *a == "777" || a.starts_with("777") || *a == "+s") {
            return Err("Forbidden pattern: chmod with dangerous permissions".to_string());
        }
    }

    // 检查 wget/curl 管道执行
    if (cmd == "wget" || cmd == "curl") && raw_lower.contains("| sh") {
        return Err("Forbidden pattern: downloading and executing script".to_string());
    }

    Ok(())
}

/// Shell 工具：仅允许白名单内命令
pub struct ShellTool {
    allowed_commands: HashSet<String>,
    timeout_secs: u64,
}

impl ShellTool {
    pub fn new(allowed_commands: Vec<String>, timeout_secs: u64) -> Self {
        let allowed_commands = allowed_commands
            .into_iter()
            .map(|s| s.to_lowercase())
            .collect();
        Self {
            allowed_commands,
            timeout_secs,
        }
    }

    /// 解析命令：提取实际命令名（处理路径和前缀）
    fn command_name<'a>(&self, raw: &'a str) -> &'a str {
        let first = raw.split_whitespace().next().unwrap_or("");
        // 处理路径如 /bin/ls -> ls
        let cmd = first.rsplit('/').next().unwrap_or(first);
        // 处理 sudo/doas 前缀
        if cmd == "sudo" || cmd == "doas" {
            // 获取 sudo 后面的命令
            raw.split_whitespace().nth(1).unwrap_or("")
        } else {
            cmd
        }
    }

    fn is_allowed(&self, raw: &str) -> Result<(), String> {
        // 首先检查禁止的命令和参数模式
        is_forbidden_command(raw)?;

        // 提取命令名并检查白名单
        let name = self.command_name(raw);
        if name.is_empty() {
            return Err("Empty command".to_string());
        }
        if self.allowed_commands.contains(name) {
            return Ok(());
        }
        Err(format!("Command '{}' not in allowlist", name))
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Run a whitelisted shell command. Allowed commands: ls, grep, cat, head, tail, wc, find, cargo, rustc (configurable)."
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(
            ToolScope::System,
            vec![ToolIntent::RunCommand, ToolIntent::ExecuteSideEffect],
        )
        .with_risk(ToolRisk::High)
        .with_output_shape(ToolOutputShape::PlainText)
        .with_side_effects(true)
        .with_disallowed_use_cases(vec![
            ToolUseCase::DirectExplanation,
            ToolUseCase::TimeSensitiveCurrent,
            ToolUseCase::ExternalGitHubRepo,
        ])
        .with_requires_explicit_user_request(true)
        .with_capability(
            ToolCapabilityGroup::SystemExecution,
            ToolCapabilitySubgroup::CommandExecution,
        )
        .with_costs(
            ToolCostClass::Medium,
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Medium,
        )
        .with_preferred_rank(10)
        .with_critic_mode(ToolCriticMode::Always)
    }

    fn timeout_secs(&self) -> Option<u64> {
        Some(self.timeout_secs)
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute (must be in allowlist)"
                }
            },
            "required": ["command"]
        })
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
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        self.is_allowed(command)?;

        tracing::info!(command = %command, "shell tool execute");

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        cmd.kill_on_drop(true);

        let output = tokio::select! {
            _ = cancel_token.cancelled() => {
                return Err("Cancelled by user".to_string());
            }
            result = tokio::time::timeout(
                std::time::Duration::from_secs(self.timeout_secs),
                cmd.output(),
            ) => result
                .map_err(|_| format!("Command timed out after {}s", self.timeout_secs))?
                .map_err(|e| format!("Execution failed: {}", e))?,
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            return Err(format!(
                "Exit {:?}\nstderr: {}",
                output.status,
                stderr.trim()
            ));
        }
        Ok(if stderr.is_empty() {
            stdout
        } else {
            format!("{}\nstderr: {}", stdout.trim(), stderr.trim())
        })
    }
}
