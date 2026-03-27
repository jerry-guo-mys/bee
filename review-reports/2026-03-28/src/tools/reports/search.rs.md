# Rust 代码审查报告：search.rs

## 业务场景和职责
- Search/Web 工具：抓取 URL 内容
- 域名白名单、超时、结果大小限制
- 对 HTML 响应使用 html2text 提取可读文本

---

## 问题

### 1. **Client 构建使用 unwrap_or_default**
**行号**: 172-188
```rust
let client = Client::builder()
    .timeout(std::time::Duration::from_secs(timeout_secs))
    .user_agent(USER_AGENT)
    // ...
    .build()
    .unwrap_or_default();
```
**修复方案**: 使用 expect 提供更好错误信息：
```rust
.build()
.expect("Failed to create HTTP client")
```

### 2. **正则编译在每次调用时执行**
**行号**: 263
```rust
let Ok(regex) = regex::Regex::new(r#"https?://[^\s"'<>)]+"#) else {
    return Vec::new();
};
```
**触发场景**: extract_candidate_urls 每次调用都重新编译正则
**修复方案**: 使用 once_cell 缓存：
```rust
use once_cell::sync::Lazy;
static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"https?://[^\s"'<>)]+"#).unwrap()
});
```

### 3. **extract_domain 函数重复**
**行号**: 87-95
```rust
fn extract_domain(url: &str) -> Option<String> {
    let url = url.trim();
    let url = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    // ...
}
```
**触发场景**: 与 deep_search.rs、source_validator.rs 等中的逻辑重复
**修复方案**: 移到共享工具函数模块

### 4. **fetch_search_engine_results 中候选 URL 提取逻辑脆弱**
**行号**: 262-287
```rust
fn extract_candidate_urls(&self, html: &str, engine_host: &str) -> Vec<String> {
    // 正则提取 URL
}
```
**触发场景**: 搜索引擎页面结构变化可能导致提取失败
**修复方案**: 使用 HTML 解析库如 scraper 或 html5ever

### 5. **魔术数字硬编码**
**行号**: 283
```rust
if urls.len() >= 5 {
    break;
}
```
**修复方案**: 定义为常量：
```rust
const MAX_CANDIDATE_URLS: usize = 5;
```

### 6. **is_blocked_host 逻辑与 extract_domain 重复**
**行号**: 97-115
```rust
fn is_blocked_host(domain: &str) -> bool {
    // 检查内网 IP 等
}
```
**设计确认**: 逻辑正确，但可以合并到 extract_domain 的验证中

---

## 设计确认（非问题）
- 域名白名单是好的安全措施
- 社交媒体镜像 URL 回退实用
- 可恢复错误处理设计优秀
- html2text 提取可读文本正确

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ✓ 可恢复错误处理 |
| Async | ✓ 异步 HTTP 请求 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：3
- 💡 建议：2
