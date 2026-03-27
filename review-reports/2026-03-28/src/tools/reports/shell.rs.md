# Rust 代码审查报告：shell.rs

## 业务场景和职责
- Shell 执行器：仅允许白名单命令
- 禁止危险操作（rm -rf、wget 等）
- 带超时与审计日志

---

## 问题

### 1. **FORBIDDEN_SUBSTR 检查可能被绕过**
**行号**: 19-31
```rust
const FORBIDDEN_SUBSTR: &[&str] = &[
    "rm -rf",
    "rm -fr",
    // ...
];
```
**触发场景**: 攻击者可能使用 `rm -r -f` 或 `rm --recursive --force` 绕过
**修复方案**: 使用更严格的解析：
```rust
// 解析命令参数，检查是否有危险标志
```

### 2. **命令名提取过于简单**
**行号**: 52-54
```rust
fn command_name<'a>(&self, raw: &'a str) -> &'a str {
    raw.split_whitespace().next().unwrap_or("")
}
```
**触发场景**: 如果命令有前缀如 `sudo ls` 或路径 `/bin/ls`，可能绕过白名单
**修复方案**: 提取实际命令名并规范化：
```rust
fn command_name<'a>(&self, raw: &'a str) -> &'a str {
    let first = raw.split_whitespace().next().unwrap_or("");
    // 提取 basename
    first.rsplit('/').next().unwrap_or(first)
}
```

### 3. **shell 注入风险（通过参数）**
**行号**: 148-156
```rust
let mut cmd = if cfg!(target_os = "windows") {
    let mut c = Command::new("cmd");
    c.args(["/C", command]);
    // ...
} else {
    let mut c = Command::new("sh");
    c.args(["-c", command]);
}
```
**触发场景**: 使用 `sh -c` 执行，LLM 可能传入 `ls; rm -rf /` 这样的命令
**修复方案**: 当前设计依赖 is_allowed 检查，但应确保检查在解析后：
```rust
// 当前设计：is_allowed 检查整个命令字符串，包括参数
// 建议：添加额外的参数级检查
```

### 4. **超时错误信息不够详细**
**行号**: 168
```rust
.map_err(|_| format!("Command timed out after {}s", self.timeout_secs))?
```
**修复方案**: 添加命令信息：
```rust
.map_err(|_| format!("Command '{}' timed out after {}s", command, self.timeout_secs))?
```

### 5. **魔术数字硬编码**
**行号**: 36
```rust
timeout_secs: u64,  // 由配置决定，但无默认值说明
```
**修复方案**: 添加默认值常量：
```rust
const DEFAULT_TIMEOUT_SECS: u64 = 60;
```

---

## 设计确认（非问题）
- 白名单 + 黑名单双层检查是好的设计
- 审计日志（tracing::info）是好的实践
- 使用 sh -c / cmd /C 正确

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ 部分错误信息可改进 |
| Async | ✓ 使用 tokio::process |

## 问题统计
- ❌ 严重：1 (命令绕过风险)
- ⚠️ 警告：2
- 💡 建议：1
