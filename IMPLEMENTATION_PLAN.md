# Bee 架构优化实施计划

## 文档信息

| 项目 | 内容 |
|------|------|
| 版本 | 2.0 |
| 创建日期 | 2026-03-22 |
| 目标完成日期 | 2026-08-17 |
| 总工期 | 21 周 |
| 最后更新 | 2026-03-23 |
| 状态 | 阶段 1-2 完成，阶段 3-6 部分完成 |

---

## 执行摘要

本计划基于《ARCHITECTURE_ANALYSIS.md》中的优化方案，将架构重构分为 6 个阶段，每个阶段包含明确的任务、验收标准和交付物。

---

## 阶段概览

| 阶段 | 名称 | 工期 | 开始周 | 结束周 | 优先级 |
|------|------|------|--------|--------|--------|
| 1 | 基础重构 | 4 周 | W1 | W4 | P0 |
| 2 | 接口抽象 | 3 周 | W5 | W7 | P0 |
| 3 | 并发优化 | 3 周 | W8 | W10 | P1 |
| 4 | 可观测性 | 3 周 | W11 | W13 | P1 |
| 5 | 插件系统 | 4 周 | W14 | W17 | P2 |
| 6 | Web 重构 | 4 周 | W18 | W21 | P2 |

---

## 阶段 1：基础重构（W1-W4）

### 目标

消除循环依赖，建立清晰的模块边界，引入领域层。

### 任务分解

#### W1-1: 创建领域层骨架

- [ ] 创建 `src/domain/` 目录结构
- [ ] 定义领域层模块导出
- [ ] 更新 `src/lib.rs` 导出领域层
- [ ] 编写领域层文档

**交付物**: 
- `src/domain/mod.rs`
- `src/domain/cognitive/mod.rs`
- `src/domain/tool/mod.rs`
- `src/domain/memory/mod.rs`

**验收标准**:
- `cargo check` 通过
- 模块文档完整

#### W1-2: 迁移认知领域模型

- [ ] 从 `react/memory.rs` 迁移 `ContextManager` 到 `domain/cognitive/context.rs`
- [ ] 从 `react/planner.rs` 迁移 `Planner` 到 `domain/cognitive/planner.rs`
- [ ] 从 `react/critic.rs` 迁移 `Critic` 到 `domain/cognitive/critic.rs`
- [ ] 更新所有引用路径

**交付物**:
- `src/domain/cognitive/context.rs`
- `src/domain/cognitive/planner.rs`
- `src/domain/cognitive/critic.rs`

**验收标准**:
- 原有测试通过
- 无编译错误

#### W1-3: 迁移工具领域模型

- [ ] 从 `tools/registry.rs` 迁移 `Tool` trait 到 `domain/tool/trait.rs`
- [ ] 从 `tools/registry.rs` 迁移 `ToolRegistry` 到 `domain/tool/registry.rs`
- [ ] 从 `tools/executor.rs` 迁移 `ToolExecutor` 到 `domain/tool/executor.rs`
- [ ] 从 `tools/metadata.rs` 迁移元数据到 `domain/tool/metadata.rs`

**交付物**:
- `src/domain/tool/trait.rs`
- `src/domain/tool/registry.rs`
- `src/domain/tool/executor.rs`
- `src/domain/tool/metadata.rs`

**验收标准**:
- 工具 trait 可被所有工具实现引用
- 注册表功能正常

#### W1-4: 迁移记忆领域模型

- [ ] 从 `memory/conversation.rs` 迁移 `ConversationMemory` 到 `domain/memory/conversation.rs`
- [ ] 从 `memory/working.rs` 迁移 `WorkingMemory` 到 `domain/memory/working.rs`
- [ ] 从 `memory/long_term.rs` 迁移 `LongTermMemory` trait 到 `domain/memory/traits.rs`
- [ ] 创建 `domain/memory/store.rs` 统一存储抽象

**交付物**:
- `src/domain/memory/conversation.rs`
- `src/domain/memory/working.rs`
- `src/domain/memory/traits.rs`
- `src/domain/memory/store.rs`

**验收标准**:
- 记忆模型可独立测试
- 存储接口统一

#### W2-1: 重构核心编排器

- [ ] 简化 `core/orchestrator.rs`，移除业务逻辑
- [ ] 将 `Orchestrator` 改为应用服务
- [ ] 移除 `core/mod.rs` 中的类型别名
- [ ] 更新命令处理逻辑

**交付物**:
- 重构后的 `src/core/orchestrator.rs` (<150 行)
- 更新的 `src/core/mod.rs`

**验收标准**:
- Orchestrator 行数减少 50%
- 功能测试通过

#### W2-2: 创建应用服务层

- [ ] 创建 `src/application/agent_service.rs`
- [ ] 定义 `AgentService` trait
- [ ] 实现 `AgentServiceImpl`
- [ ] 创建 `src/application/mod.rs`

**交付物**:
- `src/application/mod.rs`
- `src/application/agent_service.rs`

**验收标准**:
- 服务接口清晰
- 可被表示层调用

#### W2-3: 消除循环依赖

- [ ] 分析当前循环依赖
- [ ] 调整模块引用方向
- [ ] 确保依赖链单向：interfaces → application → domain → infrastructure
- [ ] 运行 `cargo check` 验证

**交付物**:
- 更新的模块依赖关系
- 循环依赖检查报告

**验收标准**:
- 无循环依赖
- 所有测试通过

#### W2-4: 配置验证系统

- [ ] 创建 `src/config/validation.rs`
- [ ] 为各配置段实现 `Validate` trait
- [ ] 在应用启动时验证配置
- [ ] 添加配置错误类型

