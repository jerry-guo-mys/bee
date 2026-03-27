# Rust 代码审查报告

## 业务场景和职责

**文件**: `src/core/task_scheduler.rs`

**职责**: 任务调度模块，按任务类型（AgentStep / ToolExecution / Background）分类管理，工具执行使用 Semaphore 限制并发。

**设计意图**:
- `TaskKind` 枚举区分三种任务类型：前台串行（AgentStep）、可并行受限（ToolExecution）、后台不阻塞 UI（Background）
- `TaskScheduler` 使用 `Semaphore` 限制工具并发执行数量（默认 3）
- 使用 `AtomicU64` 生成全局唯一的任务 ID
- 使用 `CancellationToken` 支持任务取消检查

**上下游影响**:
- 上游：被 `src/core/orchestrator.rs` 调用，用于控制 ReAct 循环中的任务调度
- 下游：与 `src/react/loop_.rs` 配合，管理工具执行的并发

**关键依赖**:
- `tokio::sync::Semaphore` - 信号量并发控制
- `tokio_util::sync::CancellationToken` - 取消令牌

---

## 问题

### 1. ⚠️ 警告：`acquire_tool()` 返回 `expect("semaphore closed")` 可能 panic

**问题代码**（第 58-64 行）:
```rust
pub async fn acquire_tool(&self) -> tokio::sync::OwnedSemaphorePermit {
    self.tool_semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("semaphore closed")
}
```

**触发场景**: 当 `Semaphore` 被关闭（所有 permit 被遗忘或显式调用 `close()`）时，`acquire_owned()` 返回 `Err(ClosedError)`，此时 `expect()` 会触发 panic。

**影响分析**:
- 在正常运行场景中，`tool_semaphore` 由 `Arc` 持有且不会被关闭，此问题不会触发
- 但如果 `TaskScheduler` 被 drop 而仍有任务在等待 permit，可能导致 panic
- 更安全的做法是返回 `Result<OwnedSemaphorePermit, ClosedError>` 或使用 `unwrap_or_else` 提供更友好的错误处理

**修复方案**:
```rust
pub async fn acquire_tool(&self) -> Result<tokio::sync::OwnedSemaphorePermit, tokio::sync::AcquireError> {
    self.tool_semaphore
        .clone()
        .acquire_owned()
        .await
}
```

或者保持当前签名但使用更安全的处理：
```rust
pub async fn acquire_tool(&self) -> tokio::sync::OwnedSemaphorePermit {
    self.tool_semaphore
        .clone()
        .acquire_owned()
        .await
        .unwrap_or_else(|_| {
            // 信号量已关闭，返回一个已关闭的信号量的 permit（实际上不会发生）
            // 或者根据业务需求处理
            panic!("task_scheduler: semaphore unexpectedly closed")
        })
}
```

---

### 2. ⚠️ 警告：`_active_tasks` 字段未实际使用

**问题代码**（第 45-46 行）:
```rust
/// 活跃任务
_active_tasks: HashMap<TaskId, TaskKind>,
```

**触发场景**: `_active_tasks` 在 `new()` 中初始化为空 `HashMap`，但在整个模块中从未被修改或读取。

**影响分析**:
- 该字段标记为 `_` 前缀表示故意未使用，避免编译警告
- 可能是预留功能（用于跟踪活跃任务），但当前未实现
- 如果未来需要实现任务跟踪、取消或监控功能，需要补充相关逻辑

**修复方案**:
- 如果是预留设计，建议添加 TODO 注释说明意图
- 如果暂不需要，可考虑移除该字段以简化代码

```rust
/// 活跃任务（TODO: 用于任务跟踪和取消）
_active_tasks: HashMap<TaskId, TaskKind>,
```

---

### 3. ⚠️ 警告：`acquire_tool()` 不必要的 `clone()`

**问题代码**（第 59-60 行）:
```rust
self.tool_semaphore
    .clone()
    .acquire_owned()
```

**触发场景**: 每次调用 `acquire_tool()` 都会对 `Arc<Semaphore>` 进行 clone，增加引用计数操作。

**影响分析**:
- `Semaphore::acquire_owned()` 接受 `Arc<Semaphore>` 的 clone，这是设计所需
- 但 `clone()` 操作涉及原子引用计数增减，在高并发场景下可能成为微小瓶颈
- 可考虑直接持有 `Arc<Semaphore>` 而非在方法内 clone

**修复方案**:
```rust
// 方案 A：在结构体中存储 Arc 的 clone（当前设计已合理）
// 方案 B：返回非 owned permit（但生命周期受限）
pub async fn acquire_tool(&self) -> tokio::sync::SemaphorePermit<'_> {
    self.tool_semaphore.acquire().await.expect("semaphore closed")
}
```

注意：非 owned permit 的生命周期与 `&self` 绑定，可能不适用于跨 async 任务边界。当前设计使用 `OwnedSemaphorePermit` 是合理的，`clone()` 开销可接受。

---

## 设计确认（非问题）

1. **`TaskId` 使用 `AtomicU64` 生成唯一 ID** - 这是全局唯一 ID 的标准做法，`Relaxed` 内存序对 ID 生成足够（仅需原子性，无需顺序保证）。

2. **`ToolExecution` 默认并发限制为 3** - 这是一个合理的默认值，平衡了并发效率和资源消耗。通过 `max(1)` 确保至少允许 1 个并发，避免配置错误导致死锁。

3. **`_active_tasks` 使用 `HashMap<TaskId, TaskKind>`** - 虽然当前未使用，但设计合理，未来可用于任务跟踪、取消或监控。

4. **`is_cancelled()` 接受 `&CancellationToken` 而非存储于结构体** - 这是合理的设计，取消令牌通常由调用者管理（如 `SessionSupervisor` 或 `Orchestrator`），任务调度器仅被动检查。

5. **`TaskScheduler` 实现 `Default`** - 提供默认并发限制 3，符合 Rust 惯例，简化使用。

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `.clone()`、`Arc<Mutex<T>>` | ⚠️ `Arc::clone()` 用于 `acquire_tool()`（可接受） |
| 错误处理 | `unwrap()`、`let _ =`、`?` | ⚠️ `expect("semaphore closed")` 可能 panic |
| Async | 阻塞调用、`spawn_blocking` | ✅ 无阻塞调用 |
| 未使用字段 | `_active_tasks` | ⚠️ 预留但未使用 |
| 并发控制 | Semaphore 使用 | ✅ 正确使用 `OwnedSemaphorePermit` |
| 取消检查 | CancellationToken | ✅ 通过参数传递，设计合理 |

---

## 统计

- ❌ **严重**: 0 个
- ⚠️ **警告**: 3 个
- 💡 **建议**: 0 个
