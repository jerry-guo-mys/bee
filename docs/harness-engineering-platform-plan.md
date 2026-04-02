# Harness Engineering 平台规划文档

> **文档版本**: 0.1  
> **创建日期**: 2026-04-02  
> **状态**: 产品调研与架构设计阶段

---

## 一、产品调研与市场背景

### 1.1 行业趋势：从 Agents 到 Harnesses

根据 2025-2026 年的行业研究和实践，AI Agent 领域正在经历重大范式转变：

| 阶段 | 时间 | 核心关注点 | 局限性 |
|------|------|-----------|--------|
| **Prompt Engineering** | 2022-2024 | 单次交互的提示词优化 | 无法处理长程任务，缺乏可靠性 |
| **Context Engineering** | 2025 | 上下文管理与检索增强 | 仍局限于单次对话，缺乏多 Agent 协作 |
| **Harness Engineering** | 2026 | 长程任务的可靠执行与多 Agent 编排 | 解决复杂任务的端到端交付 |

### 1.2 什么是 Agent Harness？

**Agent Harness（智能体框架）** 是围绕 AI 模型构建的基础设施层，用于可靠地执行长程、复杂任务。核心职责包括：

```
┌─────────────────────────────────────────────────────────┐
│                    Agent Harness                         │
├─────────────────────────────────────────────────────────┤
│  • Human-in-the-loop 工具调用管理                        │
│  • 子智能体编排（Sub-agent Orchestration）              │
│  • 上下文生命周期管理                                    │
│  • 工具定义与输出验证                                    │
│  • 失败恢复与重试机制                                    │
│  • 多步骤任务编排                                        │
│  • 进度跟踪与状态持久化                                  │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
              ┌─────────────────────────┐
              │    AI Model (LLM)       │
              │  (只负责生成响应)        │
              └─────────────────────────┘
```

### 1.3 行业参考架构

根据调研，主流的 Harness Engineering 架构包含以下核心层：

```
┌─────────────────────────────────────────────────────────────┐
│                    Interface Layer                           │
│  (Web Dashboard, API, CLI, IDE Plugin, Slack/Discord)       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Gating Layer                              │
│  (请求路由、权限控制、速率限制、成本管控)                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Orchestration Layer                       │
│  (DAG 工作流引擎、多 Agent 协作、任务依赖管理)               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Knowledge Layer                           │
│  (向量数据库、知识图谱、长期记忆、RAG 管道)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Governance Layer                          │
│  (审计日志、合规检查、安全策略、输出验证)                    │
└─────────────────────────────────────────────────────────────┘
```

### 1.4 竞品分析

| 产品 | 定位 | 核心能力 | 差异化机会 |
|------|------|----------|------------|
| **Cline** | 日常编码助手 | 本地控制、编辑器原生、模型选择灵活 | 更专注于长程任务和多 Agent 协作 |
| **Augment Code** | 复杂生产级开发 | 深度上下文感知、规范驱动的 AI 代码生成 | 提供更灵活的 Agent 人格定义 |
| **LangGraph** | 图式多 Agent 编排 | 基于图的 workflow、对话循环 | 更强调工程化和企业级特性 |
| **Microsoft AutoGen** | 多 Agent 协作 | 任务委派、自主执行 | 更注重开发者体验和可视化 |

---

## 二、Bee 系统的现状与能力评估

### 2.1 现有架构概览

Bee 系统当前已具备以下核心能力：

```
┌─────────────────────────────────────────────────────────┐
│                    Interface Layer                       │
│  TUI(Ratatui) │ Web(Axum SSE) │ WhatsApp │ Lark │ ...  │
└─────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────┐
│                    Headless Agent Runtime                │
│  create_agent() → process_message()                      │
└─────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────┐
│                    Core Orchestrator                     │
│  AgentBuilder │ Session Supervisor │ Recovery           │
└─────────────────────────────────────────────────────────┘
                              │
    ┌─────────────────────────┼─────────────────────────┐
    ▼                         ▼                         ▼
┌────────────────┐  ┌──────────────────┐  ┌────────────────────┐
│  Cognitive     │  │     Tool         │  │     Memory         │
│  Planner       │  │  Sandbox FS      │  │  Short-term(conv)  │
│  Critic        │  │  Shell whitelist │  │  Mid-term(workspace)│
│  ReAct Loop    │  │  Code R/W/Edit   │  │  Long-term(file+vector)│
│  (20 steps)    │  │  Git/Diff/Commit │  │  User memory        │
│                │  │  Web/DeepSearch  │  │  Learnings          │
└────────────────┘  └──────────────────┘  └────────────────────┘
```

