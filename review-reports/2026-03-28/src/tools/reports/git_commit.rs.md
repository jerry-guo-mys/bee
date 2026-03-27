# Rust 代码审查报告：git_commit.rs

## 业务场景和职责
- 执行 git add 和 commit 保存代码修改
- 支持指定文件列表或默认所有文件

---

## 问题

### 1. **使用 tokio::process::Command 但无超时**
**行号**: 69-82, 94-99
```rust
let mut add_cmd = Command::new("git");
// ...
let add_output = add_cmd.output().await
```
**触发场景**: git 命令可能挂起（如等待 GPG 签名），无超时保护
**修复方案**: 添加超时：
```rust
use tokio::time::{timeout, Duration};
let output = timeout(
    Duration::from_secs(30),
    cmd.output()
).await
.map_err(|_| "Command timed out")?
.map_err(|e| format!("Execution failed: {}", e))?;
```

### 2. **错误信息泄露内部命令细节**
**行号**: 86, 103
```rust
.map_err(|e| format!("Failed to run git add: {}", e))?;
```
**触发场景**: 错误信息包含系统细节，可能泄露环境信息
**修复方案**: 简化错误信息：
```rust
.map_err(|_| "Failed to run git add".to_string())?;
```

### 3. **项目根目录验证缺失**
**行号**: 17-21
```rust
pub fn new(project_root: impl AsRef<Path>) -> Self {
    Self {
        project_root: project_root.as_ref().to_path_buf(),
    }
}
```
**触发场景**: 未验证 project_root 是否存在或确实是 git 仓库
**修复方案**: 添加验证：
```rust
pub fn new(project_root: impl AsRef<Path>) -> Result<Self, String> {
    let root = project_root.as_ref().to_path_buf();
    if !root.join(".git").exists() {
        return Err("Not a git repository".to_string());
    }
    Ok(Self { project_root: root })
}
```

### 4. **git commit 失败时 stdout 丢失**
**行号**: 105-111
```rust
let stdout = String::from_utf8_lossy(&commit_output.stdout);
let stderr = String::from_utf8_lossy(&commit_output.stderr);
if commit_output.status.success() {
    // ...
} else {
    Err(format!("git commit failed: {}", stderr))
}
```
**修复方案**: 同时返回 stdout 和 stderr 便于调试：
```rust
Err(format!("git commit failed: {}\n{}", stderr, stdout))
```

---

## 设计确认（非问题）
- 支持文件列表参数是好的设计
- 默认 "." 添加所有文件符合 git 习惯
- ToolMetadata 配置正确（High risk, Always critic）

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ 错误信息可改进 |
| Async | ✓ 使用 tokio::process |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：2
