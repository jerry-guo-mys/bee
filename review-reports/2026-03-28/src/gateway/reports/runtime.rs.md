# Rust 代码审查报告 - runtime.rs

## 业务场景和职责

**文件路径**: `src/gateway/runtime.rs`

**职责**: Agent Runtime（代理运行时），实际的 AI 处理逻辑，与 Gateway 解耦。

**关键设计**:
- `AgentRuntime` 封装 Agent 组件和会话存储
- `process_message` 处理用户消息并流式返回响应
- 支持 ReAct 循环和直接工具路由
- 多租户工作空间隔离
- 工具策略解析（基于作用域）

---

## 问题列表

### 1. ❌ 严重 - `run_react_loop` 中获取取消令牌时会话不存在则创建本地令牌，可能导致资源泄漏

**问题代码** (行 235-242):
```rust
let cancel_token = match self.session_store.new_cancel_token(session_id).await {
    Some(token) => token,
    None => {
        // 会话不存在，创建本地取消令牌
        tracing::warn!("Session {} not found, using local cancel token", session_id);
        tokio_util::sync::CancellationToken::new()
    }
};
```

**触发场景**: 当会话不存在时（如过期被清理），创建本地取消令牌但该令牌不会被会话管理追踪，如果请求长时间运行，无法通过会话管理取消。

**修复方案**: 会话不存在时应拒绝请求或重新创建会话：
```rust
let cancel_token = self.session_store.new_cancel_token(session_id).await
    .ok_or_else(|| AgentError::SessionNotFound(session_id.to_string()))?;
```

---

### 2. ❌ 严重 - `process_message` 中 spawn 的任务可能泄露敏感信息

**问题代码** (行 128-187):
```rust
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        let msg = match event {
            // ...
            ReactEvent::Error { text } => GatewayMessage::new(
                Some(session_id_owned.clone()),
                MessageType::Error {
                    request_id: Some(request_id_clone.clone()),
                    code: "react_error".to_string(),
                    message: text,  // 直接暴露内部错误
                },
            ),
            // ...
        };
        if response_tx_clone.send(msg).is_err() {
            break;
        }
    }
});
```

**触发场景**: 内部错误（如栈跟踪、路径信息）可能通过 `text` 泄露给客户端。

**修复方案**: 对错误进行过滤或脱敏：
```rust
ReactEvent::Error { text } => {
    tracing::error!("React error: {}", text);  // 完整日志
    GatewayMessage::new(
        Some(session_id_owned.clone()),
        MessageType::Error {
            request_id: Some(request_id_clone.clone()),
            code: "react_error".to_string(),
            message: "An internal error occurred".to_string(),  // 脱敏消息
        },
    )
}
```

---

### 3. ⚠️ 警告 - `process_message` 中响应通道发送失败未处理

**问题代码** (行 183-185):
```rust
if response_tx_clone.send(msg).is_err() {
    break;
}
```

**说明**: 仅 `break` 退出循环，但未通知 `process_message` 主函数，导致结果可能无法返回。

---

### 4. ⚠️ 警告 - `scoped_runtime_workspace` 中路径遍历未防止

**问题代码** (行 379-400):
```rust
fn scoped_runtime_workspace(base_workspace: &std::path::Path, scope: &SessionScope) -> PathBuf {
    let mut path = base_workspace.join(".bee").join("runtime_scopes");
    path.push(sanitize_scope_segment(
        scope.tenant_id.as_deref().unwrap_or("tenant-default"),
    ));
    // ...
}
```

**说明**: `sanitize_scope_segment` (行 403-418) 已替换非字母数字字符为 `_`，但不能防止 `..` 攻击（如果 `tenant_id` 包含 `..` 会被转换为 `__`，但逻辑正确）。

---

### 5. ⚠️ 警告 - `resolve_allowed_tools_for_scope` 中数据库错误被忽略

**问题代码** (行 437-455):
```rust
if let Ok(store) = SaasSqliteStore::new(db_path) {
    if let Ok(resolved) = resolve_effective_tool_allowlist(...) {
        return resolved;
    }
}
default_tools  // 静默 fallback
```

**触发场景**: 数据库查询失败时静默使用默认工具，可能导致工具权限超出预期。

**修复方案**: 记录错误日志：
```rust
if let Ok(store) = SaasSqliteStore::new(db_path) {
    match resolve_effective_tool_allowlist(...) {
        Ok(resolved) => return resolved,
        Err(e) => tracing::warn!("Failed to resolve tool allowlist: {}", e),
    }
}
```

---

### 6. 💡 建议 - `process_message` 中事件通道处理逻辑过长

**问题代码** (行 122-187): 66 行的 match 表达式

**修复方案**: 提取为独立函数：
```rust
fn react_event_to_message(
    event: ReactEvent,
    session_id: String,
    request_id: String,
) -> Option<GatewayMessage> {
    match event {
        ReactEvent::Thinking => None,
        // ...
    }
}
```

---

### 7. 💡 建议 - `allowed_tools_hint` 函数可移至工具策略模块

**问题代码** (行 24-33):
```rust
fn allowed_tools_hint(allowed_tools: &[String]) -> Option<String> {
    if allowed_tools.is_empty() {
        None
    } else {
        Some(format!(
            "For this conversation, you may use only these tools: {}. Do not call any other tool.",
            allowed_tools.join(", ")
        ))
    }
}
```

**说明**: 该函数与工具策略相关，建议移至 `tool_policy` 模块。

---

### 8. 💡 建议 - `RuntimeConfig` 中 `max_concurrent` 字段未使用

**问题代码** (行 36-54):
```rust
pub struct RuntimeConfig {
    // ...
    pub max_concurrent: usize,  // 行 45
    // ...
}
```

**说明**: 该字段在 `AgentRuntime` 中未使用，应移除或添加使用说明。

---

### 9. 💡 建议 - `process_message` 中硬编码的 `request_id` 生成

**问题代码** (行 107):
```rust
let request_id = uuid::Uuid::new_v4().to_string();
```

**说明**: 合理，但如果需要可追踪的请求 ID，可考虑使用 `ulid` 或带前缀的 ID。

---

## 设计确认（非问题）

1. **ReAct 循环与直接路由双模式** - 根据意图选择执行路径，合理。
2. **工具策略解析** - 基于租户/团队/用户作用域解析工具权限，合理。
3. **工作空间隔离** - `scoped_runtime_workspace` 为不同租户创建独立工作空间，合理。
4. **流式响应** - 通过 channel 实现流式事件通知，合理。

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `Arc` 使用 | ✅ 合理 |
| 错误处理 | 静默 fallback | ⚠️ 部分场景需改进 |
| Async | `tokio::spawn` 使用 | ✅ 合理 |
| 安全性 | 路径遍历防护 | ✅ `sanitize_scope_segment` 防护 |
| 安全性 | 错误信息泄露 | ❌ 需脱敏 |

---

## 问题统计

| 等级 | 数量 |
|------|------|
| ❌ 严重 | 2 |
| ⚠️ 警告 | 3 |
| 💡 建议 | 4 |
