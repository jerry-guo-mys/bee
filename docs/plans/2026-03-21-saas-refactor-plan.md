# SaaS 化与服务化重构执行计划

> 创建日期：2026-03-21
> 当前状态：进行中
> 执行策略：按阶段顺序推进，优先模块化单体，再做多租户和服务拆分。

## 总体进度


| 阶段      | 目标          | 状态  | 进度   | 备注                                                          |
| ------- | ----------- | --- | ---- | ----------------------------------------------------------- |
| Phase 0 | 架构收口与入口拆分   | 已完成 | 100% | 入口边界已收口，完成配置/仓储/应用服务初步拆分                                    |
| Phase 1 | 主数据与仓储层落地   | 已完成 | 100% | 已落主数据、repository、sqlite schema，迁移与 bootstrap 覆盖 legacy 文件数据 |
| Phase 2 | 公司与团队初始化产品化 | 已完成 | 100% | 已落组织初始化模板、最小持久化创建服务、Web API 与最小 UI 入口                       |
| Phase 3 | 运行时多租户化     | 已完成 | 100% | 会话、上下文、记忆根目录与主执行链路工具边界都已具备 scope 语义                         |
| Phase 4 | 配置中心与模板中心   | 已完成 | 100% | 模板种子、运行时模板读取、租户级覆盖、团队实例化，以及工具/模型/知识库绑定链路都已落地      |
| Phase 5 | 权限、安全、审计    | 已完成 | 100% | 审计日志、成员角色 seed、关键管理接口 RBAC、租户/团队工具策略，以及知识库/工具失败审计都已接入         |
| Phase 6 | 任务与工作流产品化   | 已完成 | 100% | 任务已带租户/团队/workflow 元数据，提供看板、工作流模板和工作流启动 API，协作入口已上移到产品任务流程 |
| Phase 7 | 服务拆分与部署演进   | 已完成 | 100% | 已补共享服务契约、拆分顺序、API/BFF 边界、事件主题与部署/监控方案                         |


## 执行约束

- 保持 `TUI`、`Web`、`Gateway` 至少一条主链路可运行。
- 每个阶段先做边界收口，再做行为迁移。
- 优先替换文件型业务数据，再替换接口和运行时依赖。
- 每完成一个子任务都要更新本文件的状态与备注。

## Phase 0：架构收口与入口拆分

目标：从超大入口文件和混合职责中抽出稳定边界，为后续组织域和多租户改造做准备。

### 子任务

- P0.1 建立总计划文件并初始化阶段状态
- P0.2 识别 `src/bin/web.rs` 中的业务责任簇并按模块分组
- P0.3 抽离“助手目录/技能覆盖/静态配置加载”模块
- P0.4 抽离“任务读写与任务服务”模块
- P0.5 抽离“群组/会话持久化”模块
- P0.6 将 `src/bin/web.rs` 收敛为路由装配与 handler 入口
- P0.7 形成边界说明文档：运行时域 vs 产品域
- P0.8 执行最小编译检查并记录结果

### 进度记录


| 日期         | 子任务  | 状态  | 说明                                                                                                 |
| ---------- | ---- | --- | -------------------------------------------------------------------------------------------------- |
| 2026-03-21 | P0.1 | 完成  | 新建本计划文件并初始化阶段跟踪                                                                                    |
| 2026-03-21 | P0.2 | 完成  | 已识别 `web.rs` 中助手、任务、群组/会话、模型与路由等责任簇                                                                |
| 2026-03-21 | P0.3 | 完成  | 新增 `src/bin/web/assistant_catalog.rs`，迁出助手目录与技能覆盖加载逻辑                                              |
| 2026-03-21 | P0.4 | 完成  | 新增 `src/bin/web/task_service.rs` 和 `src/bin/web/task_coordinator_service.rs`，迁出任务模型、规则与统筹启动流程      |
| 2026-03-21 | P0.5 | 完成  | 新增 `src/bin/web/session_store.rs`，迁出群组/会话快照与磁盘持久化逻辑                                                |
| 2026-03-21 | P0.6 | 完成  | 新增 `src/bin/web/inbox_service.rs`、`src/bin/web/dynamic_agent_catalog.rs`，`web.rs` 收敛到路由与轻量 handler |
| 2026-03-21 | P0.7 | 完成  | 新增 `docs/plans/2026-03-21-phase0-boundaries.md`，明确接入层、产品域、运行时域边界                                   |
| 2026-03-21 | P0.8 | 完成  | `cargo check --bin bee-web --features web` 通过；顺手修复 `workflow` 的特性门问题                               |


