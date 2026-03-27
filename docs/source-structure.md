# Bee 项目源码结构

本文档列出了 `src` 目录下所有 Rust 源文件的组织结构。

## 源码统计

- **总文件数**: 195 个 `.rs` 文件

## 目录结构

### 核心架构层

| 目录 | 文件数 | 说明 |
|------|--------|------|
| `src/` | 8 | 根目录文件 (lib, main, config, container 等) |
| `src/application/` | 7 | 应用层服务 (agent_service, orchestrator, event_bus 等) |
| `src/core/` | 9 | 核心运行时 (builder, session_supervisor, recovery, state 等) |
| `src/domain/` | 17 | 领域模型 (cognitive, event, memory, session, tool) |
| `src/infrastructure/` | 14 | 基础设施层 (llm, memory, persistence, session, pool) |

### 功能模块

| 目录 | 文件数 | 说明 |
|------|--------|------|
| `src/bin/` | 35 | 可执行文件入口 (web, gateway, whatsapp, lark, evolution) |
| `src/evolution/` | 6 | 自进化引擎 (analyzer, planner, executor, engine, loop) |
| `src/gateway/` | 10 | WebSocket 网关 (hub, spoke, session, task_queue, intent) |
| `src/integrations/` | 3 | 第三方集成 (whatsapp, lark) |
| `src/llm/` | 7 | LLM 客户端 (deepseek, openai, router, embedding, traits) |
| `src/memory/` | 13 | 记忆系统 (conversation, working, long_term, rag, user_memory 等) |
| `src/react/` | 6 | ReAct 循环 (planner, critic, loop, events) |
| `src/skills/` | 3 | 技能系统 (loader, selector) |
| `src/tools/` | 40 | 工具集 (filesystem, shell, code_*, git_*, browser 等) |
| `src/workflow/` | 5 | 工作流引擎 (builder, engine, graph, types) |

### 支撑模块

| 目录 | 文件数 | 说明 |
|------|--------|------|
| `src/messaging/` | 3 | 消息系统 (channels, messages) |
| `src/observability/` | 1 | 可观测性 (metrics, tracing) |
| `src/plugins/` | 2 | 插件系统 (loader) |
| `src/saas/` | 13 | SaaS 服务 (auth, audit, bootstrap, repository, services) |
| `src/service_contracts/` | 1 | 服务契约 |
| `src/test_utils/` | 7 | 测试工具 (mocks, fixtures, assertions, harness) |
| `src/ui/` | 20 | TUI 界面 (ratatui widgets, markdown, streaming, theme) |

---

## 详细文件列表

### 根目录 (`src/`)
- `agent.rs` - Headless Agent 运行时
- `config.rs` - 配置加载
- `container.rs` - 依赖容器
- `lib.rs` - 库导出
- `main.rs` - TUI 入口
- `tool_policy.rs` - 工具策略
- `tool_router.rs` - 工具路由

### Application (`src/application/`)
- `agent_service.rs` - Agent 服务
- `event_bus.rs` - 事件总线
- `health.rs` - 健康检查
- `mod.rs` - 模块导出
- `orchestrator.rs` - 编排器
- `stream.rs` - 流式处理
- `task_queue.rs` - 任务队列

### Core (`src/core/`)
- `builder.rs` - Agent 构建器
- `error.rs` - 错误定义
- `mod.rs` - 模块导出
- `recovery.rs` - 恢复机制
- `session_supervisor.rs` - Session 监督器
- `shutdown.rs` - 关闭处理
- `state.rs` - 状态管理
- `task_scheduler.rs` - 任务调度

### Domain (`src/domain/`)
#### Cognitive (`domain/cognitive/`)
- `context.rs` - 上下文
- `critic.rs` - 批评者
- `memory.rs` - 记忆
- `mod.rs` - 模块导出
- `planner.rs` - 规划器
- `react.rs` - ReAct 循环

#### Event (`domain/event/`)
- `bus.rs` - 事件总线
- `events.rs` - 事件定义
- `mod.rs` - 模块导出

#### Memory (`domain/memory/`)
- `conversation.rs` - 会话记忆
- `mod.rs` - 模块导出
- `store.rs` - 存储
- `working.rs` - 工作记忆

#### Session (`domain/session/`)
- `mod.rs` - 模块导出
- `session.rs` - Session 定义
- `store.rs` - 存储

#### Tool (`domain/tool/`)
- `composite.rs` - 复合工具
- `executor.rs` - 执行器
- `group.rs` - 工具组
- `metadata.rs` - 元数据
- `mod.rs` - 模块导出
- `policy.rs` - 策略
- `registry.rs` - 注册表
- `trait_.rs` - 工具特征

### Infrastructure (`src/infrastructure/`)
- `llm.rs` - LLM 基础设施
- `mod.rs` - 模块导出

#### Memory (`infrastructure/memory/`)
- `file_store.rs` - 文件存储
- `in_memory_store.rs` - 内存存储
- `mod.rs` - 模块导出
- `sqlite_store.rs` - SQLite 存储

