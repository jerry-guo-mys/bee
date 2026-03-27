# Rust 代码审查报告：source_adapter.rs

## 业务场景和职责
- 内容源适配：统一识别 GitHub、社交帖文、搜索结果页、动态网页等内容源类型
- 提供 URL 分类、镜像 URL 生成等功能

---

## 问题

### 1. **正则编译在每次调用时执行**
**行号**: 105
```rust
let Ok(id_re) = Regex::new(r"\b\d{12,}\b") else {
    return Vec::new();
};
```
**触发场景**: social_status_urls_from_text 每次调用都重新编译正则
**修复方案**: 使用 once_cell 缓存：
```rust
use once_cell::sync::Lazy;
static STATUS_ID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{12,}\b").unwrap()
});
```

### 2. **extract_domain 函数重复**
**行号**: 23-26
```rust
fn extract_domain(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    parsed.host_str().map(|host| host.to_lowercase())
}
```
**触发场景**: 与 search.rs、deep_search.rs、source_validator.rs 中的逻辑重复
**修复方案**: 移到共享工具函数模块：
```rust
// 在 tools/utils.rs 或 crate::utils 中定义
pub fn extract_domain(url: &str) -> Option<String> { ... }
```

### 3. **social_mirror_urls 硬编码镜像站点**
**行号**: 86
```rust
["fixupx.com", "fxtwitter.com", "vxtwitter.com", "nitter.net"]
```
**触发场景**: 镜像站点可能失效，需要代码修改
**修复方案**: 移到配置中：
```rust
// 从配置加载镜像站点列表
```

### 4. **classify_url_source 逻辑复杂但无测试覆盖所有分支**
**行号**: 122-167
```rust
pub fn classify_url_source(url: &str) -> SourceKind {
    // 复杂分支逻辑
}
```
**触发场景**: 测试只覆盖了部分分支（173-193 行）
**修复方案**: 添加更多测试覆盖：
```rust
// 添加测试：SearchResultsPage, ArticlePage, DynamicWebPage, etc.
```

### 5. **magic string 硬编码**
**行号**: 145-162
```rust
"medium.com", "substack.com", "dev.to", "wikipedia.org",
"finance.yahoo.com", "query1.finance.yahoo.com", "open.er-api.com"
```
**修复方案**: 定义为常量或配置：
```rust
const ARTICLE_DOMAINS: &[&str] = &["medium.com", "substack.com", ...];
```

---

## 设计确认（非问题）
- SourceKind 枚举设计全面
- 社交媒体镜像 URL 生成实用
- 搜索 URL 解析支持多个引擎

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无 clone |
| 错误处理 | ✓ 使用 Option/Result |
| Async | ✓ 无异步 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：3
