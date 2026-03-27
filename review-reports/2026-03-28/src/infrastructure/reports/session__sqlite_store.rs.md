# Rust 代码审查报告

## 业务场景和职责

**文件**: `src/infrastructure/session/sqlite_store.rs`

**职责**: 实现基于 SQLite 的会话持久化存储，作为 `SessionStore` trait 的具体实现，支持会话的 CRUD 操作。

**关键依赖和设计权衡**:
- 使用 `rusqlite`（同步 SQLite）+ `tokio::sync::Mutex` 实现异步兼容
- 使用 `Arc<Mutex<Connection>>` 实现线程安全的连接共享
- 使用 `async-trait` 实现异步 trait 方法
- 设计权衡：未使用 `sqlx` 异步 SQLite，而是用同步 rusqlite + Mutex，简单但并发性能受限

---

## 问题

### ❌ 严重问题（1 个）

#### 1. **潜在死锁风险**（第 87-92 行、101-107 行等）

**问题代码**:
```rust
async fn create(&self, config: SessionConfig) -> Result<SessionId, String> {
    let conn = self.conn.lock().await;  // 第 87 行
    // ...
    conn.execute(...)  // 第 92 行
```

**触发场景**:
当多个异步任务同时调用 `create`/`get`/`update`/`delete` 等方法时，由于所有操作共享同一个 `Mutex<Connection>`，会串行执行。如果在持有锁期间执行其他异步操作（如网络 IO），可能导致死锁或性能瓶颈。

**问题说明**:
虽然当前代码在持有锁期间只执行同步的 SQLite 操作，风险较低。但如果未来扩展时在 `conn.lock().await` 和 `conn.execute()` 之间插入 `.await` 点，可能导致其他任务无法获取锁。

**修复方案**:
```rust
// 方案 1：使用 tokio::spawn_blocking 将同步操作放到线程池
async fn create(&self, config: SessionConfig) -> Result<SessionId, String> {
    let conn = self.conn.clone();
    tokio::task::spawn_blocking(move || {
        let conn = futures::executor::block_on(conn.lock());
        // 执行操作
    }).await
}

// 方案 2（推荐）：迁移到 sqlx 异步 SQLite
```

---

### ⚠️ 警告（3 个）

#### 1. **字符串错误处理丢失类型信息**（第 96、105、118、137、173、184、193、197 行）

**问题代码**:
```rust
.map_err(|e| format!("Failed to create session: {}", e))?;  // 第 96 行
.map_err(|e| format!("Failed to prepare statement: {}", e))?;  // 第 105 行
```

**触发场景**:
所有错误被转换为 `String`，调用方无法区分错误类型（如数据库锁定、约束违反、IO 错误等），无法做针对性的错误处理。

**修复方案**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

async fn create(&self, config: SessionConfig) -> Result<SessionId, SessionStoreError> {
    // 直接使用 ? 传播 rusqlite::Error
}
```

#### 2. **SessionStatus::Error 状态信息丢失**（第 77、147 行）

**问题代码**:
```rust
"error" => SessionStatus::Error("Unknown error".to_string()),  // 第 77 行
```

**触发场景**:
当数据库存储的状态为 `"error"` 时，原有的错误消息已丢失，只能恢复为 `"Unknown error"`。这意味着 `update` 方法保存的错误信息在读取时无法还原。

**问题说明**:
表结构中没有存储错误消息的字段，导致 `SessionStatus::Error(String)` 的错误消息无法持久化。

**修复方案**:
```rust
// 方案 1：在表结构中添加 error_message 字段
conn.execute(
    "ALTER TABLE sessions ADD COLUMN error_message TEXT",
    [],
)?;

// 方案 2：将错误消息序列化到 system_prompt 或额外字段
```

#### 3. **created_at 字段在 SELECT 中未使用**（第 31、192 行）

**问题代码**:
```sql
-- 第 31 行：表结构定义了 created_at
created_at INTEGER DEFAULT (strftime('%s', 'now')),

