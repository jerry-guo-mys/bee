# Rust 代码审查报告：git_diff.rs

## 业务场景和职责
- 显示 git diff 工具
- 支持多种模式：unstaged/staged/commit/branch
- 支持统计模式和特定文件过滤

---

## 问题

### 1. **使用 std::process::Command 而非异步**
**行号**: 24-42
```rust
fn run_git_command(&self, args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    // 使用 std::process::Command
```
**触发场景**: 阻塞调用可能卡住 async runtime
**修复方案**: 使用 tokio::process::Command 或 spawn_blocking：
```rust
use tokio::process::Command;
// 或在 spawn_blocking 中执行
```

### 2. **Default trait 实现冗余**
**行号**: 13-17
```rust
impl Default for GitDiffTool {
    fn default() -> Self {
        Self::new()
    }
}
```
**触发场景**: GitDiffTool 是无状态单例，Default 实现可以，但 new() 只是返回 Self
**修复方案**: 考虑使用单元结构体：
```rust
pub struct GitDiffTool;  // 不需要字段
impl GitDiffTool {
    pub fn new() -> Self { Self }
}
```

### 3. **模式验证不完整**
**行号**: 95-117
```rust
match mode {
    "unstaged" => {}
    "staged" | "cached" => { ... }
    "commit" => { ... }
    "branch" => { ... }
    _ => return Err(format!("Invalid mode: {}", mode)),
}
```
**设计确认**: 模式验证是完整的

### 4. **错误信息不够详细**
**行号**: 126
```rust
Err(e) => return Err(format!("Git command failed: {}", e)),
```
**触发场景**: 调用方可能想知道具体是哪个命令失败
**修复方案**: 添加命令信息：
```rust
Err(format!("Git diff command failed: {}", e))
```

---

## 设计确认（非问题）
- 支持多种 diff 模式是好的设计
- stat 参数支持 --stat 输出
- 空结果返回友好提示

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无状态结构体 |
| 错误处理 | ⚠️ 部分错误信息可改进 |
| Async | ❌ 使用同步 Command |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2 (同步 Command)
- 💡 建议：1