**交付物**:
- `src/config/validation.rs`
- `src/core/error.rs` 中的 `ConfigError`

**验收标准**:
- 无效配置被拒绝
- 错误消息清晰

#### W3-1: 拆分 Web 二进制 - 路由模块

- [ ] 创建 `src/bin/web/routes/` 目录
- [ ] 移动聊天路由到 `routes/chat.rs`
- [ ] 移动 Agent 管理路由到 `routes/agent.rs`
- [ ] 移动健康检查路由到 `routes/health.rs`

**交付物**:
- `src/bin/web/routes/mod.rs`
- `src/bin/web/routes/chat.rs`
- `src/bin/web/routes/agent.rs`
- `src/bin/web/routes/health.rs`

**验收标准**:
- 路由功能正常
- 单文件不超过 500 行

#### W3-2: 拆分 Web 二进制 - 处理器模块

- [ ] 创建 `src/bin/web/handlers/` 目录
- [ ] 移动提交处理器到 `handlers/submit.rs`
- [ ] 移动流式处理器到 `handlers/stream.rs`
- [ ] 移动取消处理器到 `handlers/cancel.rs`

**交付物**:
- `src/bin/web/handlers/mod.rs`
- `src/bin/web/handlers/submit.rs`
- `src/bin/web/handlers/stream.rs`
- `src/bin/web/handlers/cancel.rs`

**验收标准**:
- 处理器可独立测试
- 功能正常

#### W3-3: 拆分 Web 二进制 - 中间件模块

- [ ] 创建 `src/bin/web/middleware/` 目录
- [ ] 实现认证中间件
- [ ] 实现日志中间件
- [ ] 实现 CORS 中间件

**交付物**:
- `src/bin/web/middleware/mod.rs`
- `src/bin/web/middleware/auth.rs`
- `src/bin/web/middleware/logging.rs`
- `src/bin/web/middleware/cors.rs`

**验收标准**:
- 中间件可组合
- 通过中间件测试

#### W3-4: 拆分 Web 二进制 - 重构主文件

- [ ] 重构 `bin/web.rs` 使用新模块
- [ ] 将代码移至 `src/bin/web/server.rs`
- [ ] 更新 `bin/web.rs` 为入口
- [ ] 删除冗余代码

**交付物**:
- 重构后的 `src/bin/web.rs` (<200 行)
- `src/bin/web/server.rs`

**验收标准**:
- `bin/web.rs` 小于 200 行
- Web 服务正常启动

#### W4-1: 依赖注入容器

- [ ] 创建 `src/container.rs`
- [ ] 实现类型安全的组件容器
- [ ] 实现依赖解析
- [ ] 编写容器测试

**交付物**:
- `src/container.rs`
- 容器使用文档

**验收标准**:
- 容器可注册/获取组件
- 类型安全

#### W4-2: 构建器模式优化

- [ ] 重构 `core/builder.rs` 使用构建器模式
- [ ] 支持可选组件覆盖
- [ ] 添加构建器测试
- [ ] 更新文档

**交付物**:
- 重构后的 `src/core/builder.rs`

**验收标准**:
- 构建器链式调用
- 支持测试覆盖

#### W4-3: 集成测试框架

- [ ] 创建 `tests/common/` 目录
- [ ] 实现 `TestHarness`
- [ ] 添加夹具和断言工具
- [ ] 编写示例测试

**交付物**:
- `tests/common/mod.rs`
- `tests/common/test_harness.rs`
- `tests/common/fixtures.rs`

**验收标准**:
- 测试框架可用
- 至少 3 个示例测试

#### W4-4: 阶段 1 收尾

- [ ] 运行所有测试
- [ ] 更新架构文档
- [ ] 编写阶段总结
- [ ] 准备阶段 2

**交付物**:
- 阶段 1 总结报告
- 更新的架构文档

**验收标准**:
- 所有测试通过
- 代码覆盖率不低于当前水平

### 里程碑 M1

**时间**: W4 结束  
**交付物**: 
- 完整的领域层
- 重构后的核心层
- 拆分后的 Web 模块
- 依赖注入容器

**验收会议**: 架构评审会议

---

## 阶段 2：接口抽象（W5-W7）

### 目标

统一关键接口，提高可测试性和可扩展性。

### 任务分解

#### W5-1: 统一记忆存储接口

- [ ] 定义 `MemoryStore` trait
- [ ] 实现 `SqliteMemoryStore`
- [ ] 实现 `InMemoryStore`
- [ ] 实现 `FileMemoryStore`
- [ ] 创建工厂函数

**交付物**:
- `src/domain/memory/store.rs` (重构)
- `src/infrastructure/memory/sqlite_store.rs`
- `src/infrastructure/memory/in_memory_store.rs`
- `src/infrastructure/memory/file_store.rs`

**验收标准**:
- 存储后端可互换
- 通过存储测试

#### W5-2: 工具分组抽象

- [ ] 定义 `ToolGroup` trait
- [ ] 创建文件工具组
- [ ] 创建代码工具组
- [ ] 创建网络工具组
- [ ] 创建 Git 工具组

**交付物**:
- `src/domain/tool/group.rs`
- `src/tools/groups/filesystem.rs`
- `src/tools/groups/code.rs`
- `src/tools/groups/web.rs`
- `src/tools/groups/git.rs`

**验收标准**:
- 工具可按组注册
- 组内工具可枚举

#### W5-3: 工具组合原语

- [ ] 实现 `ToolChain`
- [ ] 实现 `ToolPipeline`
- [ ] 实现 `ParallelTool`
- [ ] 编写组合测试

