# Rust 代码审查报告：github_repo_inspect.rs

## 业务场景和职责
- 检查外部 GitHub 仓库工具
- 返回结构化技术架构信号
- 支持 repo/blob/tree 三种目标类型

---

## 问题

### 1. **Client 构建使用 unwrap_or_default**
**行号**: 41-46
```rust
client: Client::builder()
    .timeout(std::time::Duration::from_secs(20))
    .user_agent(USER_AGENT)
    .build()
    .unwrap_or_default(),
```
**修复方案**: 使用 expect 提供更好错误信息：
```rust
.build()
.expect("Failed to create HTTP client")
```

### 2. **GitHub API 无认证支持**
**行号**: 86-99
```rust
async fn fetch_json(&self, url: &str) -> Result<Value, String> {
    let resp = self
        .client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
```
**触发场景**: 未认证 API 请求速率限制较低（60 次/小时）
**修复方案**: 支持 GITHUB_TOKEN 环境变量：
```rust
// 从环境变量读取 GITHUB_TOKEN 并添加 Authorization header
```

### 3. **文件片段硬编码最大 10 个**
**行号**: 253
```rust
selected_paths.truncate(10);
```
**修复方案**: 定义为常量：
```rust
const MAX_KEY_FILES: usize = 10;
```

### 4. **栈检测逻辑有限**
**行号**: 123-155
```rust
fn detect_stack(paths: &[String]) -> Vec<String> {
    let mut stack = BTreeSet::new();
    for path in paths {
        match path.as_str() {
            p if p.ends_with("package.json") => { ... }
            // ...
        }
    }
}
```
**触发场景**: 仅检测有限的项目类型
**修复方案**: 扩展检测更多项目类型（可选）

### 5. **URL 解析不支持 git@ SSH 格式**
**行号**: 51-83
```rust
fn parse_target(input: &str) -> Option<GitHubTarget> {
    let url = input
        .trim()
        .strip_prefix("https://github.com/")
        .or_else(|| input.trim().strip_prefix("http://github.com/"))?;
```
**触发场景**: SSH 格式 `git@github.com:org/repo.git` 不被支持
**修复方案**: 支持 SSH 格式（可选）：
```rust
// 添加 SSH 格式解析
```

---

## 设计确认（非问题）
- 使用 GitHub API 是正确选择
- 支持 repo/blob/tree 三种类型全面
- 架构摘要生成实用

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ✓ 正确传播错误 |
| Async | ✓ 异步 HTTP 请求 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：3
