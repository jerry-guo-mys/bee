# Rust 代码审查报告：metadata.rs

## 业务场景和职责
- 工具元数据定义：用于路由、策略、审计与提示词构建
- 定义 ToolScope、ToolIntent、ToolRisk 等枚举
- ToolMetadata 结构体支持 builder 模式

---

## 问题

### 1. **枚举变体过多且可能重复**
**行号**: 58-69
```rust
pub enum ToolUseCase {
    DirectExplanation,
    TimeSensitiveCurrent,
    ExternalGitHubRepo,
    LocalWorkspaceInspection,
    Weather,
    News,
    ExchangeRate,
    MarketQuote,
    SportsScore,
    Testing,
}
```
**触发场景**: UseCase 与 CapabilitySubgroup 有重叠（如 Weather、News 等）
**修复方案**: 考虑合并或明确区分：
```rust
// 要么合并，要么明确区分：UseCase 是场景，Subgroup 是能力
```

### 2. **ToolMetadata 字段过多（14 个）**
**行号**: 130-148
```rust
pub struct ToolMetadata {
    pub scope: ToolScope,
    pub intents: Vec<ToolIntent>,
    // ... 14 个字段
}
```
**触发场景**: 结构体过大，难以维护和理解
**修复方案**: 分组为子结构：
```rust
pub struct ToolMetadata {
    pub classification: ToolClassification,  // scope, intents, risk
    pub behavior: ToolBehavior,            // freshness, output_shape, side_effects
    pub routing: ToolRouting,              // use_cases, critic_mode, preferred_rank
    pub capability: ToolCapability,        // group, subgroup
    pub costs: ToolCosts,                  // latency, token, api, overall
}
```

### 3. **builder 方法链可能遗漏必填字段**
**行号**: 150-243
```rust
pub fn new(scope: ToolScope, intents: Vec<ToolIntent>) -> Self {
    // 设置默认值
}
```
**设计确认**: 使用 new() + builder 方法是可以的，所有字段都有默认值

### 4. **ToolCostClass 枚举命名不直观**
**行号**: 112-118
```rust
pub enum ToolCostClass {
    Low,
    Medium,
    High,
}
```
**触发场景**: Cost 可能被误解为金钱成本，实际是延迟/资源成本
**修复方案**: 重命名为 ToolResourceClass 或添加文档说明

### 5. **Serialize derive 但无 Deserialize**
**行号**: 5, 17, 等
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
```
**触发场景**: 如果不能反序列化，配置加载可能受限
**修复方案**: 添加 Deserialize（如果需要从配置加载）

---

## 设计确认（非问题）
- 枚举设计全面覆盖场景
- builder 模式便于构造
- 元数据分离关注点是好的架构

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ Copy 类型 |
| 错误处理 | ✓ 无错误 |
| Async | ✓ 无异步 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：2
