# Rust 代码审查报告

## 业务场景和职责

**文件**: `src/infrastructure/session/mod.rs` + `src/infrastructure/session/sqlite_store.rs`

**职责**: 会话存储基础设施层，提供会话的持久化存储能力

**关键依赖**:
- `rusqlite` (0.32): SQLite 数据库操作
- `tokio::sync::Mutex`: 异步互斥锁
- `async-trait`: 异步 trait 支持

**设计意图**: 使用 `Arc<Mutex<Connection>>` 实现线程安全的 SQLite 连接共享，支持多任务并发访问

---

## 问题

### ❌ 严重 #1: 同步 SQLite 连接在异步环境阻塞

**问题代码** (sqlite_store.rs:14-16, 20-40):
```rust
pub struct SqliteSessionStore {
    conn: Arc<Mutex<Connection>>,  // 第 15 行
}

pub fn new(path: impl AsRef<Path>) -> SqliteResult<Self> {
    let conn = Connection::open(path.as_ref())?;  // 第 21 行：同步阻塞调用
    ...
}
```

**触发场景**:
- 高并发场景下，多个异步任务同时调用 `store.create/get/update/delete`
- SQLite 同步 I/O 会阻塞整个 tokio 运行时线程

**修复方案**:
```rust
// 方案 1: 使用 tokio::task::spawn_blocking 包装同步操作
async fn create(&self, config: SessionConfig) -> Result<SessionId, String> {
    let conn = self.conn.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = conn.blocking_lock();
        // ... 数据库操作
    })
    .await
    .unwrap()
}

// 方案 2 (推荐): 使用 sqlx 异步 SQLite (已有依赖但未启用)
// 启用 feature: async-sqlite
use sqlx::SqlitePool;

pub struct SqliteSessionStore {
    pool: SqlitePool,
}
```

---

### ❌ 严重 #2: 错误状态信息丢失

**问题代码** (sqlite_store.rs:77, 147):
```rust
"error" => SessionStatus::Error("Unknown error".to_string()),  // 第 77 行
"error" => SessionStatus::Error("Unknown error".to_string()),  // 第 147 行
```

**触发场景**:
- 数据库存储的 session 状态为 "error" 时
- 原始错误信息在序列化到数据库时丢失

**修复方案**:
```rust
// 方案 1: 数据库表增加 error_message 字段
// CREATE TABLE sessions (..., error_message TEXT, ...)

// 方案 2: 使用统一错误消息
"error" => SessionStatus::Error("Session error (details not persisted)".to_string()),

// 方案 3: 不在数据库存储错误详情，仅在内存中标记
"error" => SessionStatus::Error("See logs for details".to_string()),
```

---

### ⚠️ 警告 #1: `Arc<Mutex<T>>` 克隆连接而非共享锁

**问题代码** (sqlite_store.rs:13, 37-39):
```rust
#[derive(Clone)]  // 第 13 行：派生 Clone
pub struct SqliteSessionStore {
    conn: Arc<Mutex<Connection>>,
}

Ok(Self {
    conn: Arc::new(Mutex::new(conn)),  // 第 37-39 行
})
```

**触发场景**:
- `SqliteSessionStore` 被克隆时（如在多个任务间传递）
- 当前设计是正确的（`Arc` 保证共享），但需确认意图

**影响**: 当前设计正确，`Arc::clone` 不会复制底层数据。建议显式实现 `Clone` 以明确意图：

```rust
impl Clone for SqliteSessionStore {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}
```

---

### ⚠️ 警告 #2: SQL 注入风险（当前安全但需警惕）

**问题代码** (sqlite_store.rs:92-96, 104-118):
```rust
conn.execute(
    "INSERT INTO sessions (id, max_turns, system_prompt, status, message_count) VALUES (?1, ?2, ?3, ?4, ?5)",
    params![id, max_turns, system_prompt, "idle", 0],  // 正确使用参数化查询
)
```

**触发场景**:
- 当前代码使用参数化查询，安全
- 如果未来拼接 SQL 字符串会有风险

**建议**: 保持当前参数化查询风格，避免字符串拼接 SQL

---

### ⚠️ 警告 #3: `created_at` 和 `updated_at` 未读取

