# Rust 代码审查报告 - spoke.rs

## 业务场景和职责

**文件路径**: `src/gateway/spoke.rs`

**职责**: Spoke（辐条/端点）抽象层，定义通讯端点和能力端点的通用接口。

**关键设计**:
- `SpokeAdapter` trait 作为统一接口
- 通讯端点（CommunicationSpoke）：Web/TUI/WhatsApp/Lark 等
- 能力端点（CapabilitySpoke）：Skills/Tools/API 插件/脚本
- 提供 4 个具体实现：`WebSocketSpoke`、`HttpSpoke`、`TuiSpoke`、`SkillSpoke`、`ApiPluginSpoke`

---

## 问题列表

### 1. ❌ 严重 - `WebSocketSpoke::start` 未实现实际功能

**问题代码** (行 313-318):
```rust
async fn start(
    &self,
    _message_tx: mpsc::UnboundedSender<(ClientInfo, GatewayMessage)>,
) -> Result<(), String> {
    Ok(())  // 空实现
}
```

**触发场景**: 调用 `start` 时不会实际启动 WebSocket 服务器，导致连接无法建立。

**修复方案**: 实现完整的 WebSocket 服务器逻辑，参考 `hub.rs` 中的 `handle_connection`。

---

### 2. ❌ 严重 - `WebSocketSpoke` 缺少连接管理逻辑

**问题代码** (行 285-300):
```rust
pub struct WebSocketSpoke {
    bind_addr: String,
    connections: Arc<tokio::sync::RwLock<std::collections::HashMap<String, WebSocketConnection>>>,
}
```

**触发场景**: 没有 `accept` 方法添加连接，`connections` 永远为空，`send` 方法永远失败。

**修复方案**: 添加连接注册方法：
```rust
pub async fn add_connection(&self, client_id: String, tx: mpsc::UnboundedSender<String>) {
    self.connections.write().await.insert(client_id, WebSocketConnection { tx });
}

pub async fn remove_connection(&self, client_id: &str) {
    self.connections.write().await.remove(client_id);
}
```

---

### 3. ❌ 严重 - `TuiSpoke::send` 中 `client_id` 参数未使用

**问题代码** (行 424-429):
```rust
async fn send(&self, _client_id: &str, message: GatewayMessage) -> Result<(), String> {
    if let Some(tx) = self.tx.read().await.as_ref() {
        tx.send(message)
            .map_err(|e| format!("TUI send error: {}", e))?;
    }
    Ok(())
}
```

**触发场景**: TUI 通常是单实例，但接口设计支持多客户端，实际只发送到单一 tx。

**说明**: 如果 TUI 确实只需要单播，应修改 trait 或添加注释说明。

---

### 4. ⚠️ 警告 - `SkillSpoke::execute` 中硬编码 `python3`

**问题代码** (行 182-187):
```rust
let output = tokio::process::Command::new("python3")
    .arg(script_path)
    .arg(&input_str)
    .output()
    .await
```

**触发场景**:
- Windows 系统可能只有 `python` 没有 `python3`
- 用户可能使用虚拟环境或自定义 Python 路径

**修复方案**:
```rust
// 方案 1: 支持配置
let python_cmd = std::env::var("PYTHON_CMD").unwrap_or_else(|_| "python3".to_string());
let output = tokio::process::Command::new(python_cmd)
    // ...

// 方案 2: 支持多种解释器
let output = tokio::process::Command::new("python3")
    .arg(script_path)
    // ...
    .or_else(|_| tokio::process::Command::new("python")
        // ...
    )
```

---

### 5. ⚠️ 警告 - `SkillSpoke::execute` 中 `unwrap_or_else` 掩盖 JSON 解析错误

**问题代码** (行 190-191):
```rust
let result: serde_json::Value = serde_json::from_str(&stdout)
    .unwrap_or_else(|_| serde_json::Value::String(stdout.to_string()));
```

**触发场景**: 脚本输出非 JSON 格式时，错误被静默转换为 String，调用者无法感知解析失败。

**修复方案**:
```rust
let result: serde_json::Value = serde_json::from_str(&stdout)
    .map_err(|e| format!("Script output is not valid JSON: {}", e))?;
```

---

### 6. ⚠️ 警告 - `ApiPluginSpoke::execute` 中错误信息泄露 HTTP 状态码细节

**问题代码** (行 268):
```rust
.map_err(|e| format!("API request failed with status {}: {}",
    e.status().unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR), e))?
```

**触发场景**: 在生产环境中可能泄露内部服务信息。

**修复方案**: 使用通用错误消息，详细日志记录到 tracing：
```rust
.map_err(|e| {
    tracing::error!("API request failed: status={}, error={}",
        e.status().unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR), e);
    "API request failed".to_string()
})?
```

---

### 7. 💡 建议 - `CommunicationSpokeType` 和 `SpokeType` 重复

**问题代码** (行 38-58 vs message.rs 的 SpokeType):
```rust
pub enum CommunicationSpokeType {
    Web, Tui, Telegram, Slack, WhatsApp, Discord, Lark, Api,
}
```

**说明**: `message::SpokeType` 已有类似定义（缺少 Telegram/Slack/Discord），建议统一。

---

### 8. 💡 建议 - 缺少 `TelegramSpoke`、`SlackSpoke`、`DiscordSpoke` 实现

**问题代码** (行 46-53 定义了枚举但无实现):
```rust
Telegram, Slack, Discord,
```

**说明**: 如果这些是预留功能，建议添加 `#[allow(dead_code)]` 或 `todo!()` 说明。

---

### 9. 💡 建议 - `CapabilitySpoke` trait 缺少超时控制

**问题代码** (行 98-121):
```rust
async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, String>;
```

**说明**: 长时间运行的能力可能阻塞，建议添加超时参数或配置：
```rust
async fn execute_with_timeout(&self, input: serde_json::Value, timeout: Duration) -> Result<serde_json::Value, String>;
```

---

## 设计确认（非问题）

1. **Trait 抽象层设计** - `SpokeAdapter` 统一接口，合理。
2. **构建者模式** - `ApiPluginSpoke::with_header` 使用构建者模式，合理。
3. **默认方法实现** - `CommunicationSpoke` trait 的默认方法合理。

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `Arc<RwLock<T>>` | ✅ 合理 |
| 错误处理 | `unwrap_or_else` 掩盖错误 | ⚠️ 需改进 |
| Async | 空实现 `start` | ❌ 需完善 |

---

## 问题统计

| 等级 | 数量 |
|------|------|
| ❌ 严重 | 3 |
| ⚠️ 警告 | 3 |
| 💡 建议 | 3 |
