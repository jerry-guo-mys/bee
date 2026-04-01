# Web UI 管理控制台规划

**创建日期**: 2026-04-01  
**状态**: 草案  
**依据规格**: [2026-03-28-gateway-saas-architecture-design.md](../specs/2026-03-28-gateway-saas-architecture-design.md)

---

## 1. 目标与对齐

网关 SaaS 规格要求：**多租户隔离**（租户 → 组织 → 团队 → 成员）、**RBAC + 工具策略**、**审计可追溯**、Gateway 可扩展。管理端（Web UI）应支撑日常运营与治理两类工作流，并与后端真实接口一致，避免长期停留在静态 mock。

---

## 2. 现状：两条 HTTP 能力线

| 来源 | 作用 | 对 web-ui 的优先级 |
|------|------|-------------------|
| `bee-web` / `bee-admin`（`src/bin/web/server.rs` 中 `router_admin_api`，`/api/*`） | 运行态：助手/Agent、任务看板、工作流、工具策略、审计、追踪、指标、SSE 等；**bee-admin** 仅管理 API（默认 8081） | **一期优先接入**；web-ui 可代理到 `bee-admin` |
| `src/interfaces/http`（`/tenants`、`/organizations/...` 等） | SaaS 域：租户/组织/团队/成员 CQRS | **二期「租户与成员管理」**；需与 `bee-web` 同源挂载或独立 base URL + 代理，并完成 JWT 上下文（handlers 内尚有 TODO） |

---

## 3. 信息架构（侧边栏分组建议）

### A. 概览与运行

- **Dashboard**：健康摘要、关键 KPI、最近事件（短列表）
- **Agent 管理**：助手列表、动态 Agent、模板与实例化（对齐规格中的 Agent 与工具策略）

### B. 任务与工作流

- **任务 / Workflow**：看板 + 列表、工作流模板、启动工作流、任务详情与状态

### C. 可观测与合规

- **监控与日志**：指标、追踪列表/详情、审计日志检索、告警视图（对齐规格审计设计）

### D. 治理与安全（规格核心，二期可集中落地）

- **租户与组织**：租户详情、组织列表/详情、创建组织（视平台角色开放）
- **团队**：团队列表、创建团队、成员按团队筛选
- **成员与邀请**：邀请、接受邀请、暂停成员、角色展示（RBAC）
- **工具策略**：租户/组织/团队维度策略读写

### E. 系统

- **设置**：当前租户/组织/团队上下文、登录与 token、API 基址与环境标识

现有路由（`/`, `/agents`, `/workflows`, `/monitoring`, `/settings`）可保留；新增 **工具策略**、**租户/组织/团队/成员** 等子路由，或收到「治理」分组下。

---

## 4. 页面与 API 映射（`bee-web` `/api` 前缀）

与 `web-ui` 中 Vite 代理（`/api` → 后端）一致。

| 页面 | 用户任务 | 建议对接 API（一期） |
|------|----------|----------------------|
| Dashboard | 运行与健康一览 | `GET /api/metrics`、`GET /api/traces/recent`；可选 `GET /api/task-board` 摘要；活动流可用 `GET /api/events`（SSE）或任务轮询 |
| Agent 管理 | 助手与子 Agent、创建/配置 | `GET /api/assistants`、`GET /api/agents`、`POST /api/agents`；`GET /api/agent-templates`、`POST /api/teams/:team_id/agent-instances/bootstrap` |
| 任务/Workflow | 看板、列表、启停、工作流 | `GET /api/task-board`、`GET`/`POST /api/tasks`、`PATCH /api/tasks/:id`、`POST /api/tasks/:id/start`；`GET /api/workflow-templates`、`POST /api/workflows/start` |
| 监控日志 | 指标、追踪、审计 | `GET /api/metrics`、`GET /api/metrics/prometheus`（外链或说明）；`GET /api/traces/recent`、`GET /api/traces/:request_id`、`GET /api/audit-logs` |
| 工具策略（新页） | 按作用域配置工具 | `GET /api/tool-policies`、`PUT /api/tool-policies` |
| 租户/组织/团队/成员（新页） | 多租户生命周期 | `POST`/`GET /tenants...`、`/organizations...`、`/members...`（需统一前缀，如 `/api/saas/...`，并接 JWT） |

---

## 5. 分阶段交付

### 阶段 1（前端价值最大）

- 增加统一 API 层（如 `src/lib/api.ts` + React Query 或同类）：`fetch`、`Authorization`、错误与后端 `ApiError` 形态
- Dashboard、Agents、Workflows、Monitoring **全部改为真实 `/api` 数据**；完善空态与错误态
- 监控页：追踪与审计分 Tab，语义与「应用日志」区分

### 阶段 2（治理控制台）

- 接入 CQRS HTTP（或与后端约定合并进 `bee-web`）
- 登录后持久化 `tenant_id` / `organization_id` / `team_id`；列表与创建表单带作用域
- 成员流：邀请、角色与规格命名一致（如 OrgAdmin、TeamAdmin 等）

### 阶段 3（体验与安全）

- 侧边栏与按钮级权限：对齐 JWT `permissions` / `role`（规格 §5.3）
- 工具策略可视化编辑（校验、只读模式）
- 可选：`/api/events` 驱动实时活动组件

### 阶段 4（可选）

- 向导式 **组织引导**：`POST /api/organizations/bootstrap`
- Prometheus/Grafana 外链、审计导出等

---

## 6. 技术约定

- **样式**：Tailwind v4 + `@tailwindcss/vite`（见 `web-ui/vite.config.ts`）；新页沿用 `index.css` 中 `@theme` token
- **SSE**：`EventSource` 接 `/api/events`，注意代理缓冲与重连
- **类型**：为 `/api` 响应维护 TypeScript 类型（OpenAPI 生成或手写，与 `web.rs` 注释对齐）

---

## 7. 后端依赖与缺口

- CQRS 与 `bee-web` **是否同源**：若否，web-ui 需 `VITE_SAAS_API_BASE` 或第二套代理
- Handlers 中 **`tenant_id` / 组织反查** 等待补全，否则成员/团队列表可能错租户
- 规格中的 **ListOrganizations / ListTeams** 等查询若未暴露 REST，需在对应阶段补接口后再做完整 UI

---

## 8. 验证

- 本地：`cd web-ui && npm run verify`（完整构建，含类型检查）
- 联调：`npm run dev`，确认 `vite.config.ts` 中 `/api` 代理指向运行中的 `bee-web`

---

## 相关文件

- 规格：`docs/superpowers/specs/2026-03-28-gateway-saas-architecture-design.md`
- 前端：`web-ui/`（`README.md`、`vite.config.ts`）
- 运行态 API：`src/bin/web/server.rs`（`bee-web` 合并对话路由；`bee-admin` 仅管理路由）
- SaaS CQRS HTTP：`src/interfaces/http/`
