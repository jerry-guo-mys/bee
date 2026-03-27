# Rust 代码审查报告：filesystem.rs

## 业务场景和职责
- 沙箱文件系统工具
- SafeFs 绑定 root_dir，所有路径经 resolve 校验必须在 root 下
- CatTool / LsTool 基于 SafeFs 提供 cat / ls 能力

---

## 问题

### 1. **fallback_root 逻辑可能导致意外访问**
**行号**: 29-32
```rust
let fallback_root = std::env::current_dir()
    .ok()
    .and_then(|cwd| cwd.canonicalize().ok().or(Some(cwd)))
    .filter(|cwd| cwd != &root_dir);
```
**触发场景**: 与 code_read.rs 相同，可能意外访问当前工作目录
**修复方案**: 添加日志记录 fallback 使用

### 2. **resolve_under_root 静态方法但访问根目录**
**行号**: 39-50
```rust
fn resolve_under_root(root: &Path, path: &str) -> Result<PathBuf, AgentError> {
    let full = root.join(path);
    let canonical = full.canonicalize()
        .map_err(|_| AgentError::ToolExecutionFailed(format!("Path not found: {}", path)))?;
```
**设计确认**: 逻辑正确，先 join 再 canonicalize

### 3. **list_dir 跳过隐藏文件但无配置**
**行号**: 89-96
```rust
if !name.starts_with('.') {
    entries.push(format!("{}{}", name, ty));
}
```
**触发场景**: 用户可能需要查看隐藏文件（如 .git、.env）
**修复方案**: 添加参数控制：
```rust
// 添加 include_hidden 参数
```

### 4. **AgentError 依赖**
**行号**: 11
```rust
use crate::core::AgentError;
```
**触发场景**: filesystem 工具依赖 core 模块的 AgentError，耦合度高
**设计确认**: 这是架构选择，可以接受

### 5. **read_file 使用 std::fs 而非 tokio::fs**
**行号**: 71-74
```rust
pub fn read_file(&self, path: &str) -> Result<String, AgentError> {
    let resolved = self.resolve(path)?;
    std::fs::read_to_string(&resolved)
```
**触发场景**: 同步 IO 可能阻塞 async runtime
**修复方案**: 使用 tokio::fs 或 spawn_blocking：
```rust
pub async fn read_file(&self, path: &str) -> Result<String, AgentError> {
    tokio::fs::read_to_string(&resolved).await
        .map_err(|e| AgentError::ToolExecutionFailed(format!("Read failed: {}", e)))
}
```

---

## 设计确认（非问题）
- 沙箱设计正确
- 路径逃逸防护到位
- 输出结构化 JSON 便于消费

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ Clone trait derived |
| 错误处理 | ✓ 使用 AgentError |
| Async | ⚠️ 部分同步 IO |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：1 (同步 IO)
- 💡 建议：2
