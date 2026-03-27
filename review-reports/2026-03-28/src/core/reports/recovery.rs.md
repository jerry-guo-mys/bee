# Rust 代码审查报告

## 业务场景和职责

**文件**: `/Users/g/Documents/GitHub/feature/org_20260321/src/core/recovery.rs`

`RecoveryEngine` 是错误恢复引擎，负责根据 `AgentError` 类型返回 `RecoveryAction`，供 ReAct 循环决定是重试、剪枝、询问用户还是终止。

### 关键依赖和设计权衡

- **依赖**: `crate::core::{AgentError, RecoveryAction}`, `crate::memory::Message`
- **设计模式**: 策略模式 - 将错误类型映射到恢复策略
- **架构位置**: core 层，与 `error.rs` 紧密耦合，供 `react` 层（ReAct 循环）调用
- **设计确认**: `Default` trait 实现冗余（第 9-15 行），`RecoveryEngine` 无状态，`new()` 和 `Default` 功能重复

---

## 问题

### 1. ⚠️ 警告：未使用的 `_history` 参数

**问题代码**（第 18 行）:
```rust
pub fn handle(&self, err: &AgentError, _history: &mut [Message]) -> RecoveryAction {
```

**触发场景**: 当前所有错误处理逻辑都未使用 `history` 参数，编译器会发出 `unused_variables` 警告。

**修复方案**:
```rust
// 方案 1: 使用 underscore 前缀表明有意不使用
pub fn handle(&self, err: &AgentError, _history: &mut [Message]) -> RecoveryAction {
    // ... 注释说明：预留用于未来「剪枝后重试」逻辑
}

// 方案 2: 如果确实不需要，移除参数
pub fn handle(&self, err: &AgentError) -> RecoveryAction {
    // ...
}

// 方案 3: 在函数体内显式使用
pub fn handle(&self, err: &AgentError, history: &mut [Message]) -> RecoveryAction {
    let _ = history; // 明确表明当前未使用但保留参数
    // ...
}
```

**影响**: 编译器警告，影响代码整洁度。

---

### 2. ❌ 严重：`_` 通配符分支可能掩盖未处理的错误类型

**问题代码**（第 41 行）:
```rust
_ => RecoveryAction::Abort,
```

**触发场景**: 当 `AgentError` 枚举新增变体但忘记更新 `handle` 函数时，会落入 `Abort` 分支，可能导致意外终止而非正确的恢复逻辑。

**当前 `AgentError` 变体**（来自 `error.rs`）:
- `Cancelled` → 已处理（第 40 行）
- `NetworkTimeout` → 已处理（第 36-38 行）
- `ContextWindowExceeded` → 已处理（第 26 行）
- `JsonParseError` → 已处理（第 20-25 行）
- `ToolExecutionFailed` → 已处理（第 33-35 行）
- `ToolTimeout` → 已处理（第 30-32 行）
- `HallucinatedTool` → 已处理（第 27-29 行）
- `ToolNotFound` → **未显式处理，落入 `_` 分支**
- `LlmError` → 已处理（第 39 行）
- `SuggestDowngradeModel` → **未显式处理，落入 `_` 分支**
- `ConfigError` → **未显式处理，落入 `_` 分支**
- `PathEscape` → **未显式处理，落入 `_` 分支**
- `OrchestrationFailed` → **未显式处理，落入 `_` 分支**

**修复方案**:
```rust
// 显式处理所有变体，移除通配符分支
pub fn handle(&self, err: &AgentError, _history: &mut [Message]) -> RecoveryAction {
    match err {
        AgentError::JsonParseError(raw) => RecoveryAction::RetryWithPrompt(format!(
            // ...
        )),
        AgentError::ContextWindowExceeded => RecoveryAction::SummarizeAndPrune,
        AgentError::HallucinatedTool(name) => RecoveryAction::AskUser(format!(
            // ...
        )),
        AgentError::ToolTimeout(_) => {
            RecoveryAction::AskUser("工具执行超时，是否重试？".to_string())
        }
        AgentError::ToolExecutionFailed(msg) => {
            RecoveryAction::AskUser(format!("工具执行失败：{msg}"))
        }
        AgentError::NetworkTimeout => {
            RecoveryAction::RetryWithPrompt("网络请求超时，请重试。".to_string())
        }
        AgentError::LlmError(_) => RecoveryAction::DowngradeModel,
        AgentError::Cancelled => RecoveryAction::Abort,

        // 新增显式处理
        AgentError::ToolNotFound(name) => RecoveryAction::AskUser(format!(
            "工具 '{name}' 未找到，是否需要安装？"
        )),
        AgentError::SuggestDowngradeModel(_) => RecoveryAction::DowngradeModel,
        AgentError::ConfigError(msg) => RecoveryAction::AskUser(format!(
            "配置错误：{msg}"
        )),
        AgentError::PathEscape(path) => RecoveryAction::Abort, // 安全相关，直接终止
        AgentError::OrchestrationFailed(msg) => RecoveryAction::AskUser(format!(
            "编排失败：{msg}"
        )),
    }
}
```

