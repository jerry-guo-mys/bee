# Rust 代码审查报告：send.rs

## 业务场景和职责
- assistant 向另一个 assistant 发送消息（Phase 2）
- 创建/复用 P2P 群，写入消息到文件
- 发送方来自 task_local（process_message_stream 设置）

---

## 问题

### 1. **task_local 访问可能 panic**
**行号**: 167-170
```rust
let from = CURRENT_ASSISTANT_ID
    .try_with(|s| s.clone())
    .unwrap_or(None)
    .unwrap_or_else(|| "default".to_string());
```
**触发场景**: task_local 未初始化时使用 try_with 是安全的，但代码逻辑可以简化
**修复方案**: 当前使用 try_with 是正确的，但建议添加注释说明：
```rust
// task_local 可能未初始化（非 ReAct 循环调用场景），回退到 "default"
let from = CURRENT_ASSISTANT_ID
    .try_with(|s| s.clone())
    .unwrap_or(None)
    .unwrap_or_else(|| "default".to_string());
```

### 2. **文件 IO 操作未处理错误**
**行号**: 49-51, 73-75
```rust
if let Ok(json) = serde_json::to_string_pretty(groups) {
    let _ = std::fs::write(&self.groups_path, json);
}
```
**触发场景**: 当磁盘写满或权限不足时，错误被静默忽略
**修复方案**: 使用 tracing 记录错误：
```rust
match serde_json::to_string_pretty(groups) {
    Ok(json) => {
        if let Err(e) = std::fs::write(&self.groups_path, json) {
            tracing::error!("Failed to save groups: {}", e);
        }
    }
    Err(e) => {
        tracing::error!("Failed to serialize groups: {}", e);
    }
}
```

### 3. **unwrap_or 在 load_group_messages 中**
**行号**: 60
```rust
let snap: GroupSnapshot = serde_json::from_str(&data).unwrap_or(GroupSnapshot {
```
**触发场景**: 当 JSON 格式错误时，静默使用默认值可能掩盖问题
**修复方案**: 添加日志记录：
```rust
let snap: GroupSnapshot = match serde_json::from_str(&data) {
    Ok(s) => s,
    Err(e) => {
        tracing::warn!("Failed to parse group snapshot: {}", e);
        GroupSnapshot { messages: vec![], max_turns: 20 }
    }
};
```

### 4. **chrono 依赖未在文件中声明**
**行号**: 186, 117
```rust
chrono::Utc::now().to_rfc3339()
```
**触发场景**: 如果 Cargo.toml 中 chrono 依赖被移除，编译失败
**修复方案**: 确保 Cargo.toml 中有 chrono 依赖，或使用 std::time::SystemTime

---

## 设计确认（非问题）
- P2P 群 ID 按字母序排列保证唯一性是好的设计
- task_local 传递 sender ID 是合理的架构
- 消息格式包含发送者信息便于追踪

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ 3 处错误被忽略 |
| Async | ✓ 简单文件 IO |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：3
- 💡 建议：1
