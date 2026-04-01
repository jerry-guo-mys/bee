# Bee Admin 管理端功能规划与设计文档

**创建日期**: 2026-04-01  
**作者**: Claude (产品经理模式)  
**状态**: 设计审批中  
**版本**: v1.0

---

## 1. 概述

### 1.1 项目背景

Bee 是一个基于 Rust 的 ReAct 架构 AI Agent 系统，目前已实现 TUI、Web、WhatsApp、Lark 等多种交互界面。随着系统功能日益完善，需要一个功能完整的管理端来支持：
- 多租户 SaaS 运营管理
- 组织/团队/成员的权限管理
- 系统监控与可观测性
- 资源配置与策略管理

### 1.2 当前状态

**已有功能**（web-ui）：
| 页面 | 功能 | API 端点 |
|------|------|----------|
| Dashboard | 核心指标概览、最近追踪、快速统计 | `/api/metrics`、`/api/traces`、`/api/tasks` |
| 监控日志 | LLM 指标、Token 分布、请求趋势、审计日志 | `/api/metrics`、`/api/traces`、`/api/audit-logs` |
| Agent 管理 | Assistant/动态 Agent 查看与创建 | `/api/assistants`、`/api/agents` |
| 工具策略 | 工具访问权限配置 | `/api/tools`、`/api/tool-policies` |
| 任务/Workflow | 任务看板、从模板创建工作流 | `/api/tasks`、`/api/workflow-templates` |
| 系统设置 | 租户/组织/用户作用域配置 | localStorage |

**后端已实现领域模型**：
- `Tenant` (租户聚合根) + `TenantDomainService`
- `Organization` (组织聚合根) + `OrganizationDomainService`
- `Team` (团队聚合根) + `TeamDomainService`
- `Membership` (成员聚合根) + `MemberDomainService`
- `RbacService` (权限检查服务)
- `ToolPolicyService` (工具策略服务)

---

## 2. 功能规划总览

### 2.1 功能模块架构图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Bee Admin 管理端                                  │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  租户管理   │  │  组织管理   │  │  团队管理   │  │  成员管理   │    │
│  │  Tenant     │  │ Organization│  │    Team     │  │  Member     │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  RBAC 权限   │  │  审计日志   │  │  用量分析   │  │  订阅计费   │    │
│  │    RBAC     │  │   Audit     │  │   Usage     │  │  Billing    │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  技能市场   │  │  知识库     │  │  自动化告警 │  │  系统设置   │    │
│  │   Skills    │  │ Knowledge   │  │   Alerts    │  │  Settings   │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
├─────────────────────────────────────────────────────────────────────────┤
│                         现有功能 (已实现)                                │
│  Dashboard │ 监控日志 │ Agent 管理 │ 工具策略 │ 任务/Workflow          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 迭代计划

| 阶段 | 周期 | 模块 | 优先级 |
|------|------|------|--------|
| **Phase 1** | 1-2 周 | 审计日志增强、成员管理列表 | ⭐⭐⭐⭐⭐ |
| **Phase 2** | 2-3 周 | 租户管理、组织管理、团队管理 | ⭐⭐⭐⭐⭐ |
| **Phase 3** | 2 周 | RBAC 权限管理、角色分配 | ⭐⭐⭐⭐ |
| **Phase 4** | 2 周 | 模型用量分析、运营报表 | ⭐⭐⭐⭐ |
| **Phase 5** | 1-2 周 | SaaS 订阅与计费 | ⭐⭐⭐ |
| **Phase 6** | 1-2 周 | 技能市场、知识库管理 | ⭐⭐⭐ |
| **Phase 7** | 1 周 | 自动化告警配置 | ⭐⭐⭐ |

---

## 3. 详细功能设计

### 3.1 租户管理模块 (Tenant Management)

#### 3.1.1 功能列表

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 租户列表 | 查看所有租户、搜索、状态过滤 | `GET /api/tenants` | `TenantList.tsx` |
| 租户详情 | 基本信息、组织列表、用量统计 | `GET /api/tenants/:id` | `TenantDetail.tsx` |
| 创建租户 | 新建租户（名称、slug、描述） | `POST /api/tenants` | `TenantCreate.tsx` |
| 编辑租户 | 更新租户信息 | `PUT /api/tenants/:id` | `TenantEdit.tsx` |
| 暂停/恢复 | 暂停或恢复租户 | `POST /api/tenants/:id/suspend`<br>`POST /api/tenants/:id/restore` | 内置于详情/列表 |
| 归档租户 | 软删除租户 | `POST /api/tenants/:id/archive` | 内置于详情/列表 |

#### 3.1.2 API 设计

```typescript
// GET /api/tenants
interface TenantListResponse {
  tenants: TenantSummary[];
  total: number;
}

interface TenantSummary {
  id: string;
  name: string;
  slug: string;
  status: 'active' | 'suspended' | 'archived';
  organization_count: number;
  created_at: string;
}

// POST /api/tenants
interface CreateTenantRequest {
  name: string;
  slug: string;
  description?: string;
}

// GET /api/tenants/:id
interface TenantDetailResponse {
  id: string;
  name: string;
  slug: string;
  status: TenantStatus;
  description?: string;
  created_at: string;
  updated_at: string;
  organizations: OrganizationSummary[];
  usage_stats?: UsageStats;
}
```