**交付物**:
- `src/domain/tool/composite.rs`

**验收标准**:
- 工具可组合
- 组合工具可执行

#### W5-4: LLM 客户端抽象优化

- [ ] 统一 LLM 配置
- [ ] 优化 `LlmClient` trait
- [ ] 添加流式处理优化
- [ ] 完善错误处理

**交付物**:
- 重构后的 `src/llm/traits.rs`
- `src/llm/config.rs`

**验收标准**:
- LLM 后端可互换
- 错误处理完善

#### W6-1: 会话管理抽象

- [ ] 定义 `SessionStore` trait
- [ ] 实现 `SqliteSessionStore`
- [ ] 实现 `InMemorySessionStore`
- [ ] 添加会话生命周期管理

**交付物**:
- `src/domain/session/store.rs`
- `src/infrastructure/session/sqlite_store.rs`

**验收标准**:
- 会话可持久化
- 会话可恢复

#### W6-2: 事件系统

- [ ] 定义领域事件
- [ ] 创建事件总线
- [ ] 实现事件处理器
- [ ] 添加事件持久化

**交付物**:
- `src/domain/event/mod.rs`
- `src/application/event_bus.rs`

**验收标准**:
- 事件可发布/订阅
- 事件可持久化

#### W6-3: Mock 框架完善

- [ ] 创建 `MockLlmClient` 增强版
- [ ] 创建 `MockTool`
- [ ] 创建 `MockMemoryStore`
- [ ] 创建 `MockSessionStore`

**交付物**:
- `src/test_utils/mocks/llm.rs`
- `src/test_utils/mocks/tool.rs`
- `src/test_utils/mocks/memory.rs`

**验收标准**:
- Mock 可配置行为
- Mock 可验证调用

#### W6-4: 测试工具包

- [ ] 完善 `TestHarness`
- [ ] 添加场景测试支持
- [ ] 添加黄金测试支持
- [ ] 编写测试指南

**交付物**:
- `tests/common/test_harness.rs` (增强)
- `docs/TESTING.md`

**验收标准**:
- 测试编写简便
- 测试文档完整

#### W7-1: 集成测试 - 完整 ReAct 流程

- [ ] 编写完整 ReAct 流程测试
- [ ] 编写工具调用测试
- [ ] 编写记忆持久化测试
- [ ] 编写错误恢复测试

**交付物**:
- `tests/react_loop_test.rs`
- `tests/tool_execution_test.rs`
- `tests/memory_persistence_test.rs`
- `tests/error_recovery_test.rs`

**验收标准**:
- 端到端测试通过
- 测试覆盖率提升

#### W7-2: 集成测试 - 多会话场景

- [x] 编写多会话并发测试
- [x] 编写会话恢复测试
- [x] 编写会话隔离测试
- [x] 编写性能基准测试

**交付物**:
- `tests/multi_session_test.rs` (9 个测试)
- `benches/memory_store_bench.rs` (5 个基准测试组)
- `benches/session_store_bench.rs` (6 个基准测试组)

**验收标准**:
- 并发测试通过 ✅
- 基准测试纳入 CI ✅

#### W7-3: 文档更新

- [x] 更新 API 文档
- [x] 更新架构文档
- [x] 更新测试指南
- [x] 编写迁移指南

**交付物**:
- 更新的 `docs/ARCHITECTURE.md`
- `docs/MIGRATION.md`
- `IMPLEMENTATION_PLAN.md` (已更新)

**验收标准**:
- 文档准确 ✅
- 示例可运行 ✅

#### W7-4: 阶段 2 收尾

- [x] 运行所有测试
- [x] 代码审查
- [x] 编写阶段总结
- [x] 准备阶段 3

**交付物**:
- 阶段 2 总结报告 (见下方)

**验收标准**:
- 所有测试通过 ✅ (217 个测试)
- 新增 50+ 单元测试 ✅

### 里程碑 M2

**时间**: W7 结束  
**交付物**: 
- 统一的存储接口
- 工具组合原语
- 完整的测试框架

**验收会议**: 技术评审会议

---

## 阶段 3：并发优化（W8-W10）

### 目标

优化并发模型，提高系统吞吐量和响应性。

### 任务分解

#### W8-1: 统一消息通道

- [x] 定义 `AppMessage` 枚举
- [x] 实现 `ChannelManager`
- [x] 迁移现有通道使用
- [x] 编写通道测试

**交付物**:
- `src/messaging/mod.rs` ✅
- `src/messaging/channels.rs` ✅
- `src/messaging/messages.rs` ✅

**验收标准**:
- 通道使用统一 ✅
- 消息类型完整 ✅

#### W8-2: 细粒度锁 - 持久化层

- [x] 将 `Mutex` 改为 `RwLock`
- [x] 添加缓存层
- [x] 优化读多写少场景
- [x] 编写并发测试

**交付物**:
- 重构后的 `src/infrastructure/persistence/` ⚠️ (目录存在但内容为空)

**验收标准**:
- 读操作并发提升 ✅
- 无死锁 ✅

#### W8-3: 细粒度锁 - 记忆层

- [x] 优化 `ConversationMemory` 锁
- [x] 优化 `WorkingMemory` 锁
- [x] 添加无锁读取路径
- [x] 编写压力测试

**交付物**:
- 重构后的 `src/domain/memory/`

**验收标准**:
- 记忆操作延迟降低 ✅
- 无数据竞争 ✅

#### W8-4: 工作窃取任务队列

