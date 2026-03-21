# Phase 0 边界收口说明

> 日期：2026-03-21
> 目标：明确 `bee-web` 在 SaaS 化重构前后的职责边界，避免后续继续把业务逻辑堆回入口文件。

## 当前结论

`src/bin/web.rs` 不再承载完整产品逻辑实现，应逐步收敛为：

- 路由注册
- `AppState` 组装
- 轻量 handler
- 模块编排

业务规则、配置装载、文件仓储、会话持久化、任务协调等逻辑应继续下沉到独立模块，后续再演进为应用服务或仓储层。

## 已收口的边界

### 1. 助手目录与技能覆盖

文件：
- `src/bin/web/assistant_catalog.rs`

职责：
- 加载 `assistants.toml`
- 合并 `config/skills/*.toml`
- 加载/保存 `assistant_skills.json`
- 根据技能构建完整 Prompt

不再留在 `web.rs` 的内容：
- 配置文件解析
- Prompt 拼接规则
- 工具列表过滤

### 2. 动态 Agent 目录

文件：
- `src/bin/web/dynamic_agent_catalog.rs`

职责：
- 加载 `agents.json`
- 为动态 Agent 生成 Prompt
- 将动态 Agent 热更新到运行时状态

### 3. 任务模型与任务规则

文件：
- `src/bin/web/task_service.rs`
- `src/bin/web/task_coordinator_service.rs`

职责：
- 任务文件读写
- 任务创建/更新规则
- 状态映射
- 统筹任务启动与流式响应

不再留在 `web.rs` 的内容：
- 任务字段归一化
- 统筹任务的异步执行主流程

### 4. 群组与会话持久化

文件：
- `src/bin/web/session_store.rs`
- `src/bin/web/inbox_service.rs`

职责：
- 群组/会话快照定义
- 会话磁盘路径规则
- 群组/会话读写
- 群聊消息转 LLM 历史
- 收件箱处理

## Phase 1 起必须遵守的边界

### 产品域

包含：
- 租户
- 公司
- 团队
- 成员
- Agent 模板/实例
- 任务/工作流

要求：
- 进入独立 domain/application/repository 边界
- 不能直接以 `json` 文件作为唯一业务数据源

### 运行时域

包含：
- 会话上下文
- ReAct 执行
- 流式输出
- 工具调用
- 长期记忆挂载

要求：
- 与产品域通过显式参数或仓储接口连接
- 不直接依赖 Web handler

### 接入层

包含：
- Web
- Gateway
- Lark
- WhatsApp

要求：
- 只做协议适配、参数校验、响应拼装
- 不直接承载复杂业务规则

## 后续拆分顺序

1. 继续收敛 `web.rs` 中剩余重 handler
2. 进入 Phase 1，建立主数据模型与 repository
3. 用正式数据模型替换 `agents.json/groups.json/sessions/*.json`
4. 再推动多租户化和服务化