#### 3.1.3 页面原型

```
┌────────────────────────────────────────────────────────────────┐
│ 租户管理                                    [+ 创建租户]        │
├────────────────────────────────────────────────────────────────┤
│ 搜索：[________________]  状态：[全部 ▼]  [查询]                │
├────────────────────────────────────────────────────────────────┤
│ 名称          │ Slug           │ 状态    │ 组织数 │ 创建时间  │
├───────────────┼────────────────┼─────────┼────────┼───────────┤
│ ACME Corp     │ acme-corp      │ ✅ 活跃 │   3    │ 2026-01-15│
│ Tech Startup  │ tech-startup   │ ⏸ 暂停  │   1    │ 2026-02-20│
│ Old Company   │ old-company    │ 📦 归档 │   0    │ 2025-11-01│
└────────────────────────────────────────────────────────────────┘
```

---

### 3.2 组织管理模块 (Organization Management)

#### 3.2.1 功能列表

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 组织列表 | 查看租户下所有组织 | `GET /api/organizations?tenant_id=:id` | `OrganizationList.tsx` |
| 组织详情 | 基本信息、团队列表、成员统计 | `GET /api/organizations/:id` | `OrganizationDetail.tsx` |
| 创建组织 | 新建组织 | `POST /api/organizations` | `OrganizationCreate.tsx` |
| 编辑组织 | 更新组织信息 | `PUT /api/organizations/:id` | `OrganizationEdit.tsx` |
| 成员上限配置 | 设置组织成员数量上限 | `PUT /api/organizations/:id/settings` | 内置于详情 |

#### 3.2.2 API 设计

```typescript
// GET /api/organizations
interface OrganizationListResponse {
  organizations: OrganizationSummary[];
  total: number;
}

interface OrganizationSummary {
  id: string;
  tenant_id: string;
  name: string;
  slug: string;
  member_count: number;
  member_limit?: number;
  created_at: string;
}

// POST /api/organizations
interface CreateOrganizationRequest {
  tenant_id: string;
  name: string;
  slug: string;
  member_limit?: number;
}
```

---

### 3.3 团队管理模块 (Team Management)

#### 3.3.1 功能列表

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 团队列表 | 查看组织下所有团队 | `GET /api/teams?organization_id=:id` | `TeamList.tsx` |
| 团队详情 | 基本信息、成员列表、资源 | `GET /api/teams/:id` | `TeamDetail.tsx` |
| 创建团队 | 新建团队 | `POST /api/teams` | `TeamCreate.tsx` |
| 编辑团队 | 更新团队信息 | `PUT /api/teams/:id` | `TeamEdit.tsx` |
| 设置父团队 | 建立团队层级 | `PUT /api/teams/:id/parent` | 内置于编辑 |
| 团队成员管理 | 添加/移除成员 | `POST/DELETE /api/teams/:id/members` | `TeamMembers.tsx` |

#### 3.3.2 页面原型

```
┌────────────────────────────────────────────────────────────────┐
│ 团队详情 > 工程部                                 [+ 邀请成员]  │
├────────────────────────────────────────────────────────────────┤
│ 基本信息                                                        │
│ ┌─────────────────────────────────────────────────────────┐   │
│ │ 名称：工程部        代码：ENG                            │   │
│ │ 描述：负责公司核心产品研发                                 │   │
│ │ 父团队：无                                               │   │
│ │ [编辑]                                                   │   │
│ └─────────────────────────────────────────────────────────┘   │
├────────────────────────────────────────────────────────────────┤
│ 团队成员 (5)                                                    │
│ ┌─────────────────────────────────────────────────────────┐   │
│ │ 张三 (负责人)    zhang@acme.com    [ OrgAdmin ▼ ] [移除]│   │
│ │ 李四 (成员)      li@acme.com       [ Member   ▼ ] [移除]│   │
│ │ 王五 (成员)      wang@acme.com     [ Member   ▼ ] [移除]│   │
│ └─────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────┘
```

---

### 3.4 成员与 RBAC 权限模块

#### 3.4.1 功能列表

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 成员列表 | 查看所有成员、搜索、状态过滤 | `GET /api/members` | `MemberList.tsx` |
| 成员详情 | 基本信息、角色、权限 | `GET /api/members/:id` | `MemberDetail.tsx` |
| 邀请成员 | 发送邀请邮件 | `POST /api/members/invite` | `MemberInvite.tsx` |
| 接受邀请 | 用户接受邀请 | `POST /api/members/:id/accept` | `InviteAccept.tsx` |
| 角色分配 | 分配/变更角色 | `POST /api/members/:id/role` | 内置于详情/列表 |
| 暂停成员 | 暂停成员资格 | `POST /api/members/:id/suspend` | 内置于详情 |
| 移除成员 | 移除成员 | `POST /api/members/:id/remove` | 内置于详情 |
| 权限检查 | 检查用户权限 | `POST /api/rbac/check` | 内部使用 |

