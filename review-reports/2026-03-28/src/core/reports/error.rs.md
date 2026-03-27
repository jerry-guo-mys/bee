# Rust 代码审查报告

## 业务场景和职责

**文件**: `/Users/g/Documents/GitHub/feature/org_20260321/src/core/error.rs`

`AgentError` 是 Agent 运行过程中的错误类型定义，与 `RecoveryAction` 配合 `RecoveryEngine` 使用，实现错误恢复策略。

### 关键依赖和设计权衡

- **依赖**: `thiserror::Error`, `crate::llm::LlmError`
- **设计模式**: 错误枚举 + thiserror 派生宏
- **架构位置**: core 层，被 `recovery.rs`、`react` 层、`orchestrator.rs` 等调用
- **关联文件**: `recovery.rs` (RecoveryEngine), `llm/traits.rs` (LlmError)

---

## 问题

### 1. ⚠️ 警告：部分错误变体未携带足够上下文

**问题代码**（第 15-16 行）:
```rust
#[error("Network timeout")]
NetworkTimeout,
```

**触发场景**: 当网络超时时，无法区分是哪个操作、哪个端点超时，不利于日志追踪和问题诊断。

**修复方案**:
```rust
// 方案 1: 携带操作描述
#[error("Network timeout: {0}")]
NetworkTimeout(String),

// 方案 2: 携带结构化信息
#[error("Network timeout during {operation} after {elapsed_ms}ms")]
NetworkTimeout {
    operation: String,
    elapsed_ms: u64,
},
```

**影响**: 日志和错误追踪时缺乏上下文，增加调试难度。

---

### 2. ⚠️ 警告：`ToolNotFound` 和 `HallucinatedTool` 语义重叠

**问题代码**（第 30-34 行）:
```rust
#[error("Hallucinated tool: {0}")]
HallucinatedTool(String),

#[error("Tool not found: {0}")]
ToolNotFound(String),
```

**触发场景**: 两者都表示工具不存在，但语义上略有不同：
- `HallucinatedTool`: LLM 幻觉出根本不存在的工具名
- `ToolNotFound`: 工具应该存在但未找到（可能是技能未加载）

**修复方案**:
```rust
// 方案 1: 合并为一个错误，用注释说明语义
/// 工具不存在（包括 LLM 幻觉或技能未加载）
#[error("Tool not found: {0}")]
ToolNotFound(String),

// 方案 2: 保留但增加注释说明使用场景
/// LLM 幻觉出不存在的工具名
#[error("Hallucinated tool: {0} (model made up this tool name)")]
HallucinatedTool(String),

/// 工具应该存在但未找到（如技能文件缺失）
#[error("Tool not found: {0} (tool exists but not loaded)")]
ToolNotFound(String),
```

**影响**: 当前设计可能导致混淆，但如果团队内部有明确的使用约定则是合理的。

---

### 3. 💡 建议：`SuggestDowngradeModel` 可能永远不会被直接使用

**问题代码**（第 39-41 行）:
```rust
/// 恢复引擎建议降级模型（如 LLM 持续失败时），由上层决定是否切换轻量模型
#[error("Suggest downgrade model: {0}")]
SuggestDowngradeModel(String),
```

**触发场景**: 查看 `recovery.rs` 第 39 行，`LlmError` 映射到 `RecoveryAction::DowngradeModel`，而非 `AgentError::SuggestDowngradeModel`。此错误变体似乎未被 `RecoveryEngine` 使用。

**修复方案**:
```rust
// 方案 1: 如果确实需要，添加使用场景的注释
/// 保留用于未来扩展：当 Agent 直接建议降级模型时（非 LlmError 触发）
#[error("Suggest downgrade model: {0}")]
SuggestDowngradeModel(String),

// 方案 2: 如果确认无用，考虑移除
// 删除 SuggestDowngradeModel 变体
```

**影响**: 未使用的代码变体增加维护负担。

---

### 4. 💡 建议：`RecoveryAction` 缺少文档注释

**问题代码**（第 54-65 行）:
```rust
/// 恢复引擎根据错误类型给出的建议动作
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    RetryWithPrompt(String),
    SummarizeAndPrune,
    AskUser(String),
    DowngradeModel,
    Abort,
}
```

**触发场景**: `RecoveryAction` 变体缺少文档注释，不利于新成员理解每个动作的触发条件和处理方式。

**修复方案**:
```rust
/// 恢复引擎根据错误类型给出的建议动作
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// 将提示注入下一轮，让 LLM 重试（如 JSON 格式错误）
    RetryWithPrompt(String),
    /// 压缩上下文后继续（如超长上下文）
    SummarizeAndPrune,
    /// 需要用户决策（如幻觉工具、超时）
    AskUser(String),
    /// 降级到更轻量模型（如 LLM 持续失败时）
    DowngradeModel,
    /// 终止当前任务（如用户取消或不可恢复错误）
    Abort,
}
```

**影响**: 代码可读性降低，新成员需要查看 `recovery.rs` 实现才能理解每个动作的含义。

---

### 5. 💡 建议：`Clone` derive 对 `AgentError` 可能不必要

**问题代码**（第 10 行）:
```rust
#[derive(Error, Debug)]
pub enum AgentError {
```

**触发场景**: `AgentError` 没有派生 `Clone`，这是合理的，因为错误通常是消耗型的。但检查是否有其他地方需要 `Clone`。

**分析**: 当前没有 `Clone` 派生，这是正确的设计。错误通常是"发生即消耗"的，不需要克隆。

---

## 设计确认（非问题）

### 1. thiserror 使用正确
- `#[from]` 属性正确处理 `LlmError` 转换
- `#[error(...)]` 消息格式化清晰

### 2. 错误分类清晰
- 网络错误：`NetworkTimeout`, `ToolTimeout`
- LLM 错误：`LlmError` (嵌套), `ContextWindowExceeded`
- 工具错误：`ToolExecutionFailed`, `ToolNotFound`, `HallucinatedTool`
- 用户操作：`Cancelled`
- 配置/安全：`ConfigError`, `PathEscape`

### 3. `RecoveryAction` 设计合理
- 与 `RecoveryEngine::handle` 配合良好
- 覆盖重试、剪枝、询问、降级、终止等策略

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `.clone()`、`Arc<Mutex<T>>` | N/A (纯错误定义) |
| 错误处理 | `unwrap()`、`let _ =`、`?` | 符合规范 |
| Async | 阻塞调用、`spawn_blocking` | N/A (同步代码) |
| 错误类型 | thiserror 正确使用 | 通过 |
| 文档注释 | 公共 API 文档完整性 | 需要补充 |

---

## 总结

| 等级 | 数量 |
|------|------|
| ❌ 严重 | 0 |
| ⚠️ 警告 | 2 |
| 💡 建议 | 3 |

**总体评价**: `error.rs` 代码质量良好，错误类型定义清晰，与 `RecoveryEngine` 配合合理。主要改进空间在于：
1. 为无参数错误变体增加上下文信息
2. 澄清语义重叠的错误变体
3. 补充 `RecoveryAction` 的文档注释
