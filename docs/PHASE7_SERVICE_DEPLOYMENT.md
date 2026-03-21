# Phase 7 服务拆分与部署演进

## 目标

在当前模块化单体基础上，明确未来拆分 `conversation_runtime`、`workflow_task`、`knowledge_memory` 三个服务时的数据归属、事件主题、鉴权透传和部署顺序，避免后续再次重构上下文语义。

## 共享契约

代码契约位于 [src/service_contracts/mod.rs](/Users/g/Documents/GitHub/feature/org_20260321/src/service_contracts/mod.rs)。

- `ServiceTenantContext`
  - 统一透传 `tenant_id / organization_id / team_id / user_id / agent_instance_id / request_id`
- `ServiceRequestEnvelope<T>`
  - 统一服务间调用包裹层
- `ServiceEventTopic`
  - 当前定义了会话、工具失败、工作流、知识访问、策略更新、审计日志等主题
- `DeploymentSlice`
  - 明确每个候选服务的数据主权、读取依赖、发布和订阅主题

## 推荐拆分顺序

1. `conversation_runtime`
   - 当前职责已经集中在 `gateway/runtime`、`session_store`、`web` chat 链路
   - 先拆它可以把会话状态、流式输出和上下文隔离单独收口
2. `workflow_task`
   - 当前职责已经在 `task_service`、`workflow_product_service`、`task_coordinator_service`
   - 拆出后可以把任务看板、模板启动、任务协调和审计隔离
3. `knowledge_memory`
   - 当前职责分散在 `memory/*`、SaaS 模板知识绑定和知识访问审计
   - 等上面两个边界稳定后再拆，避免前期接口抖动

## API Gateway / BFF 边界

- `bee-web`
  - 继续作为 BFF
  - 负责 session cookie/token、SSE/NDJSON 流、页面聚合接口
- 未来 API Gateway
  - 只做鉴权、限流、租户透传和路由
  - 不保存业务会话状态

## 服务间鉴权与租户透传

- 所有内部调用都必须带 `ServiceTenantContext`
- Gateway/BFF 到下游服务时必须注入 `request_id`
- 下游服务不得自行推断租户信息
- 内部事件必须包含相同上下文，保证审计和故障排查可串联

## 事件总线建议

建议先采用单总线、多主题：

- `conversation.message.created`
- `conversation.tool.failed`
- `workflow.run.started`
- `workflow.task.created`
- `workflow.task.updated`
- `knowledge.accessed`
- `tool.policy.updated`
- `audit.log.created`

优先保证幂等消费，而不是追求复杂编排。

## 部署建议

第一步：
- 模块化单体，单库单部署

第二步：
- `conversation_runtime` 独立部署
- 保留共享 sqlite/数据库，先不拆 DB

第三步：
- `workflow_task` 独立部署
- 引入异步事件消费

第四步：
- `knowledge_memory` 独立部署
- 向量检索和长时记忆服务单独扩缩容

## 监控与告警

至少补齐：

- 按 `tenant_id` 的请求量、错误率、延迟
- 工具失败率和高风险工具调用次数
- 工作流启动数、任务积压数、任务完成率
- 知识访问量、检索失败率
- 事件总线消费延迟和死信数

## 当前结论

当前仓库已经具备进入“可拆分单体”的条件，但还不建议直接拆库拆仓。最稳妥的路径仍然是：

1. 保持同仓共享契约
2. 先以 `service_contracts` 固化边界
3. 再按运行时、任务域、知识域顺序拆部署