#### 3.4.2 角色定义

```typescript
enum MembershipRole {
  PlatformAdmin = 'platform_admin',  // 平台管理员 - 所有权限
  OrgAdmin = 'org_admin',            // 组织管理员 - 组织内所有权限
  TeamAdmin = 'team_admin',          // 团队管理员 - 团队内所有权限
  Member = 'member',                 // 普通成员 - 基础执行权限
  Viewer = 'viewer',                 // 观察者 - 只读权限
}
```

#### 3.4.3 权限矩阵

| 权限 | PlatformAdmin | OrgAdmin | TeamAdmin | Member | Viewer |
|------|---------------|----------|-----------|--------|--------|
| 租户读 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 租户写 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 组织读 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 组织写 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 团队读 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 团队写 | ✅ | ✅ | ✅ | ❌ | ❌ |
| Agent 读 | ✅ | ✅ | ✅ | ✅ | ✅ |
| Agent 执行 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 工具执行 | ✅ | High 及以下 | Medium 及以下 | Low 及以下 | ❌ |

#### 3.4.4 页面原型

```
┌────────────────────────────────────────────────────────────────┐
│ 成员管理                                    [+ 邀请成员]        │
├────────────────────────────────────────────────────────────────┤
│ 搜索：[________________]  角色：[全部 ▼]  状态：[全部 ▼]       │
├────────────────────────────────────────────────────────────────┤
│ 邮箱              │ 角色           │ 状态    │ 加入时间  │ 操作 │
├───────────────────┼────────────────┼─────────┼───────────┼──────┤
│ admin@acme.com    │ PlatformAdmin  │ ✅ 活跃 │ 2026-01-01│ [⋮] │
│ zhang@acme.com    │ OrgAdmin       │ ✅ 活跃 │ 2026-01-15│ [⋮] │
│ li@acme.com       │ Member         │ ✅ 活跃 │ 2026-02-01│ [⋮] │
│ wang@acme.com     │ Member         │ ⏸ 暂停 │ 2026-02-10│ [⋮] │
│ new@example.com   │ Member         │ ⏳ 待处理│ 2026-04-01│ [⋮] │
└────────────────────────────────────────────────────────────────┘
```

---

### 3.5 审计日志增强模块

#### 3.5.1 功能列表

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 日志列表 | 查看审计日志、分页 | `GET /api/audit-logs` | `AuditLogList.tsx` |
| 高级过滤 | 按时间/用户/操作/资源过滤 | `GET /api/audit-logs?filters` | 内置于列表 |
| 日志详情 | 查看操作详情 | `GET /api/audit-logs/:id` | `AuditLogDetail.tsx` (Modal) |
| 导出功能 | 导出 CSV/JSON | `GET /api/audit-logs/export` | 内置于列表 |
| 操作热力图 | 按时间段展示操作频次 | 前端聚合计算 | `ActivityHeatmap.tsx` |

#### 3.5.2 API 设计

```typescript
// GET /api/audit-logs
interface AuditLogListRequest {
  tenant_id?: string;
  organization_id?: string;
  user_id?: string;
  action_type?: string;
  resource_type?: string;
  start_time?: string;
  end_time?: string;
  limit?: number;
  offset?: number;
}

interface AuditLogRecord {
  id: string;
  tenant_id: string;
  organization_id?: string;
  user_id?: string;
  action: string;
  resource_type: string;
  resource_id: string;
  details?: Record<string, any>;
  ip_address?: string;
  user_agent?: string;
  created_at: string;
}

// GET /api/audit-logs/export
interface ExportAuditLogRequest {
  tenant_id: string;
  start_time: string;
  end_time: string;
  format: 'csv' | 'json';
}
```

#### 3.5.3 页面原型

```
┌────────────────────────────────────────────────────────────────┐
│ 审计日志                               [📥 导出] [📊 热力图]    │
├────────────────────────────────────────────────────────────────┤
│ 时间：[2026-03-01 ~ 2026-04-01]  用户：[全部 ▼]  操作：[全部 ▼]│
│ 资源类型：[全部 ▼]  [查询] [重置]                                │
├────────────────────────────────────────────────────────────────┤
│ 时间                │ 用户           │ 操作       │ 资源       │
├─────────────────────┼────────────────┼────────────┼────────────┤
│ 2026-04-01 10:30:15 │ admin@acme.com │ 创建租户   │ tenant-1   │
│ 2026-04-01 09:15:00 │ zhang@acme.com │ 邀请成员   │ member-123 │
│ 2026-04-01 08:00:00 │ system         │ 暂停租户   │ tenant-2   │
└────────────────────────────────────────────────────────────────┘
```

---

### 3.6 模型用量分析模块

#### 3.6.1 功能列表

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 用量概览 | Token 消耗趋势、成本估算 | `GET /api/usage/summary` | `UsageDashboard.tsx` |
| 按模型分解 | 各模型用量占比、成本对比 | `GET /api/usage/by-model` | `UsageByModel.tsx` |
| 按用户分解 | Top 用户用量排行 | `GET /api/usage/by-user` | `UsageByUser.tsx` |
| 按组织分解 | 各组织用量对比 | `GET /api/usage/by-org` | `UsageByOrg.tsx` |
| 预算设置 | 设置用量预算阈值 | `POST /api/budgets` | `BudgetSettings.tsx` |
| 预算告警 | 超阈值告警记录 | `GET /api/budgets/alerts` | `BudgetAlerts.tsx` |

