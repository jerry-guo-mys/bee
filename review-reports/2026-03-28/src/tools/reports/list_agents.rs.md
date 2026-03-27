# Rust 代码审查报告：list_agents.rs

## 业务场景和职责
- 列出 workspace 内所有动态创建的 agent，供统筹 agent 查看
- 从 agents.json 文件读取 agent 列表并格式化输出

---

## 问题

### 1. **未处理的 unwrap() 可能导致 panic**
**行号**: 37
```rust
serde_json::from_str(&data).unwrap_or_default()
```
**触发场景**: 当 agents.json 文件内容不是合法 JSON 时
**修复方案**: 使用 match 或 ok() 处理错误：
```rust
serde_json::from_str(&data).unwrap_or_else(|e| {
    tracing::warn!("Failed to parse agents.json: {}", e);
    Vec::new()
})
```

### 2. **路径遍历风险未完全防护**
**行号**: 32
```rust
let path = self.workspace.join("agents.json");
```
**触发场景**: 如果 workspace 路径被外部控制，可能导致路径穿越
**修复方案**: 使用 canonicalize 验证路径：
```rust
let path = self.workspace.join("agents.json");
let canonical = path.canonicalize().unwrap_or(path);
```

---

## 设计确认（非问题）
- 工具职责单一，只负责读取不修改
- 返回格式简洁明了
- 空状态处理合理

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ 1 处 unwrap_or_default |
| Async | ✓ 简单读取无异步 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：1
- 💡 建议：1
