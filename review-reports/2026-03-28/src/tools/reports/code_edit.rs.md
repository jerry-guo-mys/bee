# Rust 代码审查报告：code_edit.rs

## 业务场景和职责
- 代码编辑工具，安全地修改代码文件
- 支持精确字符串替换和缩进容忍匹配
- 自动创建备份文件

---

## 问题

### 1. **备份文件命名可能覆盖已有文件**
**行号**: 81
```rust
let backup_path = file_path.with_extension("bak");
```
**触发场景**: 如果原文件是 `main.rs.bak`，备份会变成 `main.bak`，可能覆盖已有文件
**修复方案**: 使用时间戳或唯一 ID：
```rust
let backup_path = file_path.with_extension(format!("bak.{}", std::process::id()));
// 或使用时间戳
```

### 2. **缩进容忍匹配逻辑复杂且可能有 bug**
**行号**: 134-136
```rust
let byte_pos: usize =
    content_lines[..i].join("\n").len() + if i > 0 { 1 } else { 0 };
```
**触发场景**: 字节位置计算可能有 off-by-one 错误，特别是第一行
**修复方案**: 添加测试覆盖边界情况：
```rust
// 添加测试：第一行、空文件等
```

### 3. **字节位置计算效率低**
**行号**: 134-136
```rust
content_lines[..i].join("\n").len()
```
**触发场景**: 每次计算都重新 join 字符串，O(n²) 复杂度
**修复方案**: 累积计算：
```rust
let byte_pos: usize = content_lines[..i]
    .iter()
    .map(|l| l.len() + 1)  // +1 for newline
    .sum::<usize>()
    .saturating_sub(1);  // -1 because last line doesn't need newline
```

### 4. **multi_edit 不支持原子回滚**
**行号**: 222-244
```rust
fn perform_multi_edit(&self, file_path: &Path, edits: Vec<(String, String)>)
```
**触发场景**: 如果第 3 个编辑失败，前 2 个已经写入文件，导致部分修改
**修复方案**: 使用单次写入或回滚机制：
```rust
// 先验证所有编辑都能成功，再一次性写入
// 或保存编辑前的内容用于回滚
```

### 5. **魔术数字硬编码**
**行号**: 38
```rust
max_file_size: 10 * 1024 * 1024, // 10MB
```
**修复方案**: 定义为常量：
```rust
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
```

### 6. **测试中的硬编码路径**
**行号**: 379, 410
```rust
let test_dir = std::path::PathBuf::from("./target/test_code_edit");
```
**触发场景**: 测试可能在 Windows 上失败（路径分隔符）
**修复方案**: 使用 PathBuf::from_iter 或 join：
```rust
let test_dir = PathBuf::new().join("target").join("test_code_edit");
```

---

## 设计确认（非问题）
- 缩进容忍匹配是实用的功能
- 备份机制是好的安全网
- 测试覆盖 exact_match 和 indentation_tolerance
- 错误信息清晰

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ⚠️ 部分 clone 可优化 |
| 错误处理 | ✓ 返回 Result |
| Async | ✓ 异步 trait 但无实际异步操作 |

## 问题统计
- ❌ 严重：1 (multi-edit 非原子)
- ⚠️ 警告：3
- 💡 建议：2