#### 3.6.2 API 设计

```typescript
// GET /api/usage/summary
interface UsageSummaryResponse {
  period: { start: string; end: string };
  total_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
  estimated_cost: number;
  currency: string;
  trend: {
    date: string;
    tokens: number;
    cost: number;
  }[];
}

// GET /api/usage/by-model
interface UsageByModelResponse {
  models: {
    model_name: string;
    provider: string;
    total_tokens: number;
    prompt_tokens: number;
    completion_tokens: number;
    estimated_cost: number;
  }[];
}

// POST /api/budgets
interface CreateBudgetRequest {
  tenant_id: string;
  organization_id?: string;
  period: 'daily' | 'weekly' | 'monthly';
  token_limit?: number;
  cost_limit?: number;
  alert_threshold_percent: number; // 例如 80 = 80% 时告警
}
```

#### 3.6.3 页面原型

```
┌────────────────────────────────────────────────────────────────┐
│ 用量分析                                                        │
├────────────────────────────────────────────────────────────────┤
│ 周期：[本周 ▼]  组织：[全部 ▼]                                  │
├────────────────────────────────────────────────────────────────┤
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐            │
│ │ 总 Token     │ │ 预估成本     │ │ 日均用量     │            │
│ │ 2.5M         │ │ $12.50       │ │ 357K         │            │
│ │ ↑ 12%        │ │ ↑ 8%         │ │ ↓ 3%         │            │
│ └──────────────┘ └──────────────┘ └──────────────┘            │
├────────────────────────────────────────────────────────────────┤
│ Token 消耗趋势 (7 天)                                           │
│ ┌───────────────────────────────────────────────────────────┐ │
│ │  [折线图：Mon Tue Wed Thu Fri Sat Sun]                    │ │
│ └───────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────────┤
│ 按模型分布                           按用户 Top 5               │
│ ┌───────────────┐                 ┌─────────────────────────┐ │
│ │  [饼图]       │                 │ 1. admin@acme.com  450K │ │
│ │               │                 │ 2. zhang@acme.com  320K │ │
│ │ - GPT-4  60%  │                 │ 3. li@acme.com     280K │ │
│ │ - Claude 30%  │                 │ 4. wang@acme.com   150K │ │
│ │ - Other  10%  │                 │ 5. others          800K │ │
│ └───────────────┘                 └─────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

---

### 3.7 SaaS 订阅与计费模块

#### 3.7.1 功能列表

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 套餐列表 | 查看可用套餐 | `GET /api/plans` | `PlanList.tsx` |
| 当前套餐 | 查看当前订阅 | `GET /api/subscriptions` | `CurrentPlan.tsx` |
| 变更套餐 | 升级/降级套餐 | `POST /api/subscriptions/change` | `ChangePlan.tsx` |
| 用量统计 | 当前周期用量 | `GET /api/usage/current` | `UsageCurrent.tsx` |
| 账单历史 | 查看历史账单 | `GET /api/billing/invoices` | `BillingHistory.tsx` |
| 支付方式 | 管理支付方式 | `GET/PUT /api/billing/payment-method` | `PaymentMethod.tsx` |

#### 3.7.2 套餐设计

```typescript
interface SubscriptionPlan {
  id: string;
  name: string;
  description: string;
  price_monthly: number;
  currency: string;
  features: {
    max_tenants: number;
    max_organizations: number;
    max_teams: number;
    max_members: number;
    token_quota_monthly: number;
    max_agents: number;
    priority_support: boolean;
    custom_integrations: boolean;
  };
}