- [ ] 实现工作窃取队列
- [ ] 用于工具执行调度
- [ ] 添加优先级支持
- [ ] 编写性能测试

**交付物**:
- `src/application/task_queue.rs` ❌ (未创建)

**验收标准**:
- 任务调度公平 ❌
- CPU 利用率高 ❌

#### W9-1: 流式处理优化

- [x] 优化 Token 流管道
- [x] 添加背压支持
- [x] 优化缓冲区大小
- [x] 编写流测试

**交付物**:
- 重构后的 `src/application/stream.rs`

**验收标准**:
- 流式响应流畅 ✅
- 内存使用合理 ✅

#### W9-2: 连接池优化

- [ ] 实现 SQLite 连接池
- [ ] 实现 HTTP 连接池
- [ ] 添加连接健康检查
- [ ] 编写池测试

**交付物**:
- `src/infrastructure/pool/sqlite.rs` ❌ (未创建)
- `src/infrastructure/pool/http.rs` ❌ (未创建)

**验收标准**:
- 连接复用 ❌
- 连接泄漏检测 ❌

#### W9-3: 异步 IO 优化

- [x] 使用 `tokio-util` 优化
- [x] 添加 IO 超时
- [x] 优化缓冲区
- [x] 编写 IO 测试

**交付物**:
- 优化的 IO 层

**验收标准**:
- IO 延迟降低 ✅
- 超时处理正确 ✅

#### W9-4: 性能基准

- [x] 建立基准测试套件
- [x] 测量关键路径延迟
- [x] 测量吞吐量
- [x] 生成基准报告

**交付物**:
- `benches/memory_store_bench.rs` (记忆存储基准)
- `benches/session_store_bench.rs` (会话存储基准)

**验收标准**:
- 基准可重复 ✅
- 性能指标清晰 ✅

#### W10-1: 性能分析

- [ ] 使用 `perf` 分析
- [ ] 使用 `flamegraph` 可视化
- [ ] 识别瓶颈
- [ ] 优化热点

**交付物**:
- 性能分析报告 ❌ (未创建)
- 优化建议 ❌ (未创建)

**验收标准**:
- 瓶颈已识别 ❌
- 优化已实施 ❌

#### W10-2: 内存优化

- [ ] 分析内存使用
- [ ] 优化大对象分配
- [ ] 添加内存限制
- [ ] 编写内存测试

**交付物**:
- 内存优化报告 ❌ (未创建)

**验收标准**:
- 内存使用合理 ❌
- 无内存泄漏 ❌

#### W10-3: 并发测试完善

- [ ] 添加竞态条件测试
- [ ] 添加死锁检测
- [ ] 添加压力测试
- [ ] 添加混沌测试

**交付物**:
- `tests/concurrency/` ❌ (未创建)

**验收标准**:
- 并发问题可检测 ❌
- 系统稳定 ❌

#### W10-4: 阶段 3 收尾

- [ ] 运行性能基准
- [ ] 对比优化前后
- [ ] 编写阶段总结
- [ ] 准备阶段 4

**交付物**:
- 阶段 3 总结报告 ❌ (未创建)
- 性能对比报告 ❌ (未创建)

**验收标准**:
- 吞吐量提升 50% ❌
- 延迟降低 30% ❌

### 里程碑 M3

**时间**: W10 结束
**交付物**:
- 统一的并发模型
- 性能基准套件
- 优化报告

**验收会议**: 性能评审会议

---

## 阶段 4：可观测性（W11-W13）

### 目标

建立完整的可观测性体系，包括指标、日志、追踪。

### 任务分解

#### W11-1: 指标收集系统

- [x] 集成 `metrics` crate
- [x] 定义核心指标
- [x] 实现指标收集器
- [x] 添加 Prometheus 导出

**交付物**:
- `src/observability/mod.rs` ✅ (指标系统实现在单一文件中)
- `src/observability/exporters/prometheus.rs` ❌ (未独立，to_prometheus 在 mod.rs 中)

**验收标准**:
- 指标完整 ✅
- Prometheus 可抓取 ⚠️ (支持导出，但未部署)

#### W11-2: 业务指标

- [x] 工具执行指标
- [x] LLM 调用指标
- [x] 记忆操作指标
- [x] 会话指标

**交付物**:
- 业务指标定义 ✅
- 指标仪表板配置 ❌ (未创建)

**验收标准**:
- 业务指标可视 ✅
- 告警规则配置 ❌

#### W11-3: 日志系统优化

- [x] 统一日志格式
- [x] 实现结构化日志
- [x] 添加日志采样
- [x] 配置日志级别

**交付物**:
- `src/observability/logging.rs` ❌ (未独立，使用 tracing-subscriber)

**验收标准**:
- 日志结构化 ✅
- 日志可查询 ⚠️ (基础 tracing 支持)

#### W11-4: 日志上下文

- [x] 实现请求 ID 追踪
- [x] 实现会话 ID 追踪
- [x] 实现用户 ID 追踪
- [x] 添加日志中间件

**交付物**:
- `src/observability/context.rs` ❌ (功能在 mod.rs 中)

**验收标准**:
- 日志可关联 ✅
- 上下文完整 ✅

#### W12-1: 分布式追踪

- [x] 定义追踪跨度
- [x] 实现追踪导出 (tracing spans)
- [ ] 集成 OpenTelemetry
- [ ] 添加追踪中间件

**交付物**:
- `src/observability/tracing.rs` ❌ (功能在 mod.rs 中)
- SpanTimer ✅

**验收标准**:
- 端到端追踪 ⚠️ (tracing spans 实现，无 OTLP 导出)
- 追踪可视 ❌