### 2.2 现有能力清单

| 模块 | 能力 | 状态 | 文件位置 |
|------|------|------|----------|
| **Agent 构建** | AgentBuilder 统一构建模式 | ✅ 已实现 | `src/core/builder.rs` |
| **技能系统** | TOML 定义的技能加载与选择 | ✅ 已实现 | `src/skills/` |
| **记忆系统** | 短/中/长期记忆，向量检索 | ✅ 已实现 | `src/memory/` |
| **工作流引擎** | DAG 基础、任务依赖 | ⚠️ 基础版本 | `src/workflow/` |
| **多模型路由** | DeepSeek/OpenAI/Claude 等 | ✅ 已实现 | `src/llm/` |
| **工具系统** | 27+ 工具（FS、Shell、Git、Web） | ✅ 已实现 | `src/tools/` |
| **自进化引擎** | 分析→计划→执行→提交 | ✅ 已实现 | `src/evolution/` |
| **网关系统** | WebSocket 枢纽、会话存储 | ✅ 已实现 | `src/gateway/` |

### 2.3 能力差距分析 (Gap Analysis)

| 目标能力 | 当前状态 | 差距 | 优先级 |
|----------|----------|------|--------|
| **长程任务支持** | ReAct 循环限制 20 步 | 需要支持>100 步的任务分解与恢复 | P0 |
| **多 Agent 协作** | 单 Agent 架构 | 需要 Agent 间通信与任务委派机制 | P0 |
| **Agent 人格定义** | 无 | 需要人格模板与行为约束系统 | P1 |
| **工作流程编排** | 基础 DAG | 需要可视化编排、条件分支、并行执行 | P1 |
| **团队组装流程** | 无 | 需要团队级别的流程定义与版本管理 | P2 |
| **可观测性** | 基础 Metrics | 需要分布式追踪、Agent 行为审计 | P2 |

---

## 三、平台架构设计

### 3.1 总体架构

基于 Harness Engineering 理念，重新设计 Bee 系统架构：

```
┌───────────────────────────────────────────────────────────────────────┐
│                         Interface Layer                                │
│  ┌─────────┐  ┌──────────┐  ┌────────┐  ┌────────┐  ┌────────────┐   │
│  │   TUI   │  │   Web    │  │  CLI   │  │  API   │  │  IDE Plugin│   │
│  └─────────┘  └──────────┘  └────────┘  └────────┘  └────────────┘   │
└───────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌───────────────────────────────────────────────────────────────────────┐
│                         Gating Layer                                   │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────────┐   │
│  │   Auth & RBAC    │  │  Rate Limiting   │  │   Cost Management  │   │
│  └──────────────────┘  └──────────────────┘  └────────────────────┘   │
│  ┌──────────────────┐  ┌──────────────────┐                          │
│  │   Request Router │  │   Safety Filter  │                          │
│  └──────────────────┘  └──────────────────┘                          │
└───────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌───────────────────────────────────────────────────────────────────────┐
│                      Orchestration Layer                               │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │                    Harness Controller                            │  │
│  │  • Task Decomposition  • Sub-agent Spawning                     │  │
│  │  • Progress Tracking   • Recovery & Retry                       │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │                    Workflow Engine (DAG)                         │  │
│  │  • Sequential  • Parallel  • Conditional  • Fan-out/Fan-in      │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │                    Multi-Agent Supervisor                        │  │
│  │  • Agent Registry  • Message Bus  • Conflict Resolution         │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌───────────────────────────────────────────────────────────────────────┐
│                      Agent Layer                                       │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │                    Agent Instance                                │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │  │
│  │  │  Persona    │  │   Skills    │  │      Context Manager    │  │  │
│  │  │  (人格定义)  │  │  (技能组合)  │  │   (短/中/长期记忆)      │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘  │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │  │
│  │  │   Planner   │  │   Critic    │  │      Tool Executor      │  │  │
│  │  │  (ReAct)    │  │  (可选)     │  │   (27+ 内置工具)        │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │                    Agent Types                                   │  │
│  │  • Generalist  • Coder  • Researcher  • Reviewer  • Specialist  │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌───────────────────────────────────────────────────────────────────────┐
│                      Knowledge Layer                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                │
│  │ Vector Store │  │   Graph DB   │  │  File Store  │                │
│  │  (Chroma/    │  │  (Neo4j/     │  │  (Markdown/  │                │
│  │   Qdrant)    │  │   Memgraph)  │  │    SQLite)   │                │
│  └──────────────┘  └──────────────┘  └──────────────┘                │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │                    RAG Pipeline                                  │  │
│  │  • Chunking  • Embedding  • Hybrid Search  • Context Injection  │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌───────────────────────────────────────────────────────────────────────┐
│                      Foundation Layer                                  │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │  Multi-Model Router (DeepSeek/OpenAI/Claude/Gemini/Qwen)        │  │
│  └─────────────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │  Persistence (SQLite + File-based + Vector Snapshots)           │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
```