// 预设套餐
const PLANS = {
  FREE: {
    name: '免费版',
    price: 0,
    features: {
      max_tenants: 1,
      max_organizations: 1,
      max_teams: 3,
      max_members: 5,
      token_quota_monthly: 100_000,
      max_agents: 3,
    },
  },
  PRO: {
    name: '专业版',
    price: 29,
    features: {
      max_tenants: 1,
      max_organizations: 5,
      max_teams: 20,
      max_members: 50,
      token_quota_monthly: 1_000_000,
      max_agents: 20,
      priority_support: true,
    },
  },
  ENTERPRISE: {
    name: '企业版',
    price: 99,
    features: {
      max_tenants: 10,
      max_organizations: -1, // 无限制
      max_teams: -1,
      max_members: -1,
      token_quota_monthly: 10_000_000,
      max_agents: -1,
      priority_support: true,
      custom_integrations: true,
    },
  },
};
```

---

### 3.8 技能市场与知识库管理模块

#### 3.8.1 技能市场功能

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 技能浏览 | 查看所有可用技能 | `GET /api/skills` | `SkillMarketplace.tsx` |
| 技能详情 | 查看技能说明、配置 | `GET /api/skills/:id` | `SkillDetail.tsx` |
| 启用/禁用 | 启用或禁用技能 | `POST /api/skills/:id/enable`<br>`POST /api/skills/:id/disable` | 内置于列表/详情 |
| 技能配置 | 配置技能参数 | `PUT /api/skills/:id/config` | `SkillConfig.tsx` |

#### 3.8.2 知识库功能

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 知识库列表 | 查看所有知识库 | `GET /api/knowledge-bases` | `KnowledgeBaseList.tsx` |
| 创建知识库 | 新建知识库 | `POST /api/knowledge-bases` | `KnowledgeBaseCreate.tsx` |
| 文档上传 | 上传文档 | `POST /api/knowledge-bases/:id/documents` | `DocumentUpload.tsx` |
| 文档列表 | 查看已上传文档 | `GET /api/knowledge-bases/:id/documents` | `DocumentList.tsx` |
| 文档索引状态 | 查看索引进度 | `GET /api/knowledge-bases/:id/indexing-status` | 内置于详情 |
| RAG 检索测试 | 测试检索效果 | `POST /api/knowledge-bases/:id/search-test` | `RagTest.tsx` |

---

### 3.9 自动化告警模块

#### 3.9.1 功能列表

| 功能 | 描述 | 后端 API | 前端组件 |
|------|------|----------|----------|
| 告警规则列表 | 查看所有告警规则 | `GET /api/alerts/rules` | `AlertRuleList.tsx` |
| 创建规则 | 新建告警规则 | `POST /api/alerts/rules` | `AlertRuleCreate.tsx` |
| 编辑规则 | 更新规则 | `PUT /api/alerts/rules/:id` | `AlertRuleEdit.tsx` |
| 告警历史 | 查看触发的告警 | `GET /api/alerts/history` | `AlertHistory.tsx` |
| 通知配置 | 配置通知渠道 | `GET/PUT /api/alerts/notification-channels` | `NotificationChannels.tsx` |

#### 3.9.2 告警规则设计

```typescript
interface AlertRule {
  id: string;
  name: string;
  description?: string;
  enabled: boolean;
  metric: 'error_rate' | 'latency_p99' | 'token_usage' | 'agent_failures';
  operator: '>' | '<' | '>=' | '<=' | '==';
  threshold: number;
  window_minutes: number; // 统计窗口
  severity: 'low' | 'medium' | 'high' | 'critical';
  notification_channels: string[]; // channel IDs
  created_at: string;
  updated_at: string;
}

// 预设告警规则模板
const ALERT_TEMPLATES = [
  {
    name: 'LLM 错误率高',
    metric: 'error_rate',
    operator: '>',
    threshold: 0.05, // 5%
    window_minutes: 5,
    severity: 'high',
  },
  {
    name: 'API 延迟过高',
    metric: 'latency_p99',
    operator: '>',
    threshold: 5000, // 5000ms
    window_minutes: 5,
    severity: 'medium',
  },
  {
    name: 'Token 用量超阈值',
    metric: 'token_usage',
    operator: '>',
    threshold: 1_000_000,
    window_minutes: 60,
    severity: 'low',
  },
];
```

---

## 4. 技术架构设计

### 4.1 后端 API 层设计

#### 4.1.1 路由结构

```
/api
├── /tenants              # 租户管理
│   ├── GET    /          # 列表
│   ├── POST   /          # 创建
│   ├── GET    /:id       # 详情
│   ├── PUT    /:id       # 更新
│   ├── POST   /:id/suspend
│   ├── POST   /:id/restore
│   └── POST   /:id/archive
├── /organizations        # 组织管理
│   ├── GET    /          # 列表 (支持?tenant_id=过滤)
│   ├── POST   /          # 创建
│   ├── GET    /:id       # 详情
│   ├── PUT    /:id       # 更新
│   └── PUT    /:id/settings
├── /teams               # 团队管理
│   ├── GET    /          # 列表 (支持?organization_id=过滤)
│   ├── POST   /          # 创建
│   ├── GET    /:id       # 详情
│   ├── PUT    /:id       # 更新
│   ├── PUT    /:id/parent
│   └── /:id/members     # 团队成员
├── /members             # 成员管理
│   ├── GET    /          # 列表
│   ├── POST   /invite    # 邀请
│   ├── GET    /:id       # 详情
│   ├── POST   /:id/accept
│   ├── POST   /:id/role
│   ├── POST   /:id/suspend
│   └── POST   /:id/remove
├── /rbac                # RBAC 权限
│   └── POST   /check     # 权限检查
├── /audit-logs          # 审计日志
│   ├── GET    /          # 列表
│   └── GET    /export    # 导出
├── /usage               # 用量分析
│   ├── GET    /summary   # 概览
│   ├── GET    /by-model  # 按模型
│   ├── GET    /by-user   # 按用户
│   └── GET    /by-org    # 按组织
├── /budgets             # 预算
│   ├── GET    /          # 列表
│   ├── POST   /          # 创建
│   └── GET    /alerts    # 告警
├── /plans               # 套餐
│   ├── GET    /          # 列表
│   └── GET    /current   # 当前套餐
├── /subscriptions       # 订阅
│   ├── GET    /          # 当前订阅
│   └── POST   /change    # 变更
├── /billing             # 计费
│   ├── GET    /invoices  # 账单
│   └── GET/PUT /payment-method
├── /skills              # 技能市场
│   ├── GET    /          # 列表
│   ├── GET    /:id       # 详情
│   ├── POST   /:id/enable
│   ├── POST   /:id/disable
│   └── PUT    /:id/config
├── /knowledge-bases     # 知识库
│   ├── GET    /          # 列表
│   ├── POST   /          # 创建
│   ├── GET    /:id       # 详情
│   ├── GET    /:id/documents
│   ├── POST   /:id/documents
│   └── POST   /:id/search-test
└── /alerts              # 告警
    ├── /rules            # 规则
    ├── /history          # 历史
    └── /notification-channels
