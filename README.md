# Bee 🐝

[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-0.1.0-green.svg)](Cargo.toml)
[![Tests](https://img.shields.io/badge/Tests-240%20passed-brightgreen.svg)](src/lib.rs)

> 高性能、安全且具备长期记忆的 Rust 个人智能体系统

Bee 是一个基于 ReAct 架构的智能体，支持多工具协作、分层记忆系统、技能插件、自我进化和多种交互界面（TUI/Web/WhatsApp/Lark/Gateway）。

---

## ✨ 功能特性

- 🤖 **智能编排**: ReAct 循环 + Planner/Critic 双核心，自主规划与反思（20 步上限防止死循环）
- 🧠 **分层记忆**: 短期对话（智能剪枝）+ 中期工作区 + 长期持久化记忆（向量检索 + 文件存储）
- 🔍 **RAG 检索增强**: 文档分块、混合检索（向量 + 关键词 RRF 融合）、上下文增强生成
- 🀄 **中文分词**: jieba-rs 智能分词，中英文混合检索优化
- 🛠️ **丰富工具**: 沙箱文件系统、Shell 白名单、Web 搜索、代码读写/审查/Git 操作、浏览器控制
- 📚 **深度研究**: 多轮自主搜索、信息源验证、知识图谱构建、结构化报告生成
- 🎭 **多助手系统**: 内置通用助手、自媒体助手、学习助手、搞钱助手，可自定义扩展
- 🧩 **技能系统**: TOML 定义技能插件（搜索/写作/爆款等），动态加载与选择
- 💬 **多界面**: TUI 终端 / Web 浏览器 / WhatsApp / 飞书 Lark / WebSocket 网关
- 🔌 **多模型支持**: DeepSeek / OpenAI / Claude / Gemini / Qwen / Kimi / GLM 无缝切换，多模型路由
- 🔄 **自我进化**: 代码质量分析 → 改进规划 → 自动执行 → Git 提交，支持调度与审批
- 📋 **任务管理**: 任务队列、调度器、优先级管理
- 📊 **可观测性**: Metrics 采集 + Prometheus 格式导出 + Tracing Spans
- 🔒 **安全沙箱**: 受限文件系统访问、Shell 命令白名单、域名白名单
- ⚡ **异步架构**: 全异步 I/O，支持 sqlx 异步 SQLite 持久化

---

## 🚀 快速开始

### 环境要求

- [Rust](https://rustup.rs/) 1.70+ (`rustup default stable`)
- DeepSeek 或 OpenAI API Key

### 安装运行

```bash
# 1. 克隆项目
git clone <repo-url>
cd bee

# 2. 设置 API Key（推荐 DeepSeek）
export DEEPSEEK_API_KEY=sk-xxx

# 3. 运行
cargo run
```

> 首次运行将自动创建 `workspace/` 目录和默认配置。

---

## 🖥️ 界面预览

### TUI 终端界面（默认）
```bash
cargo run              # 启动 TUI
cargo run --release    # 生产构建
```

**快捷键**:
| 快捷键 | 功能 |
|--------|------|
| `Enter` | 发送消息 |
| `Ctrl+C` | 取消当前生成 |
| `Ctrl+L` | 清空对话 |
| `Ctrl+Q` | 退出 |

### Web 界面
```bash
cargo run --bin bee-web --features web
```
访问 http://127.0.0.1:8080

### WhatsApp 集成
```bash
cargo run --bin bee-whatsapp --features whatsapp
```
> 需要公网 Webhook 回调域名（本地可用 ngrok）

### 飞书 Lark 集成
```bash
cargo run --bin bee-lark --features lark
```

### WebSocket 网关
```bash
cargo run --bin bee-gateway --features gateway
```
> Hub-Spoke 架构，支持多客户端并发连接、会话持久化、任务队列

---

## ⚙️ 配置

### 环境变量

| 变量 | 说明 | 优先级 |
|------|------|--------|
| `DEEPSEEK_API_KEY` | DeepSeek API Key | ⭐ 推荐 |
| `DEEPSEEK_MODEL` | `deepseek-chat`（默认）/ `deepseek-reasoner` | - |
| `OPENAI_API_KEY` | OpenAI API Key | 备选 |
| `ANTHROPIC_API_KEY` | Claude API Key | 可选 |
| `GOOGLE_API_KEY` | Gemini API Key | 可选 |
| `DASHSCOPE_API_KEY` | 通义千问 API Key | 可选 |
| `MOONSHOT_API_KEY` | Kimi API Key | 可选 |
| `ZHIPU_API_KEY` | GLM API Key | 可选 |
| （无需配置） | Mock LLM 离线模式 | 测试 |

**示例** - 使用 DeepSeek 思考模式：
```bash
export DEEPSEEK_API_KEY=sk-xxx
export DEEPSEEK_MODEL=deepseek-reasoner
cargo run
```

### 配置文件

| 文件 | 说明 |
|------|------|
| `config/default.toml` | 主配置（LLM、工具白名单、记忆、进化、安全等） |
| `config/models.toml` | 多模型注册（GPT-5.x / DeepSeek V3.2 / Claude 4.6 / Gemini 3 / Qwen 3.5 等） |
| `config/assistants.toml` | 多助手定义（通用助手、自媒体、学习、搞钱等） |
| `config/prompts/` | System Prompt 模板 |
| `config/skills/` | 技能插件定义（搜索、写作、爆款、Claude 风格等） |
| `workspace/` | 沙箱工作目录 |

### 多模型切换

编辑 `config/default.toml`:
```toml
[llm]
provider = "deepseek"  # 或 "openai"
model = "deepseek-reasoner"  # 或 "deepseek-chat"
```

或在 `config/models.toml` 中注册更多模型，Web 界面可动态切换。

### 自动模型路由

系统支持智能模型路由，根据任务类型自动选择最合适的模型：

**自动路由规则**：
- 简单问答（<20 字） → 快速模型（deepseek-chat）
- 代码相关（含编程关键词） → 代码模型
- 复杂推理（含分析/解释/为什么等关键词） → 推理模型
- 多问题（3+ 问号） → 推理模型
- 长内容（>500 字） → 推理模型
- 技术术语密集（3+ 术语） → 推理模型

**指令前缀覆盖**：
- `/think` 或 `/推理`：强制使用推理模型（deepseek-reasoner）
- `/fast` 或 `/快速`：强制使用快速模型（deepseek-chat）

**示例**：
```bash
# 简单问题（自动使用快速模型）
"今天天气怎么样？"

# 复杂推理（自动使用推理模型）
"分析一下这个算法的时间复杂度和空间复杂度"

# 强制使用推理模型
"/think 请详细解释量子计算的原理"

# 强制使用快速模型
"/fast 2+2 等于几"
```

---

## 🏗️ 架构

### 架构分层依赖图

```
┌─────────────────────────────────────────────────────────────┐
│                    可执行入口层 (bin/)                        │
│         bee (TUI) / bee-web / bee-lark / bee-gateway         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    用户界面层 (ui/)                           │
│         app / render / widgets / markdown / streaming        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    应用服务层 (application/)                  │
│    orchestrator / agent_service / task_queue / event_bus    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    核心编排层 (core/)                         │
│    builder / state / error / recovery / session_supervisor  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      领域层 (domain/)                         │
│    cognitive / tool / memory / session / event              │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   记忆实现层     │ │    工具箱层      │ │   ReAct 循环层    │
│   (memory/)     │ │   (tools/)      │ │   (react/)      │
└─────────────────┘ └─────────────────┘ └─────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   基础设施层 (infrastructure/)                │
│    memory / persistence / pool / session                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     LLM 适配层 (llm/)                         │
│    traits / openai / deepseek / mock / router               │
└─────────────────────────────────────────────────────────────┘
```

### 完整架构视图

```
┌───────────────────────────────────────────────────────────────┐
│                      交互层 (Interface)                       │
│   ┌───────┬──────────┬───────────┬────────┬──────────────┐   │
│   │ TUI   │ Web UI   │ WhatsApp  │ Lark   │   Gateway    │   │
│   │Ratatui│ Axum SSE │ Webhook   │Webhook │ WebSocket    │   │
│   └───┬───┴────┬─────┴─────┬─────┴───┬────┴──────┬───────┘   │
└───────┼────────┼───────────┼─────────┼──────────┼────────────┘
        │        │           │         │          │
        └────────┴─────┬─────┴─────────┘          │
                       ▼                          ▼
┌─────────────────────────────────┐  ┌─────────────────────────┐
│      Headless Agent Runtime     │  │    Gateway (Hub-Spoke)   │
│  create_agent → process_message │  │  Session + Task Queue    │
└───────────────┬─────────────────┘  └────────────┬────────────┘
                │                                  │
                └──────────────┬───────────────────┘
                               ▼
┌──────────────────────────────────────────────────────────────┐
│               核心编排 (Orchestrator)                         │
│  AgentBuilder + Session Supervisor + Recovery + Task Scheduler│
└──────────────────────────────────────────────────────────────┘
                               │
    ┌──────────────────────────┼──────────────────────────┐
    ▼                          ▼                          ▼
┌────────────────   ┌──────────────────┐   ┌────────────────────┐
│   认知层        │   │     工具层        │   │      记忆层         │
│ Planner        │   │ 沙箱文件系统      │   │ 对话记忆 (智能剪枝)   │
│ Critic         │   │ Shell 白名单      │   │ 中期工作区           │
│ ReAct Loop     │   │ 代码读写/审查     │   │ 长期记忆 (文件 + 向量)  │
│ (20 步上限)    │   │ Git Diff/Commit   │   │ 用户记忆             │
│                │   │ Web/深度搜索      │   │ 学习经验 (Learnings)  │
│                │   │ 知识图谱          │   │ RAG Pipeline         │
│                │   │ 报告生成器        │   │ Token Budget         │
└────────────────┘   └──────────────────┘   └────────────────────┘
    │                          │                          │
    ▼                          ▼                          ▼
┌────────────────┐   ┌──────────────────┐   ┌────────────────────┐
│   LLM 层       │   │   技能层          │   │     持久化层        │
│ 多模型路由     │   │ TOML 技能定义     │   │ SQLite (同步/异步)  │
│ 任务分类       │   │ 动态加载/选择     │   │ 文件 (异步 I/O)     │
│ 重试策略       │   │ 工具注册          │   │ 向量快照             │
│ Embedding      │   │                   │   │ Markdown Store       │
└────────────────┘   └──────────────────┘   └────────────────────┘
                               │
                               ▼
                   ┌──────────────────────┐
                   │    自我进化引擎       │
                   │ 分析 → 规划 → 执行   │
                   │ 调度 + 审批 + 回滚   │
                   └──────────────────────┘
```

### 关键组件

| 组件 | 说明 |
|------|------|
| **AgentBuilder** | 统一 Agent 组件构建，支持自定义 Prompt 和配置参数注入 |
| **多模型路由** | 根据任务类型（代码/推理/摘要）自动选择最优模型 |
| **RAG Pipeline** | 文档分块 → 向量存储 → 混合检索（向量 + 关键词 RRF）→ 上下文增强 |
| **智能剪枝** | 对话超长时保留系统消息，优先移除工具输出，Token Budget 管理 |
| **技能系统** | TOML 定义技能，SkillSelector 按任务选择，SkillLoader 动态注册工具 |
| **自我进化** | Analyzer 分析代码质量 → Planner 生成改进计划 → Executor 执行 → Git 提交 |
| **Gateway** | Hub-Spoke WebSocket 网关，会话持久化，任务队列，用户记忆管理 |
| **优雅关闭** | SIGINT/SIGTERM 信号处理，资源清理 |

---

## 📁 项目结构

```
bee/
├── src/
│   ├── main.rs            # TUI 入口
│   ├── lib.rs             # Library 导出
│   ├── bin/               # 附加二进制
│   │   ├── web.rs             # Web 服务器 (Axum SSE)
│   │   ├── whatsapp.rs        # WhatsApp Webhook
│   │   ├── lark.rs            # 飞书 Webhook
│   │   ├── gateway.rs         # WebSocket 网关
│   │   └── evolution_test.rs  # 进化引擎测试
│   ├── application/       # 应用服务层
│   │   ├── orchestrator.rs    # Agent 编排器
│   │   ├── agent_service.rs   # Agent 服务实现
│   │   ├── task_queue.rs      # 工作窃取任务队列
│   │   ├── event_bus.rs       # 应用事件总线
│   │   └── stream.rs          # 流式响应服务
│   ├── core/              # 核心编排层
│   │   ├── builder.rs         # Agent 组件构建器
│   │   ├── state.rs           # 状态投影 (UiState, AgentPhase)
│   │   ├── error.rs           # 核心错误定义
│   │   ├── recovery.rs        # 恢复引擎
│   │   ├── session_supervisor.rs  # 会话监管器
│   │   ├── task_scheduler.rs  # 任务调度器
│   │   └── shutdown.rs        # 优雅关闭协调器
│   ├── domain/            # 领域层（业务核心）
│   │   ├── cognitive/         # 认知领域 (Planner, Critic, ReAct)
│   │   ├── tool/              # 工具领域 (Tool, Registry, Executor)
│   │   ├── memory/            # 记忆领域 (Conversation, Working)
│   │   ├── session/           # 会话领域
│   │   └── event/             # 领域事件
│   ├── memory/            # 记忆实现层
│   │   ├── conversation.rs    # 对话记忆 (智能剪枝)
│   │   ├── working.rs         # 工作记忆
│   │   ├── long_term.rs       # 长期记忆 (文件 + 向量)
│   │   ├── user_memory.rs     # 用户记忆
│   │   ├── learnings.rs       # 学习经验
│   │   ├── rag.rs             # RAG Pipeline
│   │   ├── tokenizer.rs       # 中文分词 (jieba-rs)
│   │   ├── token_budget.rs    # Token 预算
│   │   ├── markdown_store.rs  # Markdown 存储
│   │   ├── persistence.rs     # SQLite 持久化
│   │   └── async_persistence.rs  # 异步 SQLite
│   ├── tools/             # 工具箱层 (30+ 工具)
│   │   ├── groups/              # 工具分组
│   │   │   ├── code.rs          # 代码工具组
│   │   │   ├── filesystem.rs    # 文件系统工具组
│   │   │   ├── git.rs           # Git 工具组
│   │   │   └── web.rs           # Web 工具组
│   │   ├── executor.rs      # 工具执行器
│   │   ├── registry.rs      # 工具注册表
│   │   └── [具体工具].rs    # 30+ 具体工具实现
│   ├── react/             # ReAct 认知循环层
│   │   ├── planner.rs         # 规划器
│   │   ├── critic.rs          # 批评器
│   │   ├── loop_.rs           # ReAct 主循环
│   │   ├── context.rs         # 上下文管理
│   │   └── events.rs          # ReAct 事件
│   ├── infrastructure/    # 基础设施层
│   │   ├── memory/              # 内存存储
│   │   ├── persistence/         # 持久化 (细粒度锁)
│   │   ├── pool/                # 连接池 (SQLite, HTTP)
│   │   └── session/             # 会话存储
│   ├── llm/               # LLM 适配层
│   │   ├── traits.rs            # LLM trait
│   │   ├── openai.rs            # OpenAI 客户端
│   │   ├── deepseek.rs          # DeepSeek 客户端
│   │   ├── mock.rs              # Mock 客户端
│   │   ├── router.rs            # LLM 路由
│   │   └── embedding.rs         # Embedding
│   ├── ui/                # 用户界面层
│   │   ├── app.rs               # TUI 主循环
│   │   ├── render.rs            # 渲染逻辑
│   │   ├── event.rs             # 事件处理
│   │   ├── theme.rs             # 主题定义
│   │   ├── widgets/             # UI 组件
│   │   ├── markdown/            # Markdown 渲染
│   │   └── streaming/           # 流式输出
│   ├── plugins/           # 插件系统层
│   │   └── loader.rs            # 插件加载器
│   ├── skills/            # 技能系统层
│   ├── workflow/          # 工作流引擎层
│   ├── messaging/         # 消息通道层
│   ├── integrations/      # 外部集成层
│   ├── observability/     # 可观测性层
│   ├── gateway/           # 网关层 (WebSocket)
│   ├── saas/              # 多租户 SaaS 层
│   ├── service_contracts/ # 服务契约层
│   ├── evolution/         # 自我进化层
│   ├── container/         # 依赖注入容器层
│   ├── config/            # 配置加载层
│   ├── test_utils/        # 测试工具层
│   └── tool_policy/       # 工具策略层
├── config/                # 配置文件与模板
│   ├── default.toml           # 主配置
│   ├── models.toml            # 模型注册
│   ├── assistants.toml        # 助手定义
│   ├── alerts.yml             # 告警系统配置
│   ├── prompts/               # System Prompt 模板
│   └── skills/                # 技能插件定义
├── dashboards/            # Grafana 仪表板
│   ├── bee-overview.json      # 系统概览
│   └── bee-business.json      # 业务指标
├── plugins/               # 示例插件
│   ├── code-analyzer/         # 代码分析器
│   ├── doc-generator/         # 文档生成器
│   └── test-generator/        # 测试生成器
├── benches/               # 性能基准测试
│   ├── memory_store_bench.rs
│   └── session_store_bench.rs
├── tests/                 # 集成测试
│   ├── error_recovery_test.rs
│   ├── memory_persistence_test.rs
│   ├── multi_session_test.rs
│   ├── react_loop_test.rs
│   ├── tool_execution_test.rs
│   └── ...
└── docs/                  # 项目文档
```

---

## 🔬 技术亮点

### RAG 检索增强生成

```rust
// UTF-8 安全的文档分块
let chunker = Chunker::new(ChunkingConfig {
    chunk_size: 500,
    overlap: 50,
    ..Default::default()
});
let chunks = chunker.chunk("doc_id", "你的文档内容...");

// 混合检索（向量 + 关键词，RRF 融合）
let results = vector_store.hybrid_search(&query_embedding, "关键词", 10);
```

### 多模型智能路由

```rust
// 根据任务类型自动选择模型
let router = ModelRouter::new()
    .add_model("gpt-4", gpt4_client, ModelCapabilities { code_score: 0.9, .. })
    .add_model("deepseek", deepseek_client, ModelCapabilities { reasoning_score: 0.95, .. });

// 代码任务 → GPT-4，推理任务 → DeepSeek
let client = RoutingLlmClient::new(router, RoutingStrategy::BestQuality);
```

### 自我进化引擎

```rust
// 自主迭代：分析 → 规划 → 执行 → 提交
let config = EvolutionConfig {
    max_iterations: 10,
    target_score_threshold: 0.8,
    auto_commit: true,
    focus_areas: vec!["performance", "readability", "testing"],
    safe_mode: SafeMode::Strict,
    ..Default::default()
};
let evolution = EvolutionLoop::new(config);
evolution.run().await?;
```

### 智能对话剪枝

```rust
// 保留系统消息，优先移除工具输出
let config = PruneConfig {
    preserve_system: true,
    tool_result_ratio: 0.3,  // 工具结果最多占 30%
    smart_prune: true,
};
let pruned = conversation.prune();  // 返回被移除的消息供长期记忆使用
```

### 细粒度锁持久化

```rust
// 基于键的细粒度读写锁，支持高并发
let store: FineGrainedLockStore<String, Session> = FineGrainedLockStore::new();
store.upsert("session_1".to_string(), session).await?;
```

### 连接池管理

```rust
// SQLite 连接池，信号量控制并发
let pool = SqliteConnectionPool::in_memory()?;
let guard = pool.get().await.unwrap();
guard.execute(|conn| conn.execute("CREATE TABLE ...", [])?)?;
```

---

## 📚 文档

### 项目文档
- [📖 使用文档](docs/使用文档.md) - 详细使用指南
- [🌐 Web UI 文档](docs/WEBUI.md) - Web 界面配置
- [💬 WhatsApp 文档](docs/WHATSAPP.md) - WhatsApp 集成指南
- [🔗 Lark 飞书文档](docs/LARK.md) - 飞书机器人集成
- [🌉 网关文档](docs/GATEWAY.md) - WebSocket 网关
- [📑 文档导航](docs/README.md) - 完整文档索引

### 架构与设计
- [🏗️ 架构改进](docs/ARCHITECTURE_IMPROVEMENTS.md) - 四阶段架构演进计划
- [🔍 架构分析](docs/ARCHITECTURE_ANALYSIS.md) - 架构分析报告
- [📐 架构白皮书](docs/Rust 个人智能体系统 (Bee)-架构设计白皮书.md) - 系统设计白皮书

### 功能模块
- [📚 深度研究](docs/DEEP_RESEARCH.md) - 深度研究功能完整指南
- [🚀 深度研究快速开始](docs/DEEP_RESEARCH_QUICKSTART.md) - 快速上手
- [🧠 记忆系统](docs/MEMORY.md) - 分层记忆架构
- [📖 学习经验](docs/LEARNINGS.md) - 从失败中学习
- [🧩 技能系统](docs/SKILLS.md) - 技能插件开发指南
- [🔄 自我进化](docs/EVOLUTION.md) - 自我进化机制
- [🔄 进化设计](docs/EVOLUTION_DESIGN.md) - 进化引擎设计文档

### AI 行为改进
- [🎯 AI 改进计划](docs/ai-improvement-plan.md) - 改进路线图
- [📊 AI 改进跟踪](docs/ai-improvement-tracking.md) - 改进进度跟踪
- [⚡ 快速参考](docs/ai-quick-reference.md) - 日常交互速查卡
- [✅ 自检清单](docs/ai-self-check-workflow.md) - 可执行检查清单

---

## 🛠️ 开发

```bash
# 开发模式运行
cargo run

# 生产构建
cargo build --release

# 运行测试（240 个测试）
cargo test

# 代码检查
cargo clippy
cargo fmt
```

### 功能开关

```bash
# Web 界面
cargo run --bin bee-web --features web

# WhatsApp 集成
cargo run --bin bee-whatsapp --features whatsapp

# 飞书/Lark 集成
cargo run --bin bee-lark --features lark

# WebSocket 网关（含异步 SQLite）
cargo run --bin bee-gateway --features gateway

# 浏览器控制（需安装 Chrome/Chromium）
cargo run --features browser

# 异步 SQLite 持久化
cargo build --features async-sqlite

# 进化引擎测试
cargo run --bin bee-evolution
```

### 核心模块

| 模块 | 说明 |
|------|------|
| `core::builder` | AgentBuilder 统一组件构建 |
| `core::task_scheduler` | 任务调度与队列管理 |
| `core::session_supervisor` | 会话生命周期监管 |
| `llm::router` | 多模型路由，按任务类型自动选择模型 |
| `llm::embedding` | 向量化嵌入（Embedding API） |
| `memory::rag` | RAG Pipeline，文档分块 + 混合检索 |
| `memory::long_term` | 长期记忆（文件存储 + 向量检索） |
| `memory::user_memory` | 用户个性化记忆管理 |
| `memory::tokenizer` | 中英文智能分词（jieba-rs） |
| `memory::token_budget` | Token 预算与上下文窗口管理 |
| `skills` | 技能定义、加载与自动选择 |
| `evolution` | 自我进化引擎（分析/规划/执行） |
| `gateway` | Hub-Spoke WebSocket 网关 |
| `observability` | Metrics + Tracing Spans |

---

## 🔒 安全特性

- **沙箱文件系统**: 只能访问 `workspace/` 目录
- **Shell 白名单**: 仅允许配置的命令（默认：ls, grep, cat, head, tail, wc, find, cargo, rustc）
- **域名白名单**: Web 搜索限制在允许域名内
- **进化安全**: strict/balanced/permissive 三级安全模式，支持回滚与备份
- **API Key 隔离**: 环境变量管理，不写入配置文件
- **细粒度锁**: 基于键的读写锁，避免全局锁竞争

---

##  贡献

欢迎 Issue 和 PR！

1. Fork 项目
2. 创建分支 (`git checkout -b feature/amazing`)
3. 提交更改 (`git commit -m 'Add feature'`)
4. 推送分支 (`git push origin feature/amazing`)
5. 创建 Pull Request

---

## 📄 许可证

[MIT](LICENSE) © Bee Team

---

<div align="center">
  <sub>Built with 🦀 Rust</sub>
</div>
