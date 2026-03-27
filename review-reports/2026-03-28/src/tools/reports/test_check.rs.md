# Rust 代码审查报告：test_check.rs

## 业务场景和职责
- 检查代码编译（cargo check）工具
- 支持特性、所有目标检查
- 带超时和取消支持

---

## 问题

### 1. **cargo check 输出未区分 STDOUT 和 STDERR**
**行号**: 121-132
```rust
let mut result = format!(
    "Check Result: {}\n\n",
    if success { "✓ PASSED" } else { "✗ FAILED" }
);
if !stdout.is_empty() {
    result.push_str(&stdout);
}
if !stderr.is_empty() {
    result.push_str(&stderr);
}
```
**触发场景**: cargo check 的错误通常在 STDERR，但编译警告也在 STDERR
**修复方案**: 标注输出来源：
```rust
if !stdout.is_empty() {
    result.push_str("STDOUT:\n");
    result.push_str(&stdout);
}
if !stderr.is_empty() {
    result.push_str("STDERR:\n");
    result.push_str(&stderr);
}
```

### 2. **test_run.rs 代码重复**
**行号**: 整个文件
**触发场景**: test_check.rs 和 test_run.rs 结构高度相似，代码重复
**修复方案**: 提取公共逻辑到工具函数或宏：
```rust
// 提取 execute_cargo_command 函数
```

### 3. **all_targets 默认值可能意外触发测试编译**
**行号**: 87
```rust
let all_targets = args
    .get("all_targets")
    .and_then(|v| v.as_bool())
    .unwrap_or(true);
```
**触发场景**: `--all-targets` 会编译 benchmarks 等，可能意外耗时
**修复方案**: 默认改为 false：
```rust
.unwrap_or(false);
```

---

## 设计确认（非问题）
- 超时配置合理（120 秒）
- 支持取消是好的设计
- 使用 tokio::process 正确

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ✓ 正确传播错误 |
| Async | ✓ 使用 tokio::process + timeout |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：1 (all_targets 默认值)
- 💡 建议：1 (代码重复)