## Phase 1：主数据与仓储层落地

目标：把组织、团队、Agent、会话、任务从文件模型迁移到正式数据模型和仓储接口。

### 子任务

- P1.1 定义核心实体：`tenant`、`organization`、`team`、`membership`
- P1.2 定义核心实体：`agent_template`、`agent_instance`、`workspace`
- P1.3 定义核心实体：`conversation`、`conversation_message`、`task`
- P1.4 增加 repository trait：`OrgRepository`
- P1.5 增加 repository trait：`AgentRepository`
- P1.6 增加 repository trait：`ConversationRepository`
- P1.7 增加 repository trait：`TaskRepository`
- P1.8 用 sqlite 落第一版 schema
- P1.9 替换 `agents.json`
- P1.10 替换 `groups.json`
- P1.11 替换 `sessions/*.json`
- P1.12 增加迁移/种子初始化逻辑

### 进度记录


| 日期         | 子任务                    | 状态  | 说明                                                                                                       |
| ---------- | ---------------------- | --- | -------------------------------------------------------------------------------------------------------- |
| 2026-03-21 | P1.1-P1.7              | 完成  | 新增 `src/saas/models.rs`、`src/saas/repository.rs` 和 `src/saas/mod.rs`，落主数据模型与仓储边界                         |
| 2026-03-21 | P1.8                   | 完成  | 新增 `src/saas/sqlite.rs`，完成第一版 SaaS sqlite schema 与初始化入口                                                  |
| 2026-03-21 | P1.9/P1.10/P1.11/P1.12 | 完成  | 新增 `src/saas/migration.rs`、`src/saas/bootstrap.rs`，迁移与 bootstrap 覆盖 `agents/groups/sessions/tasks` 及群聊会话 |


## Phase 2：公司与团队初始化产品化

目标：支持快速创建一个公司和相关团队，而不是让用户手工拼装 Agent。

### 子任务

- P2.1 定义组织初始化请求模型
- P2.2 定义行业模板与团队模板结构
- P2.3 设计默认团队模板：销售
- P2.4 设计默认团队模板：客服
- P2.5 设计默认团队模板：市场
- P2.6 设计默认团队模板：HR
- P2.7 设计默认团队模板：研发
- P2.8 增加“创建公司”应用服务
- P2.9 增加“根据模板创建团队”应用服务
- P2.10 增加“为团队生成默认 Agent 实例”应用服务
- P2.11 提供组织初始化 API
- P2.12 为 Web UI 增加组织初始化入口

### 进度记录


| 日期         | 子任务         | 状态  | 说明                                                                                                    |
| ---------- | ----------- | --- | ----------------------------------------------------------------------------------------------------- |
| 2026-03-21 | P2.1-P2.7   | 完成  | 新增 `src/saas/bootstrap_service.rs`，落组织初始化请求、行业模板、团队模板与默认 Agent 种子                                     |
| 2026-03-21 | P2.8-P2.10  | 完成  | 新增 `src/saas/sqlite_seed_repository.rs`，组织初始化计划已可最小化落库到 tenant/org/team/agent_template/agent_instance |
| 2026-03-21 | P2.11-P2.12 | 完成  | `bee-web` 新增 `/api/organizations/bootstrap`，设置页增加组织初始化入口，并补齐 workspace 落库                             |


## Phase 3：运行时多租户化

目标：让会话、上下文、记忆、工具边界都带上租户语义。

### 子任务

