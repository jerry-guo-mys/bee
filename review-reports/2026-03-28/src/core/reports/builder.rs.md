# Rust 代码审查报告

## 业务场景和职责

**AgentBuilder** 是统一的 Agent 构建器，负责：
- 统一配置和初始化 Agent 的各个组件
- 消除 TUI 与 Headless 的工具注册差异
- 构建工具注册表、LLM 客户端、Critic、技能加载器
- 生成完整的系统提示词（包含工具 schema）

**AgentComponents** 是预构建的 Agent 组件集合，可多会话共享。

### 关键依赖和设计权衡

- `tokio` 异步运行时：用于技能加载时的阻塞处理
- `thiserror`/`anyhow`：错误处理
- `tracing`：日志记录
- 条件编译：`browser`、`web` 特性标志控制工具注册

---

## 问题清单

### ❌ 严重问题（2 个）

**1. 阻塞调用在 async 上下文中可能导致 panic**

- **文件**: `src/core/builder.rs`
- **行号**: 212-218
```rust
if self.enable_skills {
    let loader = skill_loader.clone();
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            if let Err(e) = loader.load_all().await {
                tracing::warn!("Failed to load skills: {}", e);
            }
        });
    });
}
```
- **触发场景**: 当 `build_skill_loader()` 在单线程 runtime（如 `current_thread`）或无 runtime 的上下文中被调用时，`block_in_place` 会 panic
- **修复方案**:
```rust
// 方案 1: 使用 spawn_blocking 并 await
if self.enable_skills {
    let loader = skill_loader.clone();
    tokio::spawn(async move {
        if let Err(e) = loader.load_all().await {
            tracing::warn!("Failed to load skills: {}", e);
        }
    });
}

// 方案 2: 如果必须在同步上下文中运行
if self.enable_skills {
    let loader = skill_loader.clone();
    // 确保在多线程 runtime 中
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            let _guard = handle.enter();
            tokio::task::block_in_place(|| {
                handle.block_on(async {
                    let _ = loader.load_all().await;
                });
            });
        }
        Err(_) => {
            // 无 runtime，使用阻塞线程
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = loader.load_all().await;
                });
            }).join().unwrap();
        }
    }
}
```

**2. 错误被静默忽略**

- **文件**: `src/core/builder.rs`
- **行号**: 54-64
```rust
self.system_prompt = [
    "config/prompts/system.md",
    "../config/prompts/system.md",
    "config/prompts/default.md",
    "../config/prompts/default.md",
]
.into_iter()
.find_map(|p| std::fs::read_to_string(p).ok())
.unwrap_or_else(|| {
    "You are Bee, a helpful AI assistant...".to_string()
});
```
- **触发场景**: 当所有 prompt 文件都读取失败时，会静默使用内置默认 prompt，开发者可能不知道文件读取失败
- **修复方案**:
```rust
self.system_prompt = [
    "config/prompts/system.md",
    "../config/prompts/system.md",
    "config/prompts/default.md",
    "../config/prompts/default.md",
]
.into_iter()
.find_map(|p| {
    match std::fs::read_to_string(p) {
        Ok(content) => {
            tracing::info!("Loaded system prompt from {}", p);
            Some(content)
        }
        Err(e) => {
            tracing::debug!("Failed to load prompt from {}: {}", p, e);
            None
        }
    }
})
.unwrap_or_else(|| {
    tracing::warn!("No prompt file found, using built-in default");
    "You are Bee, a helpful AI assistant...".to_string()
});
```

### ⚠️ 警告问题（3 个）

**3. 重复的 enable_critic 检查**

- **文件**: `src/core/builder.rs`
- **行号**: 161-167
```rust
if !self.config.critic.enabled && !self.enable_critic {
    return None;
}
// enable_critic 为 false 时也不创建
if !self.enable_critic {
    return None;
}
```
- **触发场景**: 当 `enable_critic` 为 false 时，第一个 if 已经返回，第二个 if 永远不会执行
- **修复方案**:
```rust
// enable_critic 为 false 时不创建
if !self.enable_critic {
    return None;
}
// 检查配置是否启用 Critic
if !self.config.critic.enabled {
    return None;
}
```

**4. 硬编码路径可能导致跨平台问题**

