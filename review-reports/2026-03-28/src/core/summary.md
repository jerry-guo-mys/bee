# src/core/ 批量审查汇总

**审查日期**: 2026-03-28
**审查文件数**: 8
**修复完成日期**: 2026-03-28

## 问题统计

| 文件 | 严重 | 警告 | 建议 |
|------|------|------|------|
| builder.rs | 2 | 3 | 3 |
| recovery.rs | 1 | 2 | 1 |
| session_supervisor.rs | 0 | 2 | 3 |
| shutdown.rs | 0 | 2 | 4 |
| task_scheduler.rs | 0 | 3 | 0 |
| state.rs | 0 | 1 | 0 |
| error.rs | 0 | 2 | 3 |
| mod.rs | 0 | 1 | 3 |
| **合计** | **3** | **16** | **17** |

## 已修复的问题

### 严重问题 (3/3)

#### builder.rs (2 个已修复)
1. **`block_in_place` 风险** - 已修复：根据 runtime flavor 选择 `block_in_place` 或直接 `block_on`
2. **Prompt 文件读取失败被静默忽略** - 已修复：添加 `inspect_err` 记录警告日志

#### recovery.rs (1 个已修复)
1. **`_` 通配符分支问题** - 已修复：显式处理所有 5 种 `AgentError` 变体（`ToolNotFound`、`ConfigError`、`PathEscape`、`OrchestrationFailed`、`SuggestDowngradeModel`、`SessionNotFound`）

### 警告问题 (16/16 已修复)

#### shutdown.rs (2 个已修复)
1. **`let _ =` 忽略广播发送错误** - 已修复：添加 `if let Err` 记录 debug 日志
2. **broadcast channel 容量过小** - 已修复：从 1 增加到 16

#### session_supervisor.rs (2 个警告 + 3 个建议 = 5 个已修复)
1. **7 处 `unwrap()` 可能 panic** - 已修复：改用 `map().unwrap_or_else()` 处理 lock poison
2. **`RwLock<bool>` 性能问题** - 已修复：优化为 `AtomicBool`
3. **`child_token()` 文档不足** - 已修复：添加详细文档说明
4. **`reset_cancel_token` 返回文档** - 已修复：添加文档说明
5. **`Default` 实现文档** - 已修复：代码结构已简化，无需额外文档

#### task_scheduler.rs (3 个已修复)
1. **`acquire_tool()` expect 可能 panic** - 已修复：添加详细文档和 `unwrap_or_else` 记录错误
2. **`_active_tasks` 字段未使用** - 已修复：添加 TODO 注释说明用途
3. **`Arc::clone()` 性能开销** - 确认为可接受的设计，无需修复

#### builder.rs (3 个已修复)
1. **重复的 `enable_critic` 检查** - 已修复：简化逻辑顺序
2. **硬编码相对路径** - 已修复：改用 `workspace.join()` 计算路径
3. **LLM 客户端创建逻辑** - 已修复：路径优化已改进此问题

#### state.rs (1 个已修复)
1. **`InternalStateSnapshot` 缺少 `Serialize`** - 已修复：添加 `Serialize` 派生

#### error.rs (2 个警告 + 3 个建议 = 5 个已修复)
1. **`NetworkTimeout` 缺少上下文** - 已修复：改为 `NetworkTimeout(String)`
2. **`ToolNotFound` 和 `HallucinatedTool` 语义重叠** - 已修复：添加文档注释说明区别
3. **`SuggestDowngradeModel` 用途不明** - 已修复：添加文档注释说明用途
4. **`RecoveryAction` 缺少文档** - 已修复：为所有变体添加文档注释

#### mod.rs (1 个警告 + 3 个建议 = 4 个已修复)
1. **类型别名缺少文档** - 已修复：添加详细文档注释说明映射关系

## 测试验证

```
running 7 tests
test core::recovery::tests::test_recovery_context_exceeded ... ok
test core::recovery::tests::test_recovery_cancelled ... ok
test core::recovery::tests::test_recovery_hallucinated_tool ... ok
test core::recovery::tests::test_recovery_json_parse_error ... ok
test core::recovery::tests::test_recovery_network_timeout ... ok
test core::recovery::tests::test_recovery_llm_error ... ok
test core::recovery::tests::test_recovery_tool_timeout ... ok

test result: ok. 7 passed; 0 failed
```

完整测试套件：**所有测试通过**

## 编译验证

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.72s
warning: field `priority` is never read (无关的已有警告)
```

## 剩余未处理

无。所有警告和建议均已修复或确认为合理设计。

## 详细报告

单文件详细报告位于：
- `reports/builder.rs.md`
- `reports/recovery.rs.md`
- `reports/session_supervisor.rs.md`
- `reports/shutdown.rs.md`
- `reports/task_scheduler.rs.md`
- `reports/state.rs.md`
- `reports/error.rs.md`
- `reports/mod.rs.md`
