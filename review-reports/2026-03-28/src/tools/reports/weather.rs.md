# Rust 代码审查报告：weather.rs

## 业务场景和职责
- 获取实时天气工具
- 使用 wttr.in API
- 支持今天/明天预报

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
**修复方案**: 使用 expect 提供更好错误信息：
```rust
.build()
.expect("Failed to create HTTP client")
```

### 2. **API URL 硬编码**
**行号**: 127
```rust
let mut url = reqwest::Url::parse("https://wttr.in/").map_err(|e| e.to_string())?;
```
**设计确认**: wttr.in 是稳定的免费 API

### 3. **location 推断逻辑复杂且可能误判**
**行号**: 36-61
```rust
fn infer_location_from_text(text: &str) -> Option<String> {
    let mut location = text
        .replace("今天天气", "")
        .replace("明天天气", "")
        // ... 多个替换
}
```
**触发场景**: 复杂查询可能误判，如 "北京今天天气如何" 可能变成 "京"
**修复方案**: 使用 NLP 或 LLM 提取位置（但增加复杂度）

### 4. **sanitize_location 可能过度修剪**
**行号**: 64-87
```rust
fn sanitize_location(location: &str) -> String {
    location
        .trim()
        .trim_matches(|c: char| {
            c.is_whitespace()
                || matches!(c, '？' | '?' | '。' | '.' | ...)
        })
```
**触发场景**: 地名中包含标点（如 "St. Louis"）可能被误剪
**修复方案**: 更精确的修剪逻辑：
```rust
// 只修剪首尾标点，不修剪中间
```

### 5. **测试覆盖部分功能**
**行号**: 208-223
```rust
#[test]
fn test_sanitize_location_trims_punctuation() { ... }
#[test]
fn test_infer_location_from_text_strips_weather_noise() { ... }
```
**设计确认**: 测试覆盖关键功能，可以接受

---

## 设计确认（非问题）
- 使用 wttr.in 是合理选择（免费、无需 API key）
- 支持中英文是好的设计
- 简要天气摘要实用

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ✓ 正确传播错误 |
| Async | ✓ 异步 HTTP 请求 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：1
- 💡 建议：2
