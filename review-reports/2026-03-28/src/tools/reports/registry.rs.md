# Rust 代码审查报告：registry.rs

## 业务场景和职责
- 工具注册表：按名称存储 Arc<dyn Tool>
- 支持 register / get / execute / tool_names
- 生成工具 schema JSON

---

## 问题

### 1. **Arc::clone 在 get 方法中**
**行号**: 79-81
```rust
pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
    self.tools.get(name).cloned()
}
```
**设计确认**: 这是正确的，返回 Arc 克隆以便调用方持有

### 2. **to_schema_json 中 tool.metadata() 调用无缓存**
**行号**: 134-146
```rust
pub fn to_schema_json(&self) -> String {
    let tools: Vec<serde_json::Value> = self
        .tools
        .iter()
        .map(|(name, tool)| {
            serde_json::json!({
                "name": name,
                "description": tool.description(),
                "parameters": tool.parameters_schema(),
                "metadata": tool.metadata(),
            })
        })
        .collect();
```
**触发场景**: 每次调用都重新计算，如果工具多可能耗时
**修复方案**: 添加缓存（如果频繁调用）：
```rust
// 或使用 LazyLock 缓存结果
```

### 3. **HashMap 迭代顺序不确定**
**行号**: 125-128
```rust
pub fn tool_descriptions(&self) -> Vec<(String, String)> {
    self.tools
        .iter()
        .map(|(name, tool)| (name.clone(), tool.description().to_string()))
        .collect()
}
```
**触发场景**: 每次调用返回的顺序可能不同，影响 system prompt 稳定性
**修复方案**: 使用 BTreeMap 或排序：
```rust
use std::collections::BTreeMap;
// 或在 collect 前排序
```

---

## 设计确认（非问题）
- 使用 Arc<dyn Tool> 是正确的设计
- 支持 cancellable execute 是好的
- to_schema_json 包含 metadata 是全面的

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 使用 Arc |
| 错误处理 | ✓ 正确传播错误 |
| Async | ✓ 异步 execute |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：1 (HashMap 顺序)
- 💡 建议：1 (缓存 schema)
