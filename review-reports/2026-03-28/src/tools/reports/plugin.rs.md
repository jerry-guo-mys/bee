# Rust 代码审查报告：plugin.rs

## 业务场景和职责
- 技能插件工具，由配置 [[tools.plugins]] 注册
- 运行「程序 + 参数模板」实现动态扩展
- 参数模板中 {{workspace}} 替换为沙箱根路径

---

## 问题

### 1. **模板替换可能导致命令注入**
**行号**: 63-82
```rust
fn substitute(&self, args: &Value) -> Vec<String> {
    // 简单字符串替换
    s = s.replace("{{{{{}}}}}", k), &val);
}
```
**触发场景**: 如果 LLM 传入恶意参数如 `$(rm -rf /)`，可能被 shell 执行
**修复方案**: 当前设计是直接 exec program + args，不经过 shell，相对安全；但应确保文档说明：
```rust
// 文档说明：参数不经过 shell，直接传递给程序
```

### 2. **working_dir 验证在构造时但可能被绕过**
**行号**: 36-50
```rust
let working_dir = entry
    .working_dir
    .as_ref()
    .and_then(|p| {
        if p.components().any(|c| c == std::path::Component::ParentDir) {
            tracing::warn!(...);
            return None;
        }
        Some(workspace.join(p))
    })
```
**触发场景**: 只检查了 `..`，但 symlink 可能逃逸
**修复方案**: 在 execute 时再次验证：
```rust
// 在 execute 中检查 canonical_path 是否在 workspace 内
```

### 3. **插件超时错误信息不够详细**
**行号**: 142
```rust
.map_err(|_| format!("plugin timeout after {}s", self.timeout_secs))?
```
**触发场景**: 不知道是哪个插件超时
**修复方案**: 添加插件名称：
```rust
.map_err(|_| format!("plugin '{}' timed out after {}s", self.name, self.timeout_secs))?
```

### 4. **stderr 截断可能丢失关键信息**
**行号**: 154-165
```rust
let stderr_trim = stderr.trim();
let err = if stderr_trim.is_empty() {
    ...
} else {
    format!(
        "plugin exit code {}; stderr: {}",
        code,
        if stderr_trim.len() > 500 {
            format!("{}...", &stderr_trim[..500])
        } else {
            stderr_trim.to_string()
        }
    )
};
```
**设计确认**: 500 字符限制合理，防止输出过大

### 5. **程序执行失败时 stdout 丢失**
**行号**: 145-167
```rust
let stdout = String::from_utf8_lossy(&output.stdout);
let stderr = String::from_utf8_lossy(&output.stderr);
// stdout 未被使用在错误信息中
```
**修复方案**: 同时返回 stdout 便于调试：
```rust
// 在错误信息中包含 stdout
```

---

## 设计确认（非问题）
- 模板替换设计灵活
- 支持 {{workspace}} 是好的设计
- 超时和审计日志是好的安全实践
- kill_on_drop 防止僵尸进程

## 审查清单
| 类别 | 检查项 |
|------|--------|
| 所有权 | ✓ 无额外 clone |
| 错误处理 | ⚠️ 部分错误信息可改进 |
| Async | ✓ 使用 tokio::process |

## 问题统计
- ❌ 严重：0
- ⚠️ 警告：2
- 💡 建议：1