```

#### 4.1.2 Handler 实现模式

```rust
// 示例：租户 Handler
pub struct TenantHandlers<S> {
    state: Arc<S>,
}

impl<S> TenantHandlers<S>
where
    S: TenantState + Send + Sync + 'static,
{
    pub fn new(state: Arc<S>) -> Self {
        Self { state }
    }

    pub fn routes(self) -> Router<Arc<S>> {
        Router::new()
            .route("/", get(Self::list).post(Self::create))
            .route("/:id", get(Self::get).put(Self::update))
            .route("/:id/suspend", post(Self::suspend))
            .route("/:id/restore", post(Self::restore))
            .route("/:id/archive", post(Self::archive))
    }

    async fn list(
        State(state): State<Arc<S>>,
        Query(params): Query<TenantListParams>,
    ) -> Result<Json<TenantListResponse>> {
        // ...
    }
}
```

### 4.2 前端架构设计

#### 4.2.1 目录结构

```
web-ui/
├── src/
│   ├── components/
│   │   ├── ui/              # 基础 UI 组件
│   │   ├── layout/          # 布局组件
│   │   └── shared/          # 共享组件
│   ├── pages/
│   │   ├── Dashboard.tsx
│   │   ├── Monitoring.tsx
│   │   ├── Agents.tsx
│   │   ├── ToolPolicies.tsx
│   │   ├── Workflows.tsx
│   │   ├── Settings.tsx
│   │   ├── tenants/
│   │   │   ├── TenantList.tsx
│   │   │   ├── TenantDetail.tsx
│   │   │   └── TenantCreate.tsx
│   │   ├── organizations/
│   │   ├── teams/
│   │   ├── members/
│   │   ├── audit/
│   │   ├── usage/
│   │   ├── billing/
│   │   ├── skills/
│   │   ├── knowledge/
│   │   └── alerts/
│   ├── lib/
│   │   ├── api.ts           # API 客户端
│   │   ├── api-types.ts     # 类型定义
│   │   ├── scope-storage.ts # 作用域存储
│   │   └── utils.ts
│   └── hooks/
│       └── use-admin-scope.ts
```

#### 4.2.2 API 客户端扩展

```typescript
// lib/api.ts 扩展

// === 租户管理 ===
export async function getTenants(params?: { status?: TenantStatus }): Promise<TenantListResponse> {
  const q: Record<string, string> = { ...managementScopeQuery() };
  if (params?.status) q.status = params.status;
  const res = await fetch(buildUrl('/api/tenants', q));
  return handleResponse<TenantListResponse>(res);
}