**影响**:
- `ToolNotFound` 本应询问用户是否需要安装，却直接 `Abort` 终止任务
- `PathEscape` 是安全相关错误，应记录日志或通知用户而非静默终止
- 未来新增 `AgentError` 变体时，编译器不会警告遗漏处理

---

### 3. ⚠️ 警告：`String` 参数的 `.to_string()` 调用冗余

**问题代码**（第 31、34、37 行）:
```rust
RecoveryAction::AskUser("工具执行超时，是否重试？".to_string())
RecoveryAction::AskUser(format!("工具执行失败：{msg}"))
RecoveryAction::RetryWithPrompt("网络请求超时，请重试。".to_string())
```

**触发场景**: 字符串字面量调用 `.to_string()` 在 Rust 中是合法的，但如果 `RecoveryAction` 的构造器接受 `impl Into<String>`，则可以直接传入 `&str`。

**检查 `RecoveryAction` 定义**:
```rust
pub enum RecoveryAction {
    RetryWithPrompt(String),  // 需要 String
    AskUser(String),          // 需要 String
    // ...
}
```

**分析**: 由于 `RecoveryAction` 变体直接存储 `String`，`.to_string()` 是必要的。但 `"xxx".to_string()` 可以替换为 `"xxx".to_owned()` 或 `String::from("xxx")`，性能相同但语义更清晰。

**修复方案**（可选优化）:
```rust
// 方案 1: 使用 to_owned()
RecoveryAction::AskUser("工具执行超时，是否重试？".to_owned())

// 方案 2: 使用 String::from()
RecoveryAction::RetryWithPrompt(String::from("网络请求超时，请重试。"))

// 方案 3: 使用 format! 统一风格
RecoveryAction::AskUser(format!("工具执行超时，是否重试？"))
```

**影响**: 轻微性能差异，主要是代码风格一致性。

---

### 4. 💡 建议：测试代码可覆盖更多错误变体

**问题代码**（第 46-118 行）:
当前测试覆盖了 6 种错误类型，但 `AgentError` 共有 13 种变体。

**修复方案**:
```rust
#[test]
fn test_recovery_tool_not_found() {
    let engine = RecoveryEngine::new();
    let err = AgentError::ToolNotFound("missing_tool".to_string());
    let action = engine.handle(&err, &mut []);
    // 期望：AskUser 或 Abort（取决于实现）
    assert!(matches!(action, RecoveryAction::Abort)); // 当前通配符分支
}

#[test]
fn test_recovery_config_error() {
    let engine = RecoveryEngine::new();
    let err = AgentError::ConfigError("invalid config".to_string());
    let action = engine.handle(&err, &mut []);
    // 期望：AskUser 或 Abort
}

#[test]
fn test_recovery_path_escape() {
    let engine = RecoveryEngine::new();
    let err = AgentError::PathEscape("/etc/passwd".to_string());
    let action = engine.handle(&err, &mut []);
    // 期望：Abort（安全相关）
}
```

**影响**: 提高测试覆盖率，确保所有错误类型都有预期的恢复行为。

---

## 设计确认（非问题）

- **`Default` trait 实现**: 第 9-15 行的 `#[derive(Debug, Default)]` 和 `new()` 方法虽然冗余，但在 Rust 中是常见模式，便于未来扩展（如需初始化内部状态）。
- **`_history` 参数保留**: 注释说明"预留用于未来「剪枝后重试」等逻辑"，这是合理的前瞻性设计。
- **无状态设计**: `RecoveryEngine` 不持有任何状态，符合策略模式的轻量级设计。

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `.clone()`、`Arc<Mutex<T>>` | ✅ 无相关问题 |
| 错误处理 | `unwrap()`、`let _ =`、`?` | ⚠️ 通配符分支 `_ => Abort` 可能掩盖问题 |
| Async | 阻塞调用、`spawn_blocking` | ✅ 无异步代码 |
| 命名规范 | `PascalCase`/`snake_case` | ✅ 符合 |
| 代码风格 | 导入分组、行长度 | ✅ 符合 |

---

## 总结

| 等级 | 数量 |
|------|------|
| ❌ 严重 | 1 |
| ⚠️ 警告 | 2 |
| 💡 建议 | 1 |

**关键问题**: `_` 通配符分支导致 `ToolNotFound`、`ConfigError`、`PathEscape`、`OrchestrationFailed`、`SuggestDowngradeModel` 五种错误类型未被显式处理，可能产生非预期的 `Abort` 行为。