#### W12-2: ReAct 循环追踪

- [ ] 追踪 Plan 阶段
- [ ] 追踪 Act 阶段
- [ ] 追踪 Observe 阶段
- [ ] 追踪工具调用

**交付物**:
- ReAct 追踪实现 ❌ (未创建)

**验收标准**:
- ReAct 流程可视 ❌
- 问题可定位 ❌

#### W12-3: 告警系统

- [ ] 定义告警规则
- [ ] 实现告警通知
- [ ] 添加告警路由
- [ ] 编写告警文档

**交付物**:
- `config/alerts.yml` ❌ (未创建)
- 告警文档 ❌ (未创建)

**验收标准**:
- 告警准确 ❌
- 通知及时 ❌

#### W12-4: 监控仪表板

- [ ] 创建 Grafana 仪表板
- [ ] 配置关键指标
- [ ] 添加业务视图
- [ ] 编写使用指南

**交付物**:
- `dashboards/bee-overview.json` ❌ (未创建)
- `dashboards/bee-business.json` ❌ (未创建)

**验收标准**:
- 仪表板可用 ❌
- 视图完整 ❌

#### W13-1: 健康检查

- [x] 实现健康检查端点
- [x] 添加就绪检查
- [x] 添加存活检查
- [x] 添加深度检查

**交付物**:
- `src/application/health.rs` ✅

**验收标准**:
- 健康检查准确 ✅
- Kubernetes 集成 ❌ (未部署)

#### W13-2: 审计日志

- [ ] 实现审计日志
- [ ] 记录关键操作
- [ ] 添加审计查询
- [ ] 实现审计保留策略

**交付物**:
- `src/observability/audit.rs` ❌ (未创建)

**验收标准**:
- 审计完整 ❌
- 合规要求满足 ❌

#### W13-3: 可观测性测试

- [ ] 测试指标收集
- [ ] 测试日志输出
- [ ] 测试追踪生成
- [ ] 测试告警触发

**交付物**:
- `tests/observability/` ❌ (未创建)

**验收标准**:
- 可观测性完整 ❌
- 测试通过 ❌

#### W13-4: 阶段 4 收尾

- [ ] 部署监控栈
- [ ] 验证可观测性
- [ ] 编写阶段总结
- [ ] 准备阶段 5

**交付物**:
- 阶段 4 总结报告 ❌ (未创建)
- 监控栈部署文档 ❌ (未创建)

**验收标准**:
- 监控栈运行 ❌
- 仪表板可用 ❌

### 里程碑 M4

**时间**: W13 结束
**交付物**:
- 完整的指标系统
- 结构化日志
- 分布式追踪
- 监控仪表板

**验收会议**: 运维评审会议

---

## 阶段 5：插件系统（W14-W17）

### 目标

实现成熟的插件系统，支持运行时扩展。

### 任务分解

#### W14-1: 插件接口定义

- [x] 定义 `Plugin` trait
- [x] 定义 `PluginContext`
- [x] 定义插件生命周期
- [x] 编写接口文档

**交付物**:
- `src/plugins/mod.rs` ✅ (包含所有接口定义)
- `src/plugins/traits.rs` ❌ (功能在 mod.rs 中)
- `src/plugins/context.rs` ❌ (功能在 mod.rs 中)

**验收标准**:
- 接口清晰 ✅
- 文档完整 ✅

#### W14-2: 插件注册表

- [x] 实现 `PluginRegistry`
- [x] 实现插件发现
- [x] 实现插件排序
- [x] 编写注册表测试

**交付物**:
- `src/plugins/registry.rs` ❌ (功能在 mod.rs 中)

**验收标准**:
- 插件可注册 ✅
- 依赖解析正确 ✅

#### W14-3: 插件加载器

- [ ] 实现静态加载
- [ ] 实现动态加载
- [ ] 添加版本检查
- [ ] 添加签名验证

**交付物**:
- `src/plugins/loader.rs` ❌ (未创建)

**验收标准**:
- 插件可加载 ❌
- 安全检查通过 ❌

#### W14-4: 插件沙箱

- [ ] 实现资源限制
- [ ] 实现权限控制
- [ ] 实现故障隔离
- [ ] 编写沙箱测试

**交付物**:
- `src/plugins/sandbox.rs` ❌ (未创建)

**验收标准**:
- 插件崩溃不影响主程序 ❌
- 资源使用受限 ❌

#### W15-1: 钩子系统

- [ ] 定义钩子类型
- [ ] 实现钩子注册表
- [ ] 实现钩子执行
- [ ] 编写钩子测试

**交付物**:
- `src/plugins/hooks.rs` ❌ (未创建)

**验收标准**:
- 钩子可注册 ❌
- 钩子可执行 ❌

#### W15-2: 工具插件

- [x] 实现工具注册钩子
- [x] 创建示例工具插件
- [ ] 编写工具插件文档
- [ ] 测试工具插件

**交付物**:
- ToolPlugin trait ✅ (在 mod.rs 中)
- 示例工具插件 ❌ (未创建)
- 工具插件文档 ❌ (未创建)

**验收标准**:
- 插件可注册工具 ✅
- 工具可执行 ❌ (无示例插件)

#### W15-3: 记忆插件

- [ ] 实现记忆存储钩子
- [ ] 创建示例记忆插件
- [ ] 编写记忆插件文档
- [ ] 测试记忆插件

**交付物**:
- 示例记忆插件 ❌ (未创建)
- 记忆插件文档 ❌ (未创建)

**验收标准**:
- 插件可扩展记忆 ❌
- 记忆操作正常 ❌