- **文件**: `src/core/builder.rs`
- **行号**: 54-59, 194-196
```rust
["config/prompts/system.md", "../config/prompts/system.md", ...]
["config/prompts/critic.md", "../config/prompts/critic.md"]
```
- **触发场景**: 当工作目录不是预期位置时，相对路径会失败
- **修复方案**: 使用 `PathBuf` 拼接或从 workspace 根目录计算：
```rust
let prompt_paths = [
    self.workspace.join("config/prompts/critic.md"),
    self.workspace.parent().unwrap_or(&self.workspace).join("config/prompts/critic.md"),
];
let critic_prompt = prompt_paths
    .iter()
    .find_map(|p| std::fs::read_to_string(p).ok());
```

**5. LLM 客户端创建逻辑重复且硬编码**

- **文件**: `src/core/builder.rs`
- **行号**: 170-188
```rust
let critic_llm: Arc<dyn LlmClient> = if let Some(ref model) = self.config.critic.model {
    let provider = self.config.critic.provider.as_deref().unwrap_or(&self.config.llm.provider);

    if provider.to_lowercase() == "deepseek" {
        Arc::new(crate::llm::create_deepseek_client(Some(model)))
    } else {
        let base_url = self.config.llm.base_url.as_deref();
        let api_key = std::env::var("OPENAI_API_KEY").ok();
        Arc::new(crate::llm::OpenAiClient::new(
            base_url,
            model,
            api_key.as_deref(),
        ))
    }
} else {
    planner_llm
};
```
- **触发场景**: 添加新 provider 时需要修改多处；硬编码 `OPENAI_API_KEY` 可能不是预期的 API key
- **修复方案**: 提取为独立函数：
```rust
fn create_llm_for_critic(config: &AppConfig, model: &str, default_llm: Arc<dyn LlmClient>) -> Arc<dyn LlmClient> {
    let provider = config.critic.provider.as_deref().unwrap_or(&config.llm.provider);

    match provider.to_lowercase().as_str() {
        "deepseek" => Arc::new(crate::llm::create_deepseek_client(Some(model))),
        "openai" | "claude" | "gemini" | "qwen" => {
            // 使用统一的 LLM 工厂函数
            crate::llm::create_client(provider, Some(model), None)
        }
        _ => default_llm,
    }
}
```

### 💡 建议（3 个）

**6. 文档注释缺失**

- **文件**: `src/core/builder.rs`
- **行号**: 309-326
- **问题**: `create_agent_builder` 函数缺少文档注释
- **建议**: 添加文档说明参数和返回值

**7. 工具注册表构建逻辑可提取为独立函数**

- **文件**: `src/core/builder.rs`
- **行号**: 84-150
- **问题**: `build_tool_registry` 函数过长（67 行），难以测试和维护
- **建议**: 按工具类别拆分为多个小函数：
```rust
fn register_core_tools(&self, tools: &mut ToolRegistry) { ... }
fn register_code_tools(&self, tools: &mut ToolRegistry) { ... }
fn register_search_tools(&self, tools: &mut ToolRegistry, llm: Arc<dyn LlmClient>) { ... }
```

**8. 条件编译的工具注册可更清晰**

- **文件**: `src/core/builder.rs`
- **行号**: 107-111, 141-148
- **问题**: `#[cfg(feature = "...")]` 分散在函数体内
- **建议**: 使用辅助函数：
```rust
#[cfg(feature = "browser")]
fn register_browser_tool(&self, tools: &mut ToolRegistry) {
    tools.register(BrowserTool::new(...));
}
#[cfg(not(feature = "browser"))]
fn register_browser_tool(&self, _: &mut ToolRegistry) {}
```

---

## 设计确认（非问题）

- **使用 `Arc<dyn LlmClient>` 共享**: 正确，多组件需要共享 LLM 客户端
- **Builder 模式**: 符合 Rust 惯用法，支持链式调用
- **条件编译工具注册**: 合理，避免不必要的依赖
- **`block_in_place` + `block_on` 组合**: 在同步上下文中调用 async 的正确做法（但需注意 runtime 类型）

---

## 审查清单

| 类别 | 检查项 | 状态 |
|------|--------|------|
| 所有权 | `.clone()` 使用 | ✅ 合理（Arc 克隆廉价） |
| 错误处理 | `unwrap()` | ⚠️ 3 处需改进 |
| 错误处理 | `let _ =` | ✅ 无 |
| 错误处理 | `?` 传播 | ⚠️ 缺少错误传播 |
| Async | 阻塞调用 | ❌ `block_in_place` 可能 panic |
| Async | `spawn_blocking` | ⚠️ 可改用 |

---

## 总结

| 类别 | 数量 |
|------|------|
| ❌ 严重 | 2 |
| ⚠️ 警告 | 3 |
| 💡 建议 | 3 |

**报告路径**: `review-reports/2026-03-28/src/core/reports/builder.rs.md`
