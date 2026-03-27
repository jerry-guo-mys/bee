# Rust 代码审查报告 - intent.rs

## 业务场景和职责

**文件路径**: `src/gateway/intent.rs`

**职责**: 意图识别模块，分析用户输入，识别意图并路由到合适的能力端点。

**关键设计**:
- 双模式识别：快速规则匹配 + LLM 语义识别
- 支持多种意图类型：Chat/Code/Search/FileOperation/Shell/Memory/Task/Browse
- 基于意图的工具和技能推荐
- GitHub 架构查询特殊处理

---

## 问题列表

### 1. ❌ 严重 - `llm_recognize` 中 `unwrap_or(Intent::Chat)` 掩盖潜在错误

**问题代码** (行 106):
```rust
self.llm_recognize(user_input).await.unwrap_or(Intent::Chat)
```

**触发场景**: 当 LLM 调用失败时，静默 fallback 到 Chat，用户和开发者都无法感知错误。

**修复方案**:
```rust
// 方案 1: 记录错误日志
match self.llm_recognize(user_input).await {
    Ok(intent) => intent,
    Err(e) => {
        tracing::warn!("LLM intent recognition failed: {}, falling back to Chat", e);
        Intent::Chat
    }
}

// 方案 2: 返回 Result，由调用者决定
pub async fn recognize(&self, user_input: &str) -> Result<Intent, IntentError> {
    // ...
}
```

---

### 2. ❌ 严重 - `fast_match` 中 URL 提取逻辑不完整

**问题代码** (行 113, 415-422):
```rust
let github_url = extract_url(input).filter(|url| is_github_repo_url(url));

fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            return Some(word.to_string());
        }
    }
    None
}
```

**触发场景**:
- URL 后面有标点符号时会包含标点（如 "https://github.com/foo/bar,"）
- URL 被括号包围时会包含括号

**修复方案**:
```rust
fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        // 移除常见的 URL 周围标点
        let cleaned = word.trim_end_matches(|c: char| matches!(c, ',' | '.' | '!' | '?' | ')' | ']' | '}'));
        let cleaned = cleaned.trim_start_matches(|c: char| matches!(c, '(' | '[' | '{' | '<'));

        if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
            return Some(cleaned.to_string());
        }
    }
    None
}
```

---

### 3. ⚠️ 警告 - `suggest_tools` 中对 `Intent::Search` 的 GitHub 范围检查重复

**问题代码** (行 366-376):
```rust
Intent::Search { query } => {
    if query_has_external_github_scope(query) {
        vec![
            "github_repo_inspect".to_string(),
            "search".to_string(),
            "deep_search".to_string(),
        ]
    } else {
        vec!["search".to_string(), "deep_search".to_string()]
    }
}
```

**触发场景**: `query_has_external_github_scope` 在 `fast_match` 中已调用过，这里重复计算。

**修复方案**: 在 `recognize` 时缓存 GitHub scope 标志，或重构为：
```rust
pub struct IntentRecognitionResult {
    pub intent: Intent,
    pub suggested_tools: Vec<String>,
    pub is_github_query: bool,
}
```

---

### 4. ⚠️ 警告 - `llm_recognize` 中硬编码的 prompt 未支持多语言

**问题代码** (行 261-285):
```rust
let system_prompt = r#"You are an intent classifier. Analyze the user's input and classify their intent.
Output ONLY one of these intent types (no explanation):
- chat: General conversation...
```

**触发场景**: 用户输入中文时，LLM 可能输出中文意图名称导致解析失败。

**修复方案**:
```rust
// 在 prompt 中明确说明
let system_prompt = r#"You are an intent classifier. Analyze the user's input and classify their intent.
IMPORTANT: Output ONLY the English intent type from the list below, regardless of input language.
Do not include any explanation or other text.
```

---

### 5. 💡 建议 - `fast_match` 函数过长，可拆分为多个辅助函数

**问题代码** (行 110-257): 148 行的 match 链

**修复方案**: 按意图类型拆分：
```rust
fn fast_match_search(&self, input: &str, input_lower: &str) -> Option<Intent> { ... }
fn fast_match_browse(&self, input: &str, input_lower: &str) -> Option<Intent> { ... }
fn fast_match_shell(&self, input: &str, input_lower: &str) -> Option<Intent> { ... }
// ...

fn fast_match(&self, input: &str) -> Option<Intent> {
    let input_lower = input.to_lowercase();

    self.fast_match_search(input, &input_lower)
        .or_else(|| self.fast_match_browse(input, &input_lower))
        .or_else(|| self.fast_match_shell(input, &input_lower))
        // ...
}
```

---

### 6. 💡 建议 - `is_github_architecture_query` 中的关键字列表可配置化

**问题代码** (行 436-457):
```rust
fn is_github_architecture_query(input_lower: &str) -> bool {
    [
        "架构", "技术架构", "系统设计", // ...
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword))
}
```

**说明**: 硬编码关键字列表，如需添加新词需要修改代码。建议移至配置文件。

---

### 7. 💡 建议 - 测试用例覆盖率不足

**问题代码** (行 465-529): 仅 5 个测试用例

**说明**: 缺少以下场景的测试：
- `llm_recognize` 的 mock 测试
- `suggest_tools` 的所有分支
- 边界情况（空输入、特殊字符）

---

## 设计确认（非问题）

1. **快速匹配 + LLM 降级策略** - 合理，节省成本。
2. **GitHub 架构查询特殊处理** - 针对场景的优化设计。
3. **意图 - 工具 - 技能三层推荐** - 清晰的职责分离。

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `.clone()` 使用 | ✅ 合理 |
| 错误处理 | `unwrap_or` 掩盖错误 | ❌ 需改进 |
| Async | 阻塞调用 | ✅ 无 |

---

## 问题统计

| 等级 | 数量 |
|------|------|
| ❌ 严重 | 2 |
| ⚠️ 警告 | 2 |
| 💡 建议 | 3 |
