# Rust 代码审查报告：deep_search.rs

## 业务场景和职责
- 深度研究工具，通过多轮自主搜索研究复杂主题
- 自动分解查询，执行迭代搜索，综合发现
- 使用 LLM 生成后续查询和综合结果

---

## 问题

### 1. **Client 构建使用 unwrap_or_default**
**行号**: 53-59
```rust
client: Client::builder()
    .timeout(std::time::Duration::from_secs(15))
    .user_agent(USER_AGENT)
    .build()
    .unwrap_or_default(),
```
**修复方案**: 使用 expect 提供更好错误信息：
```rust
.build()
.expect("Failed to create HTTP client")
```

### 2. **LLM 响应解析脆弱**
**行号**: 296-299
```rust
let queries: Vec<String> =
    serde_json::from_str(&response).unwrap_or_else(|_| vec![query.to_string()]);
```
**触发场景**: LLM 可能返回非 JSON 格式响应
**修复方案**: 使用 extract_first_json_object 辅助函数（已有 147-179 行定义）：
```rust
let queries: Vec<String> = extract_first_json_object(&response)
    .and_then(|s| serde_json::from_str(s).ok())
    .unwrap_or_else(|| vec![query.to_string()]);
```

### 3. **魔术数字硬编码**
**行号**: 20, 21, 22, 600
```rust
max_rounds: usize,  // 默认 3
max_results_per_round: usize,  // 默认 3
timeout_secs: u64,  // 默认 60
```
**修复方案**: 定义为常量：
```rust
const DEFAULT_MAX_ROUNDS: usize = 3;
const DEFAULT_MAX_RESULTS_PER_ROUND: usize = 3;
const DEFAULT_TIMEOUT_SECS: u64 = 60;
```

### 4. **信任域名列表硬编码在测试中**
**行号**: 595-609
```rust
fn tool() -> DeepSearchTool {
    DeepSearchTool::new(
        Arc::new(MockLlmClient),
        3,
        3,
        60,
        vec![
            "x.com".into(),
            "twitter.com".into(),
            // ...
        ],
    )
}
```
**触发场景**: 测试工具与实际工具配置不一致
**修复方案**: 使用相同的配置源

### 5. **synthesize_results 错误处理粗糙**
**行号**: 443-452
```rust
let synthesis_text = Self::extract_first_json_object(&response).unwrap_or(&response);
let synthesis: Value = match serde_json::from_str(synthesis_text) {
    Ok(value) => value,
    Err(_) => {
        return Ok((
            Self::truncate_chars(response.trim(), 1200),
            Vec::new(),
            Vec::new(),
        ));
    }
};
```
**触发场景**: JSON 解析失败时返回空结果，无错误提示
**修复方案**: 记录日志：
```rust
Err(e) => {
    tracing::warn!("Failed to parse synthesis JSON: {}", e);
    // ...
}
```

### 6. **search_round 中 tracing 日志不一致**
**行号**: 313, 323
```rust
tracing::warn!(query = %query, error = %err, "deep_search query failed");
tracing::warn!(url = %url, error = %err, "deep_search fetch failed");
```
**设计确认**: 日志格式正确，使用结构化字段

---

## 设计确认（非问题）
- 多轮搜索设计优秀
- 查询分解和后续查询生成智能
- 可信域名过滤是好的安全措施
- HTML 转文本回退逻辑健壮

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 使用 Arc |
| 错误处理 | ⚠️ 部分 LLM 响应解析脆弱 |
| Async | ✓ 异步 HTTP + LLM |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：3
- 💡 建议：2