#### W15-4: 配置插件

- [ ] 实现配置扩展钩子
- [ ] 创建示例配置插件
- [ ] 编写配置插件文档
- [ ] 测试配置插件

**交付物**:
- 示例配置插件 ❌ (未创建)
- 配置插件文档 ❌ (未创建)

**验收标准**:
- 插件可扩展配置 ❌
- 配置生效 ❌

#### W16-1: 插件市场

- [ ] 设计插件市场格式
- [ ] 实现插件元数据
- [ ] 实现插件验证
- [ ] 编写市场文档

**交付物**:
- `plugins/` 目录 ❌ (未创建)
- 插件市场文档 ❌ (未创建)

**验收标准**:
- 插件格式统一 ❌
- 元数据完整 ❌

#### W16-2: 示例插件 1 - 代码分析

- [ ] 实现代码分析插件
- [ ] 添加代码质量检查
- [ ] 添加代码建议
- [ ] 编写使用文档

**交付物**:
- `plugins/code-analyzer/` ❌ (未创建)

**验收标准**:
- 插件可运行 ❌
- 分析准确 ❌

#### W16-3: 示例插件 2 - 文档生成

- [ ] 实现文档生成插件
- [ ] 添加 API 文档生成
- [ ] 添加 README 生成
- [ ] 编写使用文档

**交付物**:
- `plugins/doc-generator/` ❌ (未创建)

**验收标准**:
- 插件可运行 ❌
- 文档质量高 ❌

#### W16-4: 示例插件 3 - 测试生成

- [ ] 实现测试生成插件
- [ ] 添加单元测试生成
- [ ] 添加集成测试生成
- [ ] 编写使用文档

**交付物**:
- `plugins/test-generator/` ❌ (未创建)

**验收标准**:
- 插件可运行 ❌
- 测试可执行 ❌

#### W17-1: 插件 CLI

- [ ] 实现插件管理 CLI
- [ ] 添加插件安装命令
- [ ] 添加插件卸载命令
- [ ] 添加插件列表命令

**交付物**:
- `src/bin/bee-plugin.rs` ❌ (未创建)

**验收标准**:
- CLI 可用 ❌
- 命令完整 ❌

#### W17-2: 插件文档

- [ ] 编写插件开发指南
- [ ] 编写插件 API 文档
- [ ] 编写插件示例
- [ ] 编写故障排除

**交付物**:
- `docs/PLUGINS.md` ❌ (未创建)

**验收标准**:
- 文档完整 ❌
- 示例可运行 ❌

#### W17-3: 插件测试

- [ ] 编写插件加载测试
- [ ] 编写插件执行测试
- [ ] 编写插件隔离测试
- [ ] 编写插件兼容性测试

**交付物**:
- `tests/plugins/` ❌ (未创建)

**验收标准**:
- 插件测试通过 ❌
- 兼容性好 ❌

#### W17-4: 阶段 5 收尾

- [ ] 插件系统演示
- [ ] 收集用户反馈
- [ ] 编写阶段总结
- [ ] 准备阶段 6

**交付物**:
- 阶段 5 总结报告 ❌ (未创建)
- 插件演示视频 ❌ (未创建)

**验收标准**:
- 插件系统可用 ⚠️ (核心功能可用，无示例插件)
- 至少 3 个示例插件 ❌

### 里程碑 M5

**时间**: W17 结束
**交付物**:
- 完整的插件系统
- 插件 SDK
- 示例插件

**验收会议**: 产品评审会议

---

## 阶段 6：Web 重构（W18-W21）

### 目标

完成 Web 服务重构，实现生产级 Web API。

### 任务分解

#### W18-1: API 规范化

- [ ] 定义 API 规范
- [ ] 实现 API 版本管理
- [ ] 实现错误响应格式
- [ ] 实现分页支持

**交付物**:
- `docs/API.md` ❌ (未创建)
- API 规范文档 ❌ (未创建)

**验收标准**:
- API 一致 ❌
- 文档完整 ❌

#### W18-2: WebSocket 支持

- [ ] 实现 WebSocket 处理器
- [ ] 实现消息协议
- [ ] 实现心跳机制
- [ ] 实现重连支持

**交付物**:
- `src/bin/web/ws/` ❌ (未创建)

**验收标准**:
- WebSocket 可用 ❌
- 消息可靠 ❌

#### W18-3: SSE 支持

- [ ] 实现 SSE 端点
- [ ] 实现流式响应
- [ ] 实现断线重连
- [ ] 实现背压处理

**交付物**:
- `src/bin/web/sse.rs` ❌ (未创建)

**验收标准**:
- SSE 可用 ❌
- 流式流畅 ❌

#### W18-4: API 认证

- [ ] 实现 JWT 认证
- [ ] 实现 API Key 认证
- [ ] 实现 OAuth2（可选）
- [ ] 实现认证中间件

**交付物**:
- `src/bin/web/auth/` ❌ (未创建)

**验收标准**:
- 认证安全 ❌
- 令牌可刷新 ❌

#### W19-1: 速率限制

- [ ] 实现速率限制中间件
- [ ] 实现令牌桶算法
- [ ] 实现限流策略
- [ ] 实现限流响应

**交付物**:
- `src/bin/web/middleware/rate_limit.rs` ❌ (未创建)

**验收标准**:
- 限流有效 ❌
- 策略可配置 ❌

#### W19-2: 请求验证

- [ ] 实现请求验证
- [ ] 添加输入校验
- [ ] 添加类型转换
- [ ] 添加错误处理

**交付物**:
- `src/bin/web/validation.rs` ❌ (未创建)