- P3.1 为会话模型加入 `tenant_id`
- P3.2 为会话模型加入 `organization_id`
- P3.3 为会话模型加入 `team_id`
- P3.4 为会话模型加入 `agent_instance_id`
- P3.5 为会话模型加入 `user_id`
- P3.6 改造 `SessionStore` 接口支持租户上下文
- P3.7 改造 `AgentRuntime` 支持租户上下文注入
- P3.8 改造长期记忆目录与存储隔离策略
- P3.9 改造 workspace 解析为“租户工作空间”
- P3.10 为高风险工具加入租户级权限边界

### 进度记录


| 日期         | 子任务               | 状态  | 说明                                                                                                                                                   |
| ---------- | ----------------- | --- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-03-21 | P3.foundation     | 完成  | `gateway/session`、`session_store`、`persistent_session` 已补 `tenant/org/team/agent_instance/user` scope 基础字段，并修复 `saas/migration.rs` 的 feature gate 耦合 |
| 2026-03-21 | P3.1-P3.4         | 完成  | `ClientInfo.metadata` 已可注入 `SessionScope`，`gateway/runtime` 在创建新上下文时会按 tenant/org/team/user/agent_instance 派生独立 memory workspace                     |
| 2026-03-21 | P3.web-scope      | 完成  | `bee-web` 的 `chat/history/compact/clear/stream` 已支持显式 scope，请求会映射到 scoped session key、snapshot path 与 memory root                                    |
| 2026-03-21 | P3.frontend-scope | 完成  | `static/index.html` 已持有当前组织 scope，并在会话加载、发送消息、删除/清理会话时显式传递 `tenant/org/team/agent_instance/user` 维度                                                  |
| 2026-03-21 | P3.scope-switcher | 完成  | 设置页已增加“当前组织上下文”可视化与编辑入口，会话列表会按当前 tenant/org/team scope 过滤显示                                                                                          |
| 2026-03-21 | P3.tool-boundary  | 完成  | `bee-web` 主对话、收件箱、任务协调与 `gateway/runtime` 都已按 scope 过滤高风险工具；没有 `team_id` 的会话默认禁用 `shell/code_edit/code_write/git_commit/create/send` 等高风险能力          |


## Phase 4：配置中心与模板中心

目标：把静态配置迁移为可被租户和团队覆盖的模板中心。

### 子任务

- P4.1 将 `config/assistants.toml` 定义为种子模板来源
- P4.2 增加 `agent_template` 持久化与读取接口
- P4.3 增加平台默认模板加载器
- P4.4 增加租户级模板覆盖
- P4.5 增加团队级模板实例化
- P4.6 增加工具包/技能包模板绑定
- P4.7 增加模型策略模板绑定
- P4.8 增加知识库模板绑定

### 进度记录


| 日期         | 子任务       | 状态  | 说明                                                                                                                               |
| ---------- | --------- | --- | -------------------------------------------------------------------------------------------------------------------------------- |
| 2026-03-21 | P4.1-P4.3 | 完成  | 新增 `src/saas/template_catalog.rs`、`src/saas/sqlite_template_repository.rs`，`assistants.toml` 已可作为平台模板种子写入 `saas_agent_templates` |
| 2026-03-21 | P4.runtime-read | 完成  | `src/bin/web/assistant_catalog.rs` 已改为优先读取 sqlite 模板仓储，静态文件退为回退来源 |
| 2026-03-21 | P4.4 | 完成 | `PUT /api/assistant/:id/skills` 已同步写入 sqlite 模板仓储，租户模板覆盖不再只依赖 `assistant_skills.json` |
| 2026-03-21 | P4.5-P4.7 | 完成 | 新增 `src/saas/template_instantiation_service.rs`，`bee-web` 提供 `/api/agent-templates` 与 `/api/teams/:team_id/agent-instances/bootstrap`，团队实例已可继承模板的 prompt、tools 与 model 策略 |
| 2026-03-21 | P4.8 | 完成 | `saas_agent_templates` / `saas_agent_instances` 已补知识库绑定字段与继承链路，`bee-web` 新增 `/api/assistant/:id/knowledge-bases` 用于租户级模板覆盖 |


## Phase 5：权限、安全、审计

目标：补齐 SaaS 必需的访问控制与审计能力。

### 子任务