### 3.2 核心概念定义

#### 3.2.1 Agent 人格 (Persona)

```toml
# config/personas/senior_engineer.toml
[persona]
id = "senior_engineer"
name = "高级软件工程师"
description = "负责代码审查、架构设计和复杂功能实现"

[persona.trait]
# 大五人格 (OCEAN)
openness = 0.8           # 开放性：愿意尝试新方法
conscientiousness = 0.9  # 尽责性：注重细节和质量
extraversion = 0.4       # 外向性：偏内向，专注深度工作
agreeableness = 0.7      # 宜人性：愿意协作但坚持标准
neuroticism = 0.2        # 神经质：情绪稳定

[persona.communication]
tone = "professional"         # 专业、简洁
verbosity = "concise"         # 简洁，不啰嗦
formality = 0.8              # 正式程度
use_examples = true          # 倾向于用代码示例说明

[persona.expertise]
domains = ["rust", "distributed_systems", "api_design"]
experience_years = 10
preferred_patterns = ["builder", "repository", "event_sourcing"]

[persona.constraints]
max_step_budget = 50        # 单次任务最多 50 步
requires_tests = true       # 必须编写测试
requires_review = true      # 必须代码审查
```

#### 3.2.2 Agent 技能 (Skill)

```toml
# config/skills/code_review/skill.toml
[skill]
id = "code_review"
name = "代码审查"
description = "系统化的代码审查能力，包括安全检查、性能分析和代码质量评估"
version = "1.0"

[skill.triggers]
keywords = ["review", "CR", "pull request", "code quality"]
patterns = ["*.rs", "*.ts", "*.py"]

[[skill.capabilities]]
name = "security_check"
description = "OWASP Top 10 安全检查"
prompt_template = "config/skills/code_review/security_prompt.md"

[[skill.capabilities]]
name = "performance_analysis"
description = "性能热点识别"
prompt_template = "config/skills/code_review/performance_prompt.md"

[[skill.capabilities]]
name = "style_check"
description = "代码风格和规范检查"
prompt_template = "config/skills/code_review/style_prompt.md"
```

#### 3.2.3 工作流 (Workflow)

```toml
# config/workflows/code_review.toml
[workflow]
id = "code_review_pipeline"
name = "代码审查流水线"
description = "多 Agent 协作的代码审查流程"

# 定义参与的角色
[[workflow.roles]]
role_id = "reviewer"
persona = "senior_engineer"
skills = ["code_review", "security_check"]

[[workflow.roles]]
role_id = "tester"
persona = "qa_engineer"
skills = ["test_generation", "edge_case_analysis"]

# 定义任务流程
[[workflow.tasks]]
task_id = "initial_review"
role = "reviewer"
instruction = "对变更的代码进行初步审查，识别潜在问题"

[[workflow.tasks]]
task_id = "security_scan"
role = "reviewer"
instruction = "执行安全检查，识别 OWASP 漏洞"
depends_on = [{ task = "initial_review", type = "sequential" }]

[[workflow.tasks]]
task_id = "test_generation"
role = "tester"
instruction = "为变更生成测试用例"
depends_on = [{ task = "initial_review", type = "sequential" }]

[[workflow.tasks]]
task_id = "final_report"
role = "reviewer"
instruction = "汇总所有发现，生成审查报告"
depends_on = [
    { task = "security_scan", type = "all" },
    { task = "test_generation", type = "all" }
]
```