-- 第 192 行：list 方法使用 created_at 排序
"SELECT id FROM sessions ORDER BY created_at DESC"
```

**触发场景**:
`CREATE TABLE` 和 `in_memory()` 中的表结构不一致。`in_memory()` 创建的表没有显式定义 `created_at` 的默认值，虽然 SQLite 会允许 NULL，但排序行为可能不一致。

**修复方案**:
```rust
// 确保两个地方的表结构完全一致
pub fn in_memory() -> SqliteResult<Self> {
    let conn = Connection::open(":memory:")?;
    conn.execute(
        "CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            max_turns INTEGER NOT NULL,
            system_prompt TEXT NOT NULL,
            status TEXT NOT NULL,
            message_count INTEGER DEFAULT 0,
            created_at INTEGER DEFAULT (strftime('%s', 'now')),
            updated_at INTEGER DEFAULT (strftime('%s', 'now'))
        )",
        [],
    )?;
    // 当前代码已一致，此问题实际不存在，但建议提取为公共函数
}
```

---

### 💡 建议（5 个）

#### 1. **提取公共的表结构定义**

**问题代码**: 第 25-34 行和第 46-55 行重复定义了相同的表结构。

**建议**:
```rust
const SESSION_TABLE_SCHEMA: &str = r#"
    CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        max_turns INTEGER NOT NULL,
        system_prompt TEXT NOT NULL,
        status TEXT NOT NULL,
        message_count INTEGER DEFAULT 0,
        created_at INTEGER DEFAULT (strftime('%s', 'now')),
        updated_at INTEGER DEFAULT (strftime('%s', 'now'))
    )
"#;

fn init_schema(conn: &Connection) -> SqliteResult<()> {
    conn.execute(SESSION_TABLE_SCHEMA, [])?;
    Ok(())
}
```

#### 2. **使用 rusqlite 内建时间函数**

**问题代码**: 第 31-32 行使用 `strftime('%s', 'now')` 计算时间戳。

**建议**:
虽然当前写法正确，但 Rusqlite 支持直接绑定 Rust 的时间戳：
```rust
use chrono::Utc;
let now = Utc::now().timestamp();
conn.execute("INSERT ... VALUES (?, ?, ?, ?, ?, ?)", params![..., now, now])?;
```

#### 3. **prepared statement 缓存**

**问题代码**: 第 103-105 行每次 `get` 调用都 `prepare` 新语句。

**建议**:
```rust
pub struct SqliteSessionStore {
    conn: Arc<Mutex<Connection>>,
    // 预编译语句
    stmt_get: Arc<Mutex<Statement<'static>>>,
    // ...
}
```
或者使用 `rusqlite::CachedStatement`。

#### 4. **事务支持**

**问题代码**: `update` 方法（第 155-175 行）执行单条 SQL，但如果未来需要批量更新，应支持事务。

**建议**:
```rust
pub async fn begin_transaction(&self) -> Result<SqliteTransaction<'_>, String> {
    let conn = self.conn.lock().await;
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;
    // ...
}
```

#### 5. **测试覆盖不完整**

**问题代码**: 第 214-239 行的测试只覆盖了基本 CRUD。

**建议**:
添加以下测试用例：
- 并发读写测试
- `get_state` 返回不同状态值的测试
- `SessionStatus::Error` 的持久化测试
- 空数据库的 `list` 测试

---

## 设计确认（非问题）

1. **`Arc<Mutex<Connection>>` 设计合理**：在单文件 SQLite 场景下，连接本身是线程安全的，使用 Mutex 保证串行访问是正确的选择。

2. **`row_to_session` 私有方法**：作为内部转换逻辑，不暴露给外部是合理的封装。

3. **使用 `String` 作为错误类型**：在应用层简单场景下可接受，虽然丢失了类型信息但便于快速开发。

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `.clone()` 使用合理（第 88 行 `config.id.0.clone()`） | ✅ |
| 所有权 | `Arc<Mutex<T>>` 使用正确 | ✅ |
| 错误处理 | `unwrap()` 仅用于测试代码 | ✅ |
| 错误处理 | 生产代码使用 `?` + `map_err` | ⚠️ 建议改进为强类型错误 |
| 错误处理 | 无 `let _ =` 忽略错误 | ✅ |
| Async | 使用 `async-trait` 正确 | ✅ |
| Async | 无阻塞调用风险（持有锁期间无 `.await`） | ✅ |
| Async | 无 `spawn_blocking` 需求（同步 SQLite 可接受） | ⚠️ 高并发场景需考虑 |

---

## 总结

| 等级 | 数量 |
|------|------|
| ❌ 严重 | 1 |
| ⚠️ 警告 | 3 |
| 💡 建议 | 5 |

**核心问题**:
1. 错误类型信息丢失（所有错误转为 `String`）
2. `SessionStatus::Error` 的错误消息无法持久化
3. 高并发场景下 `Mutex<Connection>` 可能成为瓶颈

**推荐优先级**:
1. 定义强类型错误枚举（影响 API 设计）
2. 添加 `error_message` 字段支持错误消息持久化
3. 评估是否需要迁移到 `sqlx` 异步 SQLite
