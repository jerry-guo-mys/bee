# Rust 代码审查报告：source_validator.rs

## 业务场景和职责
- 验证 web 源的可信度
- 基于域名白名单和域名类型计算信任分数

---

## 问题

### 1. **魔术数字硬编码**
**行号**: 48, 52, 56, 60, 63
```rust
return 0.9;  // 信任域名
return 0.85; // .edu/.gov
return 0.8;  // wikipedia
return 0.75; // github/stackoverflow
return 0.5;  // 默认
```
**修复方案**: 定义为常量：
```rust
const TRUST_SCORE_TRUSTED: f32 = 0.9;
const TRUST_SCORE_EDU_GOV: f32 = 0.85;
const TRUST_SCORE_WIKIPEDIA: f32 = 0.8;
const TRUST_SCORE_CODE_SITES: f32 = 0.75;
const TRUST_SCORE_DEFAULT: f32 = 0.5;
```

### 2. **域名匹配逻辑重复**
**行号**: 25-37
```rust
fn domain_matches_pattern(domain: &str, pattern: &str) -> bool {
    let domain = domain.to_lowercase();
    let pattern = pattern.trim().to_lowercase();
    // ...
}
```
**触发场景**: 与 search.rs、deep_search.rs 中的逻辑重复
**修复方案**: 移到共享工具函数模块：
```rust
// 在 tools/source_adapter.rs 或 utils 中定义
pub fn domain_matches_pattern(domain: &str, pattern: &str) -> bool { ... }
```

### 3. **content 参数未使用**
**行号**: 77
```rust
async fn execute(&self, args: Value) -> Result<String, String> {
    // args 中包含 content 但未被使用
```
**触发场景**: description 中提到接受 content 参数，但实际未使用
**修复方案**: 要么使用 content 进行内容分析，要么从 description 中移除：
```rust
// 更新 description 为：Args: {"url": "https://..."}
```

### 4. **trust_score 计算逻辑可扩展性差**
**行号**: 40-64
```rust
fn calculate_trust_score(&self, url: &str) -> f32 {
    // 硬编码的 if-else 链
}
```
**修复方案**: 使用规则引擎或配置文件：
```rust
// 从配置加载信任规则
// 或使用策略模式
```

---

## 设计确认（非问题）
- 信任分数设计合理
- .edu/.gov 自动高信任是好的启发式
- 输出结构化 JSON 便于后续处理

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ✓ 简单验证 |
| Async | ✓ 异步 trait 但无实际异步操作 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：2
