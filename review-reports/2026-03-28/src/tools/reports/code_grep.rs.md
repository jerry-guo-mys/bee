# Rust 代码审查报告：code_grep.rs

## 业务场景和职责
- 代码搜索工具，在代码库中搜索模式
- 支持正则表达式
- 用于自主迭代时查找代码位置

---

## 问题

### 1. **unwrap() 在 glob 模式解析**
**行号**: 109
```rust
let include_pattern = include
    .map(|p| glob::Pattern::new(p).unwrap_or_else(|_| glob::Pattern::new("*").unwrap()));
```
**触发场景**: 无效的 glob 模式会回退到 "*"，但 "*" 的 unwrap() 也可能失败（虽然极不可能）
**修复方案**: 使用 expect 提供更好错误信息：
```rust
let include_pattern = include
    .map(|p| glob::Pattern::new(p)
        .unwrap_or_else(|e| {
            tracing::warn!("Invalid glob pattern '{}', using '*': {}", p, e);
            glob::Pattern::new("*").expect("* is always a valid pattern")
        }));
```

### 2. **walkdir 错误被静默忽略**
**行号**: 119
```rust
.filter_map(|e| e.ok())
```
**触发场景**: 权限不足的目录被静默跳过，用户可能不知道
**修复方案**: 记录警告日志：
```rust
.filter_map(|e| {
    e.map_err(|e| tracing::warn!("Directory access error: {}", e))
     .ok()
})
```

### 3. **magic number 硬编码**
**行号**: 26, 27, 112
```rust
max_results: 50,
max_file_size: 1024 * 1024, // 1MB
.max_depth(10)
```
**修复方案**: 定义为常量：
```rust
const MAX_RESULTS: usize = 50;
const MAX_FILE_SIZE: usize = 1024 * 1024;
const MAX_DEPTH: usize = 10;
```

### 4. **路径验证在搜索前但错误信息不够清晰**
**行号**: 217
```rust
let search_path = self.validate_path(path)?;
```
**触发场景**: 用户传入绝对路径可能被拒绝，但错误信息未说明如何解决
**修复方案**: 改进错误信息：
```rust
// 在 description 中说明路径必须在 allowed_root 内
```

---

## 设计确认（非问题）
- 支持正则和普通搜索是好的设计
- 文件类型过滤（include 参数）实用
- 跳过隐藏目录和目标目录（target, node_modules）合理
- 结果截断和行号显示便于阅读

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ 部分 unwrap 和静默忽略 |
| Async | ✓ 异步 trait 但无实际异步操作 |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：2