#### Persistence (`infrastructure/persistence/`)
- `locking.rs` - 锁机制
- `mod.rs` - 模块导出

#### Pool (`infrastructure/pool/`)
- `http.rs` - HTTP 连接池
- `mod.rs` - 模块导出
- `sqlite.rs` - SQLite 连接池

#### Session (`infrastructure/session/`)
- `mod.rs` - 模块导出
- `sqlite_store.rs` - SQLite Session 存储

### Bin (`src/bin/`)
- `evolution_test.rs` - 进化测试
- `gateway.rs` - 网关入口
- `lark.rs` - 飞书入口
- `web.rs` - Web 服务入口

#### Web (`bin/web/`)
- `assistant_catalog.rs` - 助手目录
- `dynamic_agent_catalog.rs` - 动态 Agent 目录
- `inbox_service.rs` - 收件箱服务
- `session_store.rs` - Session 存储
- `task_coordinator_service.rs` - 任务协调服务
- `task_service.rs` - 任务服务
- `workflow_product_service.rs` - 工作流产品服务

##### Handlers (`bin/web/handlers/`)
- `agents.rs` - Agents 处理器
- `chat.rs` - Chat 处理器
- `mod.rs` - 模块导出
- `sessions.rs` - Sessions 处理器
- `tools.rs` - Tools 处理器

##### Middleware (`bin/web/middleware/`)
- `auth.rs` - 认证中间件
- `cors.rs` - CORS 中间件
- `error.rs` - 错误中间件
- `logging.rs` - 日志中间件
- `mod.rs` - 模块导出
- `rate_limit.rs` - 限流中间件

##### Routes (`bin/web/routes/`)
- `agents.rs` - Agents 路由
- `chat.rs` - Chat 路由
- `health.rs` - Health 路由
- `mod.rs` - 模块导出
- `sessions.rs` - Sessions 路由
- `tools.rs` - Tools 路由

- `whatsapp.rs` - WhatsApp 入口

### Evolution (`src/evolution/`)
- `analyzer.rs` - 分析器
- `engine.rs` - 进化引擎
- `executor.rs` - 执行器
- `loop_.rs` - 进化循环
- `mod.rs` - 模块导出
- `planner.rs` - 进化规划器
- `types.rs` - 类型定义

### Gateway (`src/gateway/`)
- `hub.rs` - Hub 中心
- `intent.rs` - 意图识别
- `message.rs` - 消息
- `mod.rs` - 模块导出
- `persistent_session.rs` - 持久化 Session
- `runtime.rs` - 运行时
- `session.rs` - Session
- `session_store.rs` - Session 存储
- `spoke.rs` - Spoke 节点
- `task_queue.rs` - 任务队列

### Integrations (`src/integrations/`)
- `lark.rs` - 飞书集成
- `mod.rs` - 模块导出
- `whatsapp.rs` - WhatsApp 集成

### LLM (`src/llm/`)
- `deepseek.rs` - DeepSeek 客户端
- `embedding.rs` - Embedding
- `mock.rs` - Mock LLM
- `mod.rs` - 模块导出
- `openai.rs` - OpenAI 客户端
- `router.rs` - 模型路由
- `traits.rs` - LLM 特征

### Memory (`src/memory/`)
- `async_io.rs` - 异步 IO
- `async_persistence.rs` - 异步持久化
- `conversation.rs` - 会话记忆
- `learnings.rs` - 学习记录
- `long_term.rs` - 长期记忆
- `markdown_store.rs` - Markdown 存储
- `mod.rs` - 模块导出
- `persistence.rs` - 持久化
- `rag.rs` - RAG 检索增强
- `token_budget.rs` - Token 预算
- `tokenizer.rs` - Token 化
- `user_memory.rs` - 用户记忆
- `working.rs` - 工作记忆

### React (`src/react/`)
- `critic.rs` - 批评者
- `events.rs` - 事件
- `loop_.rs` - ReAct 循环
- `memory.rs` - 记忆
- `mod.rs` - 模块导出
- `planner.rs` - 规划器

### SaaS (`src/saas/`)
- `audit_service.rs` - 审计服务
- `auth_service.rs` - 认证服务
- `bootstrap.rs` - 启动引导
- `bootstrap_service.rs` - 启动服务
- `migration.rs` - 数据库迁移
- `mod.rs` - 模块导出
- `models.rs` - 数据模型
- `repository.rs` - 仓库接口
- `sqlite.rs` - SQLite 基础
- `sqlite_seed_repository.rs` - SQLite 种子仓库
- `sqlite_template_repository.rs` - SQLite 模板仓库
- `template_catalog.rs` - 模板目录
- `template_instantiation_service.rs` - 模板实例化服务
- `tool_policy_service.rs` - 工具策略服务

### Service Contracts (`src/service_contracts/`)
- `mod.rs` - 模块导出

### Skills (`src/skills/`)
- `loader.rs` - 加载器
- `mod.rs` - 模块导出
- `selector.rs` - 选择器