**问题代码** (sqlite_store.rs:25-33, 104-105):
```sql
-- 数据库表定义了 created_at, updated_at
created_at INTEGER DEFAULT (strftime('%s', 'now')),
updated_at INTEGER DEFAULT (strftime('%s', 'now'))
```

```rust
// 但查询时未读取这两个字段
SELECT id, max_turns, system_prompt, message_count, status FROM sessions
```

**触发场景**:
- 需要会话创建/更新时间信息时
- 日志分析、审计、调试场景

**修复方案**:
```rust
// 在 Session 或 SessionState 中添加时间戳字段
pub struct SessionState {
    pub id: SessionId,
    pub status: SessionStatus,
    pub message_count: usize,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// 查询时读取
SELECT id, max_turns, system_prompt, message_count, status, created_at, updated_at FROM sessions
```

---

### 💡 建议 #1: 表结构缺少索引

**问题代码** (sqlite_store.rs:24-35):
```sql
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,  -- 主键已有索引
    ...
)
```

**建议**:
- 当前 `id` 是 `PRIMARY KEY`，已有索引，`WHERE id = ?` 查询高效
- 如果未来需要按 `status` 或 `created_at` 查询，建议添加索引：
```sql
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_sessions_created_at ON sessions(created_at DESC);
```

---

### 💡 建议 #2: `row_to_session` 函数可见性

**问题代码** (sqlite_store.rs:64-81):
```rust
fn row_to_session(...) -> Session {  // 私有函数
    ...
}
```

**建议**: 如果后续有其他模块需要转换数据库行，可考虑设为 `pub(crate)`：
```rust
pub(crate) fn row_to_session(...) -> Session { ... }
```

---

### 💡 建议 #3: 测试覆盖不完整

**问题代码** (sqlite_store.rs:214-239):
```rust
#[tokio::test]
async fn test_session_store_crud() {
    // 仅测试基本 CRUD
    // 缺少：
    // - 并发访问测试
    // - 错误状态测试
    // - 边界条件测试（max_turns=0, system_prompt 为空等）
}
```

**建议添加测试**:
```rust
#[tokio::test]
async fn test_concurrent_access() {
    // 测试并发读写
}

#[tokio::test]
async fn test_error_status_persistence() {
    // 测试错误状态存储和恢复
}

#[tokio::test]
async fn test_list_ordering() {
    // 测试 list 按 created_at DESC 排序
}
```

---

### 💡 建议 #4: 魔法字符串

**问题代码** (sqlite_store.rs:94, 162-167):
```rust
params![id, max_turns, system_prompt, "idle", 0]  // "idle" 等状态字符串
```

**建议**: 定义常量避免魔法字符串：
```rust
mod db_constants {
    pub const STATUS_IDLE: &str = "idle";
    pub const STATUS_THINKING: &str = "thinking";
    pub const STATUS_EXECUTING: &str = "executing";
    pub const STATUS_RESPONDING: &str = "responding";
    pub const STATUS_ERROR: &str = "error";
}
```

---

## 设计确认（非问题）

### 合理的设计

1. **`Arc<Mutex<Connection>>` 模式**: 虽然 rusqlite 是同步的，但使用 `Mutex` 保证线程安全访问，设计合理
2. **内存数据库测试**: `in_memory()` 方法用于测试，设计良好
3. **参数化查询**: 所有 SQL 使用参数化查询，避免注入
4. **错误类型转换**: 使用 `String` 而非自定义错误类型，简化 API

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `Arc<Mutex<T>>` 正确使用 | ✅ |
| 错误处理 | `unwrap()` 仅在测试中使用 | ✅ |
| 错误处理 | `Result` 转换为 `String` 错误 | ⚠️ 建议自定义错误类型 |
| Async | 同步 SQLite 在异步环境 | ❌ 需要 `spawn_blocking` 或迁移到 sqlx |
| 并发 | `Mutex` 保护共享连接 | ✅ |
| SQL 安全 | 参数化查询 | ✅ |
| 测试 | 基本 CRUD 覆盖 | ✅ |
| 测试 | 并发/边界测试 | ❌ 缺失 |

---

## 总结

| 类别 | 数量 |
|------|------|
| ❌ 严重 | 2 |
| ⚠️ 警告 | 3 |
| 💡 建议 | 4 |

**优先修复**:
1. 同步 SQLite 阻塞问题（严重）
2. 错误状态信息丢失（严重）
3. 添加并发测试和边界测试