---

## 四、实现路线图

### Phase 1: 长程任务支持 (P0, 4-6 周)

**目标**: 将 ReAct 循环从 20 步扩展到支持 100+ 步的长程任务

**子任务**:
1. [ ] 任务检查点与恢复机制
   - 实现任务状态定期持久化（每 N 步保存）
   - 支持从检查点恢复执行
2. [ ] 任务分解与子任务管理
   - 引入 Task Planner 将大目标分解为可管理的子任务
   - 子任务独立状态追踪
3. [ ] Token 预算与智能剪枝
   - 实现基于优先级的消息剪枝策略
   - 保留系统消息和关键工具输出
4. [ ] 失败恢复与重试
   - 指数退避重试
   - 替代策略（Fallback）机制

**关键文件**:
- `src/react/loop.rs` - ReAct 循环扩展
- `src/core/state.rs` - 状态持久化
- `src/memory/token_budget.rs` - Token 管理

### Phase 2: 多 Agent 协作 (P0, 6-8 周)

**目标**: 实现多 Agent 间的通信与任务委派

**子任务**:
1. [ ] Agent 注册与发现
   - Agent 注册表（Agent Registry）
   - 能力发现机制
2. [ ] Agent 间通信协议
   - 消息总线（Message Bus）
   - 请求 - 响应模式
   - 发布 - 订阅模式
3. [ ] 任务委派机制
   - 基于能力的路由
   - 子 Agent 动态生成
4. [ ] 冲突解决与协调
   - 锁机制（避免多 Agent 同时修改同一资源）
   - 合并策略

**关键文件**:
- `src/core/agent_registry.rs` - 新建
- `src/core/message_bus.rs` - 新建
- `src/core/supervisor.rs` - 扩展现有 session_supervisor

### Phase 3: Agent 人格系统 (P1, 4-6 周)

**目标**: 实现可配置的 Agent 人格定义与行为约束

**子任务**:
1. [ ] 人格模板系统
   - TOML 格式的人格定义
   - 大五人格模型支持
2. [ ] 人格注入机制
   - System Prompt 动态生成
   - 行为约束应用
3. [ ] 人格 - 技能映射
   - 基于人格推荐技能组合
4. [ ] 人格评估与反馈
   - 行为一致性检查
   - 人格漂移检测

**关键文件**:
- `src/agent/persona.rs` - 新建
- `src/agent/persona_loader.rs` - 新建
- `config/personas/` - 新建目录

### Phase 4: 工作流程编排增强 (P1, 6-8 周)

**目标**: 实现可视化的工作流编排与执行

**子任务**:
1. [ ] 工作流 DSL 设计
   - YAML/TOML 格式定义
   - 支持条件分支、并行、循环
2. [ ] DAG 引擎增强
   - 动态图构建
   - 条件执行支持
3. [ ] 可视化编辑器（Web 前端）
   - 拖拽式流程设计
   - 实时执行状态展示
4. [ ] 工作流版本管理
   - Git 集成
   - 回滚能力

**关键文件**:
- `src/workflow/dsl.rs` - 新建
- `src/workflow/graph.rs` - 扩展
- `src/bin/web/workflow_editor.rs` - 新建

### Phase 5: 团队与流程管理 (P2, 4-6 周)

**目标**: 支持团队级别的流程定义与协作

**子任务**:
1. [ ] 团队工作空间
   - 多用户协作
   - 权限管理 (RBAC)
2. [ ] 流程模板市场
   - 模板分享与复用
   - 版本追踪
3. [ ] 审计与合规
   - 操作日志
   - Agent 行为追踪
4. [ ] 可观测性仪表盘
   - Metrics 收集与展示
   - 分布式追踪

**关键文件**:
- `src/team/workspace.rs` - 新建
- `src/team/rbac.rs` - 新建
- `src/observability/tracing.rs` - 扩展