export async function createTenant(body: CreateTenantRequest): Promise<TenantDetailResponse> {
  const res = await fetch(buildUrl('/api/tenants'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  return handleResponse<TenantDetailResponse>(res);
}

// === 成员管理 ===
export async function getMembers(params?: { 
  role?: MembershipRole;
  status?: MembershipStatus;
}): Promise<MemberListResponse> {
  const q: Record<string, string> = { ...managementScopeQuery() };
  if (params?.role) q.role = params.role;
  if (params?.status) q.status = params.status;
  const res = await fetch(buildUrl('/api/members', q));
  return handleResponse<MemberListResponse>(res);
}

export async function inviteMember(body: InviteMemberRequest): Promise<Membership> {
  const res = await fetch(buildUrl('/api/members/invite'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  return handleResponse<Membership>(res);
}

// === 用量分析 ===
export async function getUsageSummary(period?: string): Promise<UsageSummaryResponse> {
  const res = await fetch(buildUrl('/api/usage/summary', { 
    ...managementScopeQuery(),
    period: period || '7d',
  }));
  return handleResponse<UsageSummaryResponse>(res);
}
```

---

## 5. 数据模型设计

### 5.1 核心实体关系

```
┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│   Tenant    │ 1   * │ Organization│ 1   * │    Team     │
├─────────────┤───────├─────────────┤───────├─────────────┤
│ id          │       │ id          │       │ id          │
│ name        │       │ tenant_id   │       │ org_id      │
│ slug        │       │ name        │       │ name        │
│ status      │       │ slug        │       │ parent_id   │
└─────────────┘       └─────────────┘       └─────────────┘
                              │
                              │ 1..*
                              ▼
                       ┌─────────────┐
                       │  Membership │
                       ├─────────────┤
                       │ id          │
                       │ user_id     │
                       │ email       │
                       │ role        │
                       │ status      │
                       │ org_id      │
                       │ team_id?    │
                       └─────────────┘
```

### 5.2 数据库表设计 (PostgreSQL)

```sql
-- 租户表
CREATE TABLE tenants (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    status VARCHAR(20) DEFAULT 'active',
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 组织表
CREATE TABLE organizations (
    id VARCHAR(36) PRIMARY KEY,
    tenant_id VARCHAR(36) REFERENCES tenants(id),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    member_limit INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, slug)
);

-- 团队表
CREATE TABLE teams (
    id VARCHAR(36) PRIMARY KEY,
    organization_id VARCHAR(36) REFERENCES organizations(id),
    name VARCHAR(255) NOT NULL,
    code VARCHAR(50),
    description TEXT,
    parent_team_id VARCHAR(36) REFERENCES teams(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 成员表
CREATE TABLE memberships (
    id VARCHAR(36) PRIMARY KEY,
    tenant_id VARCHAR(36) REFERENCES tenants(id),
    organization_id VARCHAR(36) REFERENCES organizations(id),
    team_id VARCHAR(36) REFERENCES teams(id),
    user_id VARCHAR(36),
    email VARCHAR(255) NOT NULL,
    role VARCHAR(30) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    tool_policies JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 审计日志表
CREATE TABLE audit_logs (
    id VARCHAR(36) PRIMARY KEY,
    tenant_id VARCHAR(36) REFERENCES tenants(id),
    organization_id VARCHAR(36) REFERENCES organizations(id),
    user_id VARCHAR(36),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(36) NOT NULL,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 用量统计表
CREATE TABLE usage_stats (
    id VARCHAR(36) PRIMARY KEY,
    tenant_id VARCHAR(36) REFERENCES tenants(id),
    organization_id VARCHAR(36) REFERENCES organizations(id),
    user_id VARCHAR(36),
    model_name VARCHAR(100) NOT NULL,
    prompt_tokens BIGINT DEFAULT 0,
    completion_tokens BIGINT DEFAULT 0,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, organization_id, user_id, model_name, period_start)
);

-- 预算表
CREATE TABLE budgets (
    id VARCHAR(36) PRIMARY KEY,
    tenant_id VARCHAR(36) REFERENCES tenants(id),
    organization_id VARCHAR(36) REFERENCES organizations(id),
    period VARCHAR(20) NOT NULL,
    token_limit BIGINT,
    cost_limit DECIMAL(12, 2),
    alert_threshold_percent INTEGER DEFAULT 80,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 订阅表
CREATE TABLE subscriptions (
    id VARCHAR(36) PRIMARY KEY,
    tenant_id VARCHAR(36) REFERENCES tenants(id) UNIQUE,
    plan_id VARCHAR(36) NOT NULL,
    status VARCHAR(20) DEFAULT 'active',
    current_period_start DATE,
    current_period_end DATE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 告警规则表
CREATE TABLE alert_rules (
    id VARCHAR(36) PRIMARY KEY,
    tenant_id VARCHAR(36) REFERENCES tenants(id),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN DEFAULT TRUE,
    metric VARCHAR(50) NOT NULL,
    operator VARCHAR(5) NOT NULL,
    threshold DECIMAL(20, 6) NOT NULL,
    window_minutes INTEGER NOT NULL,
    severity VARCHAR(20) NOT NULL,
    notification_channels JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_memberships_user ON memberships(user_id);
CREATE INDEX idx_memberships_org ON memberships(organization_id);
CREATE INDEX idx_audit_logs_tenant ON audit_logs(tenant_id);
CREATE INDEX idx_audit_logs_created ON audit_logs(created_at DESC);
CREATE INDEX idx_usage_stats_period ON usage_stats(period_start, period_end);
```

---

## 6. 安全设计

### 6.1 认证与授权

1. **JWT 认证**：所有 API 请求需携带有效 JWT Token
2. **作用域验证**：基于 `tenant_id`、`organization_id`、`team_id` 的资源隔离
3. **RBAC 权限检查**：通过 `RbacService` 进行统一权限验证

### 6.2 权限拦截器

```rust
// 中间件示例
pub async fn require_permission<S>(
    State(state): State<Arc<S>>,
    claims: JwtClaims,
    permission: Permission,
) -> Result<(), ApiError> 
where
    S: RbacState + Send + Sync,
{
    let rbac_service = state.rbac_service();
    let has_permission = rbac_service
        .check_permission(&claims.user_id, &claims.tenant_id, &permission)
        .await?;
    
    if !has_permission {
        return Err(ApiError::Forbidden("Insufficient permissions".into()));
    }
    
    Ok(())
}
```

### 6.3 审计日志

所有敏感操作必须记录审计日志：
- 租户/组织/团队的 CRUD 操作
- 成员邀请、角色变更、移除
- 工具策略变更
- 配置修改

---

## 7. 实施计划

### 7.1 Phase 1 (1-2 周) - 基础管理功能

**目标**：补齐基础管理能力

**任务**：
- [ ] 后端：租户 CRUD API (`/api/tenants`)
- [ ] 后端：组织 CRUD API (`/api/organizations`)
- [ ] 后端：团队 CRUD API (`/api/teams`)
- [ ] 后端：成员列表/邀请 API (`/api/members`)
- [ ] 前端：租户管理页面
- [ ] 前端：组织管理页面
- [ ] 前端：团队管理页面
- [ ] 前端：成员管理页面

### 7.2 Phase 2 (2-3 周) - RBAC 与审计

**目标**：完善权限体系与审计

**任务**：
- [ ] 后端：RBAC 权限检查 API (`/api/rbac/check`)
- [ ] 后端：角色分配 API (`/api/members/:id/role`)
- [ ] 后端：审计日志增强 (过滤、导出)
- [ ] 前端：权限检查集成
- [ ] 前端：审计日志页面增强
- [ ] 前端：操作热力图

### 7.3 Phase 3 (2 周) - 用量分析

**目标**：实现用量监控与预算

**任务**：
- [ ] 后端：用量统计 API (`/api/usage/*`)
- [ ] 后端：预算管理 API (`/api/budgets`)
- [ ] 后端：预算告警服务
- [ ] 前端：用量 Dashboard
- [ ] 前端：预算设置页面

### 7.4 Phase 4 (1-2 周) - SaaS 功能

**目标**：支持 SaaS 化运营

**任务**：
- [ ] 后端：套餐管理 API (`/api/plans`)
- [ ] 后端：订阅管理 API (`/api/subscriptions`)
- [ ] 后端：账单 API (`/api/billing`)
- [ ] 前端：套餐页面
- [ ] 前端：订阅与账单页面

### 7.5 Phase 5 (1-2 周) - 技能与知识

**目标**：完善技能与知识管理

**任务**：
- [ ] 后端：技能市场 API (`/api/skills`)
- [ ] 后端：知识库 API (`/api/knowledge-bases`)
- [ ] 前端：技能市场页面
- [ ] 前端：知识库管理页面

### 7.6 Phase 6 (1 周) - 自动化告警

**目标**：实现告警系统

**任务**：
- [ ] 后端：告警规则 API (`/api/alerts/rules`)
- [ ] 后端：告警历史 API (`/api/alerts/history`)
- [ ] 后端：通知渠道 API
- [ ] 前端：告警配置页面

---

## 8. 成功标准

### 8.1 功能完整性

- [ ] 所有计划模块功能完整实现
- [ ] API 接口通过单元测试覆盖率达 80%+
- [ ] 前端页面交互流畅、无明显 bug

### 8.2 性能指标

- [ ] 列表页面加载时间 < 500ms
- [ ] API 响应时间 P95 < 200ms
- [ ] 支持 100+ 并发管理用户

### 8.3 安全指标

- [ ] 所有 API 通过 JWT 认证
- [ ] RBAC 权限检查覆盖所有敏感操作
- [ ] 审计日志记录 100% 敏感操作

---

## 9. 风险与依赖

### 9.1 技术风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 后端领域服务未完成 | 高 | 优先完成后端服务层 |
| 数据库 schema 变更 | 中 | 使用 migration 工具管理 |
| 前端组件复用性低 | 中 | 提前设计基础组件库 |

### 9.2 依赖项

- Postgres 数据库 (已支持)
- JWT 认证服务 (已实现)
- 审计日志服务 (已实现)
- 用量统计收集 (需完善)

---

## 10. 附录

### 10.1 API 类型定义

```typescript
// lib/api-types.ts 扩展

// === 租户相关类型 ===
export interface TenantSummary {
  id: string;
  name: string;
  slug: string;
  status: 'active' | 'suspended' | 'archived';
  organization_count: number;
  created_at: string;
}

export interface TenantDetailResponse extends TenantSummary {
  description?: string;
  updated_at: string;
  organizations: OrganizationSummary[];
  usage_stats?: UsageStats;
}

export interface CreateTenantRequest {
  name: string;
  slug: string;
  description?: string;
}

// === 成员相关类型 ===
export type MembershipRole = 'platform_admin' | 'org_admin' | 'team_admin' | 'member' | 'viewer';
export type MembershipStatus = 'pending' | 'active' | 'suspended' | 'removed';

export interface Membership {
  id: string;
  tenant_id: string;
  organization_id: string;
  team_id?: string;
  user_id?: string;
  email: string;
  role: MembershipRole;
  status: MembershipStatus;
  created_at: string;
  updated_at: string;
}

export interface InviteMemberRequest {
  tenant_id: string;
  organization_id: string;
  team_id?: string;
  email: string;
  role: MembershipRole;
}

// === 用量相关类型 ===
export interface UsageStats {
  total_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
  estimated_cost: number;
  currency: string;
}

export interface UsageSummaryResponse extends UsageStats {
  period: { start: string; end: string };
  trend: {
    date: string;
    tokens: number;
    cost: number;
  }[];
}
```

---

**文档结束**

此设计文档涵盖 Bee Admin 管理端的完整功能规划，包括：
- 9 大功能模块的详细设计
- 后端 API 路由规划
- 前端组件设计
- 数据库表设计
- 安全与权限设计
- 分阶段实施计划

请审阅并提出修改意见，确认后即可开始实施。