### Test Utils (`src/test_utils/`)
- `assertions.rs` - 断言工具
- `fixtures.rs` - 测试夹具
- `mod.rs` - 模块导出
- `test_harness.rs` - 测试框架

#### Mocks (`test_utils/mocks/`)
- `llm.rs` - LLM Mock
- `memory.rs` - Memory Mock
- `mod.rs` - 模块导出
- `tool.rs` - Tool Mock

### Tools (`src/tools/`)
- `browser.rs` - 浏览器工具
- `code_edit.rs` - 代码编辑
- `code_grep.rs` - 代码搜索
- `code_read.rs` - 代码阅读
- `code_review.rs` - 代码审查
- `code_write.rs` - 代码写入
- `create.rs` - 创建工具
- `create_group.rs` - 创建组
- `deep_search.rs` - 深度搜索
- `echo.rs` - Echo 工具
- `exchange_rate.rs` - 汇率查询
- `executor.rs` - 执行器
- `filesystem.rs` - 文件系统
- `git_commit.rs` - Git 提交
- `git_diff.rs` - Git 差异
- `github_repo_inspect.rs` - GitHub 仓库检查
- `knowledge_graph.rs` - 知识图谱
- `list_agents.rs` - Agent 列表
- `market_quote.rs` - 市场行情
- `metadata.rs` - 元数据
- `mod.rs` - 模块导出
- `news.rs` - 新闻
- `output.rs` - 输出工具
- `plugin.rs` - 插件工具
- `registry.rs` - 注册表
- `report_generator.rs` - 报告生成
- `schema.rs` - Schema
- `search.rs` - 搜索
- `send.rs` - 发送工具
- `shell.rs` - Shell 命令
- `source_adapter.rs` - 源适配器
- `source_validator.rs` - 源验证器
- `sports_score.rs` - 体育比分
- `test_check.rs` - 测试检查
- `test_run.rs` - 测试运行
- `weather.rs` - 天气

#### Groups (`tools/groups/`)
- `code.rs` - 代码工具组
- `filesystem.rs` - 文件工具组
- `git.rs` - Git 工具组
- `mod.rs` - 模块导出
- `web.rs` - Web 工具组

### UI (`src/ui/`)
- `app.rs` - 应用
- `event.rs` - 事件
- `mod.rs` - 模块导出
- `render.rs` - 渲染
- `theme.rs` - 主题

#### Markdown (`ui/markdown/`)
- `highlight.rs` - 高亮
- `mod.rs` - 模块导出
- `renderer.rs` - 渲染器

#### Streaming (`ui/streaming/`)
- `collector.rs` - 收集器
- `controller.rs` - 控制器
- `mod.rs` - 模块导出
- `state.rs` - 状态

#### Widgets (`ui/widgets/`)
- `activity_rail.rs` - 活动栏
- `command_popup.rs` - 命令弹窗
- `conversation.rs` - 对话
- `file_popup.rs` - 文件弹窗
- `input.rs` - 输入框
- `input_history.rs` - 输入历史
- `mod.rs` - 模块导出
- `renderable.rs` - 可渲染
- `status_indicator.rs` - 状态指示器
- `textarea.rs` - 文本域

### Workflow (`src/workflow/`)
- `builder.rs` - 构建器
- `engine.rs` - 引擎
- `graph.rs` - 图
- `mod.rs` - 模块导出
- `types.rs` - 类型定义

### Messaging (`src/messaging/`)
- `channels.rs` - 通道
- `messages.rs` - 消息
- `mod.rs` - 模块导出

### Observability (`src/observability/`)
- `mod.rs` - 模块导出

### Plugins (`src/plugins/`)
- `loader.rs` - 加载器
- `mod.rs` - 模块导出

---

## 架构分层总结

```
┌─────────────────────────────────────────────────────────────┐
│                    Interface Layer                           │
│  TUI (ui/)  │  Web (bin/web/)  │  Gateway (gateway/)       │
│  WhatsApp/Lark (integrations/)                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                 Application Layer                            │
│  application/, core/, domain/                               │
└─────────────────────────────────────────────────────────────┘
                              │
    ┌─────────────────────────┼─────────────────────────┐
    ▼                         ▼                         ▼
┌────────────────┐  ┌──────────────────┐  ┌────────────────────┐
│  Cognitive     │  │     Tool         │  │     Memory         │
│  (react/)      │  │  (tools/)        │  │  (memory/)         │
│  Planner       │  │  Sandbox FS      │  │  Conversation      │
│  Critic        │  │  Code R/W/Edit   │  │  Working/Long-term │
│  ReAct Loop    │  │  Git/Web/Search  │  │  RAG/User          │
└────────────────┘  └──────────────────┘  └────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                 Infrastructure Layer                         │
│  LLM (llm/) │ Skills (skills/) │ SaaS (saas/)              │
│  Evolution (evolution/) │ Workflow (workflow/)              │
└─────────────────────────────────────────────────────────────┘
```
