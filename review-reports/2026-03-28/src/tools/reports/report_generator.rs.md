# Rust 代码审查报告：report_generator.rs

## 业务场景和职责
- 生成结构化研究报告工具
- 支持 Markdown 和 JSON 两种格式
- 使用 LLM 从研究结果生成报告

---

## 问题

### 1. **format 参数比较硬编码**
**行号**: 52
```rust
let prompt = if format == "json" {
```
**触发场景**: 如果传入 "JSON" 或 "Json" 等不同大小写，会被当作 markdown
**修复方案**: 使用大小写不敏感比较：
```rust
let prompt = if format.eq_ignore_ascii_case("json") {
```

### 2. **LLM 响应无错误恢复**
**行号**: 111-115
```rust
let response = self
    .llm
    .complete(&messages)
    .await
    .map_err(|e| format!("LLM error: {}", e))?;
```
**触发场景**: LLM 响应可能包含额外文本，直接返回可能不是纯报告
**修复方案**: 对于 JSON 格式，应尝试解析并重新序列化确保格式正确：
```rust
if format.eq_ignore_ascii_case("json") {
    // 验证 JSON 格式
    let value: Value = serde_json::from_str(&response)
        .map_err(|e| format!("Invalid JSON response: {}", e))?;
    return Ok(serde_json::to_string_pretty(&value)?);
}
```

### 3. **prompt 模板中格式说明不够严格**
**行号**: 53-72, 74-107
```rust
Output JSON structure:
{
    "title": "report title",
    ...
}
```
**触发场景**: LLM 可能输出额外文本或不遵循格式
**修复方案**: 添加 system prompt 明确只输出 JSON：
```rust
// 添加 "Output ONLY the JSON, no additional text."
```

### 4. **错误类型转换不够精确**
**行号**: 49
```rust
return Err("Missing topic or findings".to_string());
```
**修复方案**: 提供更具体的错误信息：
```rust
return Err(format!(
    "Missing required parameter: {}",
    if topic.is_empty() { "topic" } else { "findings" }
));
```

---

## 设计确认（非问题）
- 支持多种格式是好的设计
- prompt 模板结构清晰
- 使用 LLM 生成报告是合理方案

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 使用 Arc |
| 错误处理 | ⚠️ 部分错误信息可改进 |
| Async | ✓ 使用 async LLM client |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：2
