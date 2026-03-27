# Rust 代码审查报告：knowledge_graph.rs

## 业务场景和职责
- 构建知识图谱工具，从研究信息中提取实体和关系
- 使用 LLM 分析文本，输出结构化图谱数据

---

## 问题

### 1. **build 方法未实现功能**
**行号**: 43-49
```rust
#[allow(dead_code)]
pub fn build(&self, topic: &str, _information: &str) -> KnowledgeGraph {
    KnowledgeGraph {
        topic: topic.to_string(),
        nodes: Vec::new(),
        edges: Vec::new(),
    }
}
```
**触发场景**: build 方法返回空图谱，实际功能由 execute 实现，设计不一致
**修复方案**: 要么实现 build 方法，要么移除它：
```rust
// 移除或标记为 deprecated
```

### 2. **LLM 响应解析无错误恢复**
**行号**: 108-109
```rust
let graph_data: Value =
    serde_json::from_str(&response).map_err(|e| format!("Failed to parse graph: {}", e))?;
```
**触发场景**: LLM 可能返回非 JSON 格式响应，导致解析失败
**修复方案**: 添加 JSON 提取逻辑（类似 deep_search.rs 的做法）：
```rust
// 尝试从响应中提取第一个 JSON 对象
let json_text = extract_first_json_object(&response)
    .ok_or_else(|| "LLM response does not contain valid JSON".to_string())?;
let graph_data: Value = serde_json::from_str(&json_text)
    .map_err(|e| format!("Failed to parse graph: {}", e))?;
```

### 3. **Arc<dyn LlmClient> 生命周期未说明**
**行号**: 12
```rust
pub struct KnowledgeGraphBuilder {
    llm: Arc<dyn LlmClient>,
}
```
**触发场景**: 作为工具使用时，LLM client 的生命周期管理需由调用方负责
**修复方案**: 当前设计合理，但应确保文档说明

### 4. **KnowledgeNode 和 KnowledgeEdge 未被使用**
**行号**: 16-28
```rust
pub struct KnowledgeNode { ... }
pub struct KnowledgeEdge { ... }
```
**触发场景**: 这些结构体定义后未在 execute 中使用，代码冗余
**修复方案**: 要么使用它们构建强类型输出，要么移除：
```rust
// 使用这些结构体替代纯 JSON 操作
```

---

## 设计确认（非问题）
- 使用 LLM 提取实体是合理方案
- 输出包含可视化提示是好的用户体验
- 参数验证逻辑正确

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 使用 Arc |
| 错误处理 | ⚠️ LLM 响应解析脆弱 |
| Async | ✓ 使用 async LLM client |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：2
