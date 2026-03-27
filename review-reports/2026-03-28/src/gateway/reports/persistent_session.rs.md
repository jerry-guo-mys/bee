# Rust 代码审查报告 - persistent_session.rs

## 业务场景和职责

**文件路径**: `src/gateway/persistent_session.rs`

**职责**: 持久化会话管理，使用 SQLite 存储会话状态，支持跨重启恢复。

**关键设计**:
- 与内存版 `SessionManager` 接口相似
- 会话元数据和消息历史持久化到 SQLite
- 活跃会话缓存在内存中
- 服务重启后可从数据库恢复会话
- 使用 `async-sqlite` feature 控制编译

---

## 问题列表

### 1. ❌ 严重 - `restore_sessions` 中时间戳计算可能在跨时区场景下出错

**问题代码** (行 165-180):
```rust
// 从 RFC3339 时间戳计算 Instant
let now = chrono::Utc::now();
let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
    .map(|dt| {
        let elapsed = now.signed_duration_since(dt.to_utc());
        std::time::Instant::now()
            - std::time::Duration::from_millis(elapsed.num_milliseconds().max(0) as u64)
    })
    .unwrap_or_else(|_| std::time::Instant::now());
```

**触发场景**:
- 程序在不同时区的机器上运行
- 系统时钟在保存和恢复之间被调整
- `Instant::now() - Duration` 可能溢出（如果 elapsed 为负）

**修复方案**:
```rust
let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
    .map(|dt| {
        let elapsed_ms = now.signed_duration_since(dt.to_utc()).num_milliseconds();
        std::time::Instant::now()
            - std::time::Duration::from_millis(elapsed_ms.max(0) as u64)
    })
    .unwrap_or_else(|_| {
        tracing::warn!("Failed to parse created_at: {}", created_at_str);
        std::time::Instant::now()
    });
```

---

### 2. ❌ 严重 - `save_message` 中事务可能长时间持有锁

**问题代码** (行 254-282):
```rust
async fn save_message(
    &self,
    session_id: &str,
    message: &crate::memory::Message,
) -> Result<(), sqlx::Error> {
    let mut tx = self.pool.begin().await?;  // 开始事务

    // ... 执行查询 ...

    tx.commit().await?;  // 提交事务
    Ok(())
}
```

**触发场景**: 高并发写入时，事务锁可能导致其他写入等待。

**说明**: SQLite 使用文件级锁，高并发写入可能成为瓶颈。考虑：
1. 批量写入
2. WAL 模式
3. 异步队列缓冲写入

---

### 3. ⚠️ 警告 - `ensure_session_scope_columns` 中 ALTER TABLE 错误处理不完整

**问题代码** (行 115-130):
```rust
async fn ensure_session_scope_columns(&self) -> Result<(), sqlx::Error> {
    for column in [
        "tenant_id TEXT",
        "organization_id TEXT",
        "team_id TEXT",
        "agent_instance_id TEXT",
    ] {
        let statement = format!("ALTER TABLE gateway_sessions ADD COLUMN {}", column);
        match sqlx::query(&statement).execute(&self.pool).await {
            Ok(_) => {}
            Err(sqlx::Error::Database(db_err))
                if db_err.message().contains("duplicate column name") => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}
```

**触发场景**: 如果错误不是 "duplicate column name"（如权限错误），会被忽略还是返回？

**修复方案**: 代码逻辑正确（仅忽略 duplicate column name），但建议记录警告：
```rust
Err(sqlx::Error::Database(db_err))
    if db_err.message().contains("duplicate column name") => {
        tracing::debug!("Column {} already exists", column);
    }
```

---

### 4. ⚠️ 警告 - `get_context` 中 `clone()` 可能开销较大

**问题代码** (行 405-411):
```rust
pub async fn get_context(&self, session_id: &str) -> Option<ContextManager> {
    self.sessions
        .read()
        .await
        .get(session_id)
        .map(|s| s.context.clone())
}
```

**说明**: 同 `session_store.rs` 的问题 2，`ContextManager` 较大时克隆开销大。

---

### 5. ⚠️ 警告 - `cleanup_expired` 未同步删除数据库中的会话

**问题代码** (行 376-391):
```rust
pub async fn cleanup_expired(&self) -> usize {
    let mut sessions = self.sessions.write().await;
    let mut user_sessions = self.user_sessions.write().await;

    let expired: Vec<_> = sessions
        .iter()
        .filter(|(_, s)| s.is_expired(self.session_timeout))
        .map(|(id, s)| (id.clone(), s.user_id.clone()))
        .collect();

    for (session_id, user_id) in &expired {
        sessions.remove(session_id);
        user_sessions.remove(user_id);
    }

    expired.len()
}
```

**触发场景**: 内存中清理了会话，但数据库中仍保留，下次重启时会话会恢复。

**修复方案**: 同时删除数据库记录：
```rust
// 删除数据库记录
for (session_id, _) in &expired {
    sqlx::query("DELETE FROM gateway_sessions WHERE id = ?")
        .bind(session_id)
        .execute(&self.pool)
        .await?;
}
```

---

### 6. 💡 建议 - `PersistentSessionManager` 缺少关闭时的数据刷新

**问题代码** (行 436-438):
```rust
pub async fn close(&self) {
    self.pool.close().await;
}
```

**说明**: 关闭连接池前，应确保所有缓存的会话数据已刷新到数据库。

---

### 7. 💡 建议 - `load_messages` 中缺少 `limit` 参数

**问题代码** (行 219-247):
```rust
async fn load_messages(&self, session_id: &str) -> Result<Vec<crate::memory::Message>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT role, content FROM gateway_messages WHERE session_id = ? ORDER BY id ASC"
    )
    .bind(session_id)
    .fetch_all(&self.pool)
    .await?;
```

**触发场景**: 长对话会话可能有数千条消息，全部加载影响性能。

**修复方案**: 添加 limit 参数：
```rust
async fn load_messages(&self, session_id: &str, limit: usize) -> Result<Vec<crate::memory::Message>, sqlx::Error> {
    sqlx::query(
        "SELECT role, content FROM gateway_messages WHERE session_id = ? ORDER BY id DESC LIMIT ?"
    )
    // ...
}
```

---

### 8. 💡 建议 - 测试依赖 `tempfile` crate 但未在 gateway 模块中声明

**问题代码** (行 444):
```rust
use tempfile::TempDir;
```

**说明**: `tempfile` 在 `dev-dependencies` 中声明，合理。但需确保测试能正确运行。

---

## 设计确认（非问题）

1. **内存 + 持久化混合存储** - 活跃会话缓存 + 全量持久化，合理。
2. **级联删除** - `ON DELETE CASCADE` 确保会话删除时消息自动删除，合理。
3. **索引优化** - 为 `user_id` 和 `session_id` 添加索引，合理。
4. **事务完整性** - `save_message` 使用事务保证原子性，合理。

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `.clone()` 使用 | ⚠️ `ContextManager` 克隆开销 |
| 错误处理 | 模式匹配 | ✅ 合理 |
| Async | 数据库事务 | ⚠️ 高并发可能阻塞 |
| 数据一致性 | 内存/DB 同步 | ❌ `cleanup_expired` 未同步 DB |

---

## 问题统计

| 等级 | 数量 |
|------|------|
| ❌ 严重 | 2 |
| ⚠️ 警告 | 3 |
| 💡 建议 | 3 |