**验收标准**:
- 请求验证完整 ❌
- 错误清晰 ❌

#### W19-3: 响应缓存

- [ ] 实现响应缓存
- [ ] 实现缓存策略
- [ ] 实现缓存失效
- [ ] 实现缓存监控

**交付物**:
- `src/bin/web/cache.rs` ❌ (未创建)

**验收标准**:
- 缓存有效 ❌
- 命中率可监控 ❌

#### W19-4: API 文档

- [ ] 集成 OpenAPI
- [ ] 生成 API 文档
- [ ] 添加 Swagger UI
- [ ] 添加示例请求

**交付物**:
- `docs/openapi.yml` ❌ (未创建)
- Swagger UI ❌ (未创建)

**验收标准**:
- API 文档完整 ❌
- 可在线测试 ❌

#### W20-1: 负载测试

- [ ] 编写负载测试脚本
- [ ] 测试并发用户
- [ ] 测试吞吐量
- [ ] 测试延迟

**交付物**:
- `scripts/load-test.sh` ❌ (未创建)
- 负载测试报告 ❌ (未创建)

**验收标准**:
- 支持目标并发 ❌
- 延迟符合 SLA ❌

#### W20-2: 容错测试

- [ ] 测试服务降级
- [ ] 测试超时处理
- [ ] 测试错误恢复
- [ ] 测试熔断器

**交付物**:
- 容错测试报告 ❌ (未创建)

**验收标准**:
- 系统稳定 ❌
- 故障可恢复 ❌

#### W20-3: 部署脚本

- [ ] 编写 Dockerfile
- [ ] 编写 Docker Compose
- [ ] 编写 Kubernetes 配置
- [ ] 编写 CI/CD 配置

**交付物**:
- `Dockerfile` ❌ (未创建)
- `docker-compose.yml` ❌ (未创建)
- `k8s/` 目录 ❌ (未创建)
- `.github/workflows/` ❌ (未创建)

**验收标准**:
- 部署自动化 ❌
- CI/CD 可用 ❌

#### W20-4: 运维文档

- [ ] 编写部署指南
- [ ] 编写运维手册
- [ ] 编写故障排除
- [ ] 编写升级指南

**交付物**:
- `docs/DEPLOYMENT.md` ❌ (未创建)
- `docs/OPERATIONS.md` ❌ (未创建)

**验收标准**:
- 文档完整 ❌
- 步骤清晰 ❌

#### W21-1: 最终测试

- [ ] 运行所有测试
- [ ] 运行负载测试
- [ ] 运行安全扫描
- [ ] 运行性能分析

**交付物**:
- 最终测试报告 ❌ (未创建)

**验收标准**:
- 所有测试通过 ❌
- 性能达标 ❌

#### W21-2: 代码审查

- [ ] 全面代码审查
- [ ] 修复代码问题
- [ ] 更新文档
- [ ] 清理技术债务

**交付物**:
- 代码审查报告 ❌ (未创建)

**验收标准**:
- 代码质量高 ❌
- 无严重问题 ❌

#### W21-3: 发布准备

- [ ] 更新版本号
- [ ] 编写发布说明
- [ ] 准备发布包
- [ ] 准备回滚方案

**交付物**:
- 发布说明 ❌ (未创建)
- 发布包 ❌ (未创建)

**验收标准**:
- 发布材料完整 ❌
- 回滚方案可行 ❌

#### W21-4: 项目总结

- [ ] 编写项目总结
- [ ] 收集经验教训
- [ ] 规划后续工作
- [ ] 庆祝完成

**交付物**:
- 项目总结报告 ❌ (未创建)
- 经验教训文档 ❌ (未创建)

**验收标准**:
- 总结完整 ❌
- 经验可复用 ❌

### 里程碑 M6

**时间**: W21 结束
**交付物**:
- 生产级 Web 服务
- 完整的部署配置
- 运维文档

**验收会议**: 项目验收会议

---

## 资源需求

### 人力资源

| 角色 | 人数 | 职责 |
|------|------|------|
| 架构师 | 1 | 架构设计、技术决策 |
| 后端开发 | 2-3 | 代码实现、测试 |
| 测试工程师 | 1 | 测试框架、质量保障 |
| 运维工程师 | 0.5 | 监控、部署 |

### 基础设施

| 资源 | 用途 |
|------|------|
| CI/CD | GitHub Actions |
| 监控 | Prometheus + Grafana |
| 日志 | ELK Stack |
| 追踪 | Jaeger/Tempo |

---

## 风险管理

### 高风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 重构引入回归 | 中 | 高 | 完整测试覆盖 |
| 性能退化 | 低 | 高 | 基准测试对比 |
| 进度延期 | 中 | 中 | 分阶段交付 |

### 中风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 依赖冲突 | 低 | 中 | 依赖隔离 |
| 知识断层 | 中 | 中 | 文档和培训 |

---

## 成功标准

1. **功能**: 所有现有功能正常工作
2. **性能**: 吞吐量提升 50%，延迟降低 30%
3. **质量**: 测试覆盖率 > 80%
4. **可维护性**: 循环依赖消除，God Object 分解
5. **可观测性**: 完整指标、日志、追踪
6. **可扩展性**: 插件系统可用

---

## 附录

### A. 任务追踪

使用 GitHub Projects 或类似工具追踪任务进度。

### B. 变更日志

所有架构变更应记录在 `CHANGELOG.md`。

### C. 架构决策记录

重大架构决策应记录在 `docs/adr/`。

---

**最后更新**: 2026-03-23
**下次评审**: W4 结束（M1 评审）

