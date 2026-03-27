# Rust 代码审查报告：news.rs

## 业务场景和职责
- 获取新闻头条工具
- 使用 Google News RSS
- 支持查询和限制数量

---

## 问题

### 1. **Client 构建使用 unwrap_or_default**
**行号**: 19-24
```rust
client: Client::builder()
    .timeout(std::time::Duration::from_secs(timeout_secs))
    .user_agent("Bee/1.0")
    .build()
    .unwrap_or_default(),
```
**触发场景**: Client 构建几乎不会失败，但 unwrap_or_default 可能掩盖配置错误
**修复方案**: 使用 expect 提供更好错误信息：
```rust
.build()
.expect("Failed to create HTTP client")
```

### 2. **RSS 解析使用手动字符串操作**
**行号**: 46-91
```rust
fn strip_cdata(text: &str) -> String {
    text.replace("<![CDATA[", "")
        .replace("]]>", "")
        // ...
}
fn extract_tag(block: &str, tag: &str) -> Option<String> { ... }
```
**触发场景**: 手动解析 XML 脆弱，RSS 格式变化可能导致解析失败
**修复方案**: 使用 RSS 解析库如 `rss` crate：
```rust
// 使用 rss::Channel 解析
```

### 3. **硬编码的 Google News URL**
**行号**: 30-41
```rust
let mut url = Url::parse("https://news.google.com/rss/search")
```
**触发场景**: 如果 Google News URL 变化，需要代码修改
**修复方案**: 移到配置中（可选）

### 4. **limit 限制范围硬编码**
**行号**: 158
```rust
.clamp(1, 10) as usize
```
**修复方案**: 定义为常量：
```rust
const MAX_NEWS_LIMIT: u64 = 10;
const MIN_NEWS_LIMIT: u64 = 1;
```

---

## 设计确认（非问题）
- 使用 RSS 是简单有效的方案
- 支持查询参数灵活
- 输出结构化 JSON 便于消费

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ 部分 unwrap |
| Async | ✓ 异步 HTTP 请求 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：2