---

## 五、技术选型建议

### 5.1 核心依赖

| 类别 | 推荐方案 | 理由 |
|------|----------|------|
| **向量数据库** | Qdrant / Chroma | 轻量、支持持久化、Rust 友好 |
| **知识图谱** | Memgraph / Neo4j | Cypher 查询、图算法支持 |
| **消息总线** | Tokio MPSC + Redis Pub/Sub | 异步、高性能 |
| **工作流引擎** | 自研 DAG + Petri Net | 灵活定制 |
| **前端框架** | React + React Flow | 可视化编排成熟生态 |
| **API 网关** | Axum + Tower | Rust 原生、高性能 |

### 5.2 架构决策

| 决策点 | 选项 A | 选项 B | 推荐 | 理由 |
|--------|--------|--------|------|------|
| **Agent 状态管理** | 中心化 | 去中心化 | 中心化 + 分布式缓存 | 简单可控，性能可通过缓存优化 |
| **工作流执行** | 同步 | 异步 | 异步 | 长程任务必须异步 |
| **人格定义** | 硬编码 | 配置文件 | 配置文件 (TOML) | 灵活性、可维护性 |
| **技能加载** | 启动时 | 按需 | 按需 + 缓存 | 减少启动时间，按需加载 |

---

## 六、风险与缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| **长程任务稳定性** | 高 | 中 | 分阶段验证，每阶段设置检查点 |
| **多 Agent 死锁** | 高 | 中 | 实现超时机制、死锁检测算法 |
| **Token 成本失控** | 中 | 高 | 实时成本追踪、预算告警 |
| **人格一致性漂移** | 中 | 中 | 定期人格评估、自动校正 |
| **工作流循环依赖** | 中 | 低 | DAG 验证、启动前检查 |

---

## 七、下一步行动

### 7.1 立即行动 (本周)

1. [ ] 确认产品方向与优先级
2. [ ] 组建核心团队（2-3 人）
3. [ ] 建立项目追踪机制

### 7.2 短期目标 (本月)

1. [ ] 完成 Phase 1 详细设计
2. [ ] 搭建开发环境
3. [ ] 实现第一个检查点机制原型

### 7.3 中期目标 (本季度)

1. [ ] Phase 1 & 2 核心功能上线
2. [ ] 内部 Alpha 测试
3. [ ] 收集反馈并迭代

---

## 八、参考资料

### 8.1 行业研究

- [2025 Was Agents. 2026 Is Agent Harnesses](https://aakashgupta.medium.com/2025-was-agents-2026-is-agent-harnesses-heres-why-that-changes-everything-073e9877655e)
- [Harness Engineering Evolution](https://www.epsilla.com/blogs/harness-engineering-evolution-prompt-context-autonomous-agents)
- [Multi-Agent Orchestration with DAGs](https://medium.com/@arpitnath42/a-practical-perspective-on-orchestrating-ai-agent-systems-with-dags-c9264bf38884)

### 8.2 学术研究

- [Natural-Language Agent Harnesses (arXiv 2026)](https://arxiv.org/html/2603.25723v1)
- [The Orchestration of Multi-Agent Systems (arXiv 2026)](https://arxiv.org/html/2601.13671v1)
- [Personality and Person AI Agents (2025)](https://www.researchgate.net/publication/397614373_Personality_and_Personal_AI_Agents_A_Co-Evolutionary_Framework)

### 8.3 竞品参考

- [Anthropic Persona Selection Model](https://www.anthropic.com/research/persona-selection-model)
- [LangGraph Multi-Agent Orchestration](https://latenode.com/blog/ai-frameworks-technical-infrastructure/langgraph-multi-agent-orchestration)

---

## 附录 A: 术语表

| 术语 | 定义 |
|------|------|
| **Harness** | 围绕 LLM 的基础设施层，负责任务编排、状态管理等 |
| **Persona** | Agent 的人格定义，包括性格、沟通风格、专业领域等 |
| **Skill** | Agent 可调用的具体能力集合 |
| **Workflow** | 多步骤任务的编排定义 |
| **Orchestration** | 多 Agent、多任务的协调与管理 |
| **Gating** | 请求入口的控制层（认证、限流、路由等） |

---

*文档结束*
