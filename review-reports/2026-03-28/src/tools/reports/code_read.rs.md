# Rust 代码审查报告：code_read.rs

## 业务场景和职责
- 代码读取工具，安全地读取项目代码文件
- 用于自主迭代时读取代码内容进行分析
- 支持 offset/limit 分页读取

---

## 问题

### 1. **fallback_root 逻辑可能导致意外访问**
**行号**: 32-35
```rust
let fallback_root = std::env::current_dir()
    .ok()
    .and_then(|cwd| cwd.canonicalize().ok().or(Some(cwd)))
    .filter(|cwd| cwd != &allowed_root);
```
**触发场景**: 用户可能意外访问当前工作目录的文件，而非项目文件
**修复方案**: 添加日志记录 fallback 使用：
```rust
// 在 validate_path 中使用 fallback 时记录日志
```

### 2. **validate_under_root 静态方法命名不一致**
**行号**: 52
```rust
fn validate_under_root(root: &Path, file_path: &str) -> Result<PathBuf, String> {
```
**触发场景**: 作为私有方法是可以的，但命名与实例方法 validate_path 不一致
**修复方案**: 重命名为一致的命名：
```rust
fn resolve_path_under_root(root: &Path, file_path: &str) -> Result<PathBuf, String> {
```

### 3. **错误信息泄露路径结构**
**行号**: 63-67
```rust
Err(format!(
    "Access denied: path '{}' is outside allowed root '{}'",
    file_path,
    root.display()
))
```
**设计确认**: 这是内部错误，可以接受

### 4. **max_lines 和 max_line_length 无配置**
**行号**: 40-41
```rust
max_lines: 2000,
max_line_length: 2000,
```
**修复方案**: 支持配置：
```rust
// 通过构造函数参数或 builder 模式
```

---

## 设计确认（非问题）
- 路径验证逻辑正确
- 支持 offset/limit 分页是好的设计
- 输出带行号便于定位
- 测试覆盖路径验证安全

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ✓ 正确传播错误 |
| Async | ✓ 异步 trait 但无实际异步操作 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：1
- 💡 建议：2