- P5.1 定义平台角色与组织角色
- P5.2 增加成员与角色绑定
- P5.3 增加团队级资源授权
- P5.4 增加工具白名单的租户/团队覆盖
- P5.5 增加高风险操作审计日志
- P5.6 增加配置变更审计日志
- P5.7 增加知识库访问审计
- P5.8 为管理接口加权限校验

### 进度记录


| 日期  | 子任务 | 状态  | 说明  |
| --- | --- | --- | --- |
| 2026-03-21 | P5.foundation | 完成 | 新增 `src/saas/audit_service.rs` 与 `saas_audit_logs`，`bee-web` 已开始为组织初始化、团队实例化、模板技能/知识库变更写审计日志，并提供 `/api/audit-logs` 查询 |
| 2026-03-21 | P5.1-P5.3/P5.8 | 完成 | 新增 `src/saas/auth_service.rs`，组织初始化会 seed 默认 `org_admin` membership，`bee-web` 的组织初始化、模板查询、团队实例化、模板更新与审计查询都已接入最小 RBAC 校验 |
| 2026-03-21 | P5.4 | 完成 | 新增 `src/saas/tool_policy_service.rs` 和 `saas_tool_access_policies`，`bee-web`/`bee-gateway` 的 scope 工具过滤已切到租户/组织/团队级策略，`bee-web` 提供 `/api/tool-policies` 查询与更新 |
| 2026-03-21 | P5.5-P5.7 | 完成 | `bee-web` 已为工具策略更新、模板配置变更、知识库访问和流式/群聊工具失败写审计日志，Phase 5 审计覆盖面已闭环 |


## Phase 6：任务与工作流产品化

目标：把协作从底层群聊原语提升为业务任务和流程。

### 子任务

- P6.1 梳理当前 `group/create/send` 的运行时职责
- P6.2 定义产品级任务域模型
- P6.3 定义工作流模板与节点模型
- P6.4 将任务与团队绑定
- P6.5 将工作流与组织模板绑定
- P6.6 将 `workflow/engine` 接入产品任务域
- P6.7 提供任务看板 API
- P6.8 提供工作流启动 API
- P6.9 收敛用户侧暴露的底层群聊原语

### 进度记录


| 日期  | 子任务 | 状态  | 说明  |
| --- | --- | --- | --- |
| 2026-03-21 | P6.1-P6.5 | 完成 | 新增 `src/bin/web/workflow_product_service.rs`，任务模型已补 `tenant/org/team/workflow_template/workflow_run` 元数据，并支持模板化工作流启动 |
| 2026-03-21 | P6.6-P6.8 | 完成 | `bee-web` 新增 `/api/task-board`、`/api/workflow-templates`、`/api/workflows/start`，当前产品任务域已能承接工作流模板和看板查询 |
| 2026-03-21 | P6.9 | 完成 | 用户侧新增的高层工作流与任务看板入口已替代直接面向群聊原语的主路径，`group` 机制退回为内部协作细节 |


## Phase 7：服务拆分与部署演进

目标：在边界稳定后按职责拆服务，而不是提前拆散。

### 子任务

- P7.1 抽离 `conversation_runtime` 服务边界
- P7.2 抽离 `workflow_task` 服务边界
- P7.3 抽离 `knowledge_memory` 服务边界
- P7.4 设计 API Gateway / BFF 边界
- P7.5 设计服务间鉴权与租户透传
- P7.6 设计事件总线或异步任务总线
- P7.7 补齐部署、监控、告警方案

### 进度记录


| 日期  | 子任务 | 状态  | 说明  |
| --- | --- | --- | --- |
| 2026-03-21 | P7.1-P7.6 | 完成 | 新增 `src/service_contracts/mod.rs`，统一服务租户上下文、请求信封、事件主题与部署切片定义，为 `conversation_runtime` / `workflow_task` / `knowledge_memory` 拆分提供共享契约 |
| 2026-03-21 | P7.7 | 完成 | 新增 `docs/PHASE7_SERVICE_DEPLOYMENT.md`，明确拆分顺序、API Gateway/BFF 边界、服务间鉴权透传、事件总线主题以及部署/监控方案 |