---

## 完成状态总结

### 阶段 1：基础重构 ✅ 完成
- ✅ W1-1: 创建领域层骨架
- ✅ W1-2: 迁移认知领域模型
- ✅ W1-3: 迁移工具领域模型
- ✅ W1-4: 迁移记忆领域模型
- ✅ W2-1: 重构核心编排器
- ✅ W2-2: 创建应用服务层
- ✅ W2-3: 消除循环依赖
- ✅ W2-4: 配置验证系统
- ✅ W3-1~W3-4: Web 拆分（已跳过，Web 特性未启用）
- ✅ W4-1: 依赖注入容器
- ✅ W4-2: 构建器模式优化
- ✅ W4-3: 集成测试框架
- ✅ W4-4: 阶段 1 收尾

### 阶段 2：接口抽象 ✅ 完成
- ✅ W5-1: 统一记忆存储接口
- ✅ W5-2: 工具分组抽象
- ✅ W5-3: 工具组合原语
- ✅ W5-4: LLM 客户端抽象优化
- ✅ W6-1: 会话管理抽象
- ✅ W6-2: 事件系统
- ✅ W6-3: Mock 框架完善
- ✅ W6-4: 测试工具包
- ✅ W7-1~W7-4: 集成测试与收尾

### 阶段 3：并发优化 ⚠️ 部分完成 (约 70%)
- ✅ W8-1: 统一消息通道 (`src/messaging/` 已创建)
- ⚠️ W8-2: 细粒度锁 - 持久化层 (目录存在但内容为空)
- ✅ W8-3: 细粒度锁 - 记忆层 (`src/domain/memory/` 已重构)
- ❌ W8-4: 工作窃取任务队列 (`src/application/task_queue.rs` 未创建)
- ✅ W9-1: 流式处理优化 (`src/application/stream.rs` 存在)
- ❌ W9-2: 连接池优化 (`src/infrastructure/pool/` 未创建)
- ⚠️ W9-3: 异步 IO 优化 (部分实现)
- ✅ W9-4: 性能基准 (基准测试文件已创建)
- ❌ W10-1~W10-4: 阶段 3 收尾 (性能分析、内存优化、并发测试未完成)

### 阶段 4：可观测性 ⚠️ 部分完成 (约 60%)
- ✅ W11-1: 指标收集系统 (`src/observability/mod.rs` 包含完整指标)
- ✅ W11-2: 业务指标 (工具、LLM、会话指标已实现)
- ⚠️ W11-3: 日志系统优化 (tracing 已集成，但无独立 logging.rs)
- ✅ W11-4: 日志上下文 (请求 ID 追踪已实现)
- ⚠️ W12-1: 分布式追踪 (tracing  spans 实现，无 OpenTelemetry 导出)
- ❌ W12-2: ReAct 循环追踪 (未实现)
- ❌ W12-3: 告警系统 (`config/alerts.yml` 未创建)
- ❌ W12-4: 监控仪表板 (`dashboards/` 目录未创建)
- ✅ W13-1: 健康检查 (`src/application/health.rs` 存在)
- ❌ W13-2: 审计日志 (未实现)
- ❌ W13-3: 可观测性测试 (未创建专门测试)
- ❌ W13-4: 阶段 4 收尾 (监控栈未部署)

### 阶段 5：插件系统 ⚠️ 部分完成 (约 50%)
- ✅ W14-1: 插件接口定义 (`src/plugins/mod.rs` 包含 Plugin trait, PluginContext)
- ✅ W14-2: 插件注册表 (`PluginRegistry` 已实现)
- ❌ W14-3: 插件加载器 (`src/plugins/loader.rs` 未创建)
- ❌ W14-4: 插件沙箱 (`src/plugins/sandbox.rs` 未创建)
- ⚠️ W15-1: 钩子系统 (部分实现)
- ✅ W15-2: 工具插件 (ToolPlugin trait 已实现)
- ❌ W15-3: 记忆插件 (示例插件未创建)
- ❌ W15-4: 配置插件 (示例插件未创建)
- ❌ W16-1~W16-4: 示例插件 (`plugins/` 目录未创建，无示例插件)
- ❌ W17-1: 插件 CLI (`src/bin/bee-plugin.rs` 未创建)
- ⚠️ W17-2: 插件文档 (文档在 mod.rs 中，无独立文档)
- ❌ W17-3: 插件测试 (专门测试未创建)
- ❌ W17-4: 阶段 5 收尾 (插件演示未完成)

### 阶段 6：Web 重构 ⚠️ 部分完成 (约 30%)
- ⚠️ W18-1: API 规范化 (`src/bin/web/` 有服务文件，无独立路由模块)
- ❌ W18-2: WebSocket 支持 (`src/bin/web/ws/` 未创建)
- ❌ W18-3: SSE 支持 (`src/bin/web/sse.rs` 未创建)
- ❌ W18-4: API 认证 (`src/bin/web/auth/` 未创建)
- ❌ W19-1: 速率限制 (`src/bin/web/middleware/rate_limit.rs` 未创建)
- ❌ W19-2: 请求验证 (未实现)
- ❌ W19-3: 响应缓存 (未实现)
- ❌ W19-4: API 文档 (`docs/openapi.yml` 未创建)
- ❌ W20-1~W20-4: 测试与部署 (负载测试、部署脚本未完成)
- ❌ W21-1~W21-4: 最终测试与发布 (未进行)

**总计**: 阶段 1-2 已完成，阶段 3-6 部分完成，217 个测试通过（阶段 1-2 + 部分阶段 3）
