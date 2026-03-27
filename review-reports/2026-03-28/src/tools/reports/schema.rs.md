# Rust 代码审查报告：schema.rs

## 业务场景和职责
- 生成工具调用的 JSON Schema，用于注入 system prompt
- 使用 schemars 库自动生成 schema，减少 LLM 输出格式错误

---

## 问题

### 1. **HashMap 值类型过于宽泛**
**行号**: 15
```rust
pub args: HashMap<String, String>,
```
**触发场景**: 工具参数可能包含非字符串类型（如数组、对象、数字）
**修复方案**: 使用 serde_json::Value 类型：
```rust
pub args: HashMap<String, serde_json::Value>,
```

### 2. **schema 生成失败时返回空字符串**
**行号**: 21
```rust
serde_json::to_string_pretty(&schema).unwrap_or_else(|_| String::new())
```
**触发场景**: 当 schema 无法序列化时，返回空字符串可能导致调用方困惑
**修复方案**: 记录错误日志并返回有意义的错误：
```rust
match serde_json::to_string_pretty(&schema) {
    Ok(s) => s,
    Err(e) => {
        tracing::error!("Failed to serialize tool call schema: {}", e);
        String::new()
    }
}
```

---

## 设计确认（非问题）
- 职责单一清晰
- 使用 schemars 自动生成 schema 是合理选择
- dead_code 允许合理（仅用于 schema 生成）

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ 1 处 unwrap_or_else |
| Async | ✓ 无异步 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：1
- 💡 建议：1
