# Bee Gateway & SaaS 架构重构设计方案

**创建日期**: 2026-03-28
**状态**: 已批准
**作者**: Claude Code

---

## 一、架构概述

### 1.1 架构目标

支撑企业级 SaaS 产品化，实现：
- **多租户隔离** - 租户 → 组织 → 团队 → 用户的四级层级
- **细粒度权限** - RBAC + 工具策略双控制
- **完整审计** - 所有关键操作可追溯
- **水平扩展** - Gateway 可横向扩容，支持千级并发

### 1.2 架构原则

| 原则 | 说明 |
|------|------|
| **领域驱动** | 业务逻辑内聚在领域层，基础设施可替换 |
| **端口适配器** | 外部依赖通过 Port trait 抽象，易于测试和替换 |
| **事件驱动** | 领域事件解耦模块，支持后续事件溯源 |
| **事务边界** | 每个聚合根一个事务，跨聚合用最终一致性 |

---

## 二、整体架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Interface Layer                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   Web UI     │  │  WebSocket   │  │   REST API   │              │
│  │  (React)     │  │   Gateway    │  │   (Axum)     │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Application Layer                               │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                   Command Handlers                           │    │
│  │  - CreateTenant  - InviteMember  - AssignRole                │    │
│  │  - CreateOrg     - AcceptInvite   - RevokeRole               │    │
│  │  - CreateTeam    - ChangeRole     - SetToolPolicy            │    │
│  └─────────────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    Query Handlers                            │    │
│  │  - GetTenant     - ListMembers   - GetAuditLog               │    │
│  │  - GetOrg        - ListTeams       - GetToolPolicy           │    │
│  │  - GetTeam       - ListUsers       - SearchAuditLog          │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
                              │
            ┌─────────────────┼─────────────────┐
            ▼                 ▼                 ▼
┌─────────────────┐ ┌─────────────────┐ ┌────────────────────┐
│  Domain Layer   │ │  Domain Events  │ │   Application      │
│   (Core)        │ │                 │ │   Events           │
│                 │ │ - TenantCreated │ │ - UserLoggedIn     │
│ ┌─────────────┐ │ │ - OrgCreated    │ │ - SessionStarted   │
│ │Tenant Agg.  │ │ │ - TeamCreated   │ │ - TaskQueued       │
│ └─────────────┘ │ │ - MemberInvited │ │ - ToolExecuted     │
│ ┌─────────────┐ │ │ - RoleAssigned  │ │ - AuditLogged      │
│ │ Org Agg.    │ │ │ - ToolPolicyChg │ └────────────────────┘
│ └─────────────┘ │ └─────────────────┘
│ ┌─────────────┐ │
│ │ Team Agg.   │ │
│ └─────────────┘ │
│ ┌─────────────┐ │
│ │ Agent Agg.  │ │
│ └─────────────┘ │
└─────────────────┘
            │
            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   Infrastructure Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  Postgres    │  │    Kafka     │  │     JWT      │              │
│  │ Repository   │  │ Event Bus    │  │   Auth       │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   Email      │  │    Cache     │  │   Metrics    │              │
│  │   SMTP       │  │   (Redis)    │  │  (Prometheus)│              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 三、模块划分

### 3.1 领域模块

```
src/
├── domain/
│   ├── tenant/              # 租户聚合
│   │   ├── mod.rs
│   │   ├── entity.rs        # Tenant, TenantStatus
│   │   ├── value_object.rs  # TenantId, TenantName
│   │   ├── repository.rs    # TenantRepository trait
│   │   ├── service.rs       # TenantDomainService
│   │   └── event.rs         # TenantCreated, TenantDeleted...
│   │
│   ├── organization/        # 组织聚合
│   │   ├── mod.rs
│   │   ├── entity.rs        # Organization, Industry
│   │   ├── value_object.rs  # OrganizationId, Slug
│   │   ├── repository.rs    # OrganizationRepository trait
│   │   └── event.rs
│   │
│   ├── team/                # 团队聚合
│   │   ├── mod.rs
│   │   ├── entity.rs        # Team
│   │   ├── value_object.rs  # TeamId, TeamCode
│   │   ├── repository.rs    # TeamRepository trait
│   │   └── event.rs
│   │
│   ├── member/              # 成员聚合（核心权限）
│   │   ├── mod.rs
│   │   ├── entity.rs        # Membership, MembershipRole
│   │   ├── value_object.rs  # MemberId, Role
│   │   ├── repository.rs    # MembershipRepository trait
│   │   ├── service.rs       # MemberDomainService (权限检查)
│   │   └── event.rs         # MemberInvited, RoleAssigned...
│   │
│   ├── agent/               # Agent 聚合
│   │   ├── mod.rs
│   │   ├── entity.rs        # AgentTemplate, AgentInstance
│   │   ├── value_object.rs  # AgentId, ToolPolicy
│   │   ├── repository.rs    # AgentRepository trait
│   │   └── event.rs
│   │
│   └── audit/               # 审计日志（独立实体）
│       ├── mod.rs
│       ├── entity.rs        # AuditLogRecord, AuditAction
│       ├── repository.rs    # AuditRepository trait
│       └── event.rs
│
├── application/
│   ├── commands/            # 命令处理
│   │   ├── tenant/
│   │   ├── organization/
│   │   ├── team/
│   │   ├── member/
│   │   └── agent/
│   │
│   ├── queries/             # 查询处理
│   │   ├── tenant/
│   │   ├── organization/
│   │   └── member/
│   │
│   └── events/              # 应用事件
│       ├── handler.rs       # 事件处理器
│       └── publisher.rs     # 事件发布
│
├── infrastructure/
│   ├── persistence/
│   │   ├── postgres/        # PostgreSQL 实现
│   │   │   ├── mod.rs
│   │   │   ├── tenant_repo.rs
│   │   │   ├── org_repo.rs
│   │   │   ├── team_repo.rs
│   │   │   ├── member_repo.rs
│   │   │   ├── agent_repo.rs
│   │   │   └── audit_repo.rs
│   │   │
│   │   └── migration/       # 数据库迁移
│   │       ├── mod.rs
│   │       └── migrations/
│   │
│   ├── event_bus/
│   │   ├── mod.rs
│   │   ├── kafka.rs         # Kafka 事件总线
│   │   └── in_memory.rs     # 内存实现（测试用）
│   │
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── jwt.rs           # JWT 认证服务
│   │   ├── claims.rs        # JWT Claims 定义
│   │   └── middleware.rs    # Axum 中间件
│   │
│   └── notification/
│       ├── mod.rs
│       ├── email.rs         # 邮件通知
│       └── webhook.rs       # Webhook 通知
│
└── interfaces/
    ├── http/                # REST API
    │   ├── mod.rs
    │   ├── routes/
    │   ├── handlers/
    │   └── middleware/
    │
    ├── websocket/           # WebSocket Gateway
    │   ├── mod.rs
    │   ├── hub.rs
    │   ├── session.rs
    │   └── handler.rs
    │
    └── web/                 # Web UI (静态文件服务)
        └── mod.rs
```

---

## 四、核心领域模型设计

### 4.1 租户聚合根

```rust
// src/domain/tenant/entity.rs

#[derive(Debug, Clone)]
pub struct TenantId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantStatus {
    Active,
    Suspended,
    Archived,
}

/// 租户聚合根
#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: TenantId,
    pub name: TenantName,
    pub status: TenantStatus,
    pub slug: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 聚合内实体
    organizations: Vec<OrganizationId>,
}

impl Tenant {
    pub fn create(name: String, slug: Option<String>) -> Result<Self, DomainError> {
        let tenant = Self {
            id: TenantId::generate(),
            name: TenantName::new(name)?,
            status: TenantStatus::Active,
            slug,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            organizations: Vec::new(),
        };

        DomainEvent::publish(TenantCreated {
            tenant_id: tenant.id.clone(),
            name: tenant.name.clone(),
        }).await;

        Ok(tenant)
    }

    pub fn add_organization(&mut self, org_id: OrganizationId) {
        self.organizations.push(org_id);
        self.updated_at = Utc::now();
    }

    pub fn suspend(&mut self) {
        self.status = TenantStatus::Suspended;
        DomainEvent::publish(TenantSuspended { tenant_id: self.id.clone() }).await;
    }
}
```

### 4.2 成员聚合根（权限核心）

```rust
// src/domain/member/entity.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipRole {
    PlatformAdmin,
    OrgAdmin,
    TeamAdmin,
    Member,
    Viewer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    // 租户级
    TenantRead,
    TenantWrite,
    TenantDelete,

    // 组织级
    OrgRead,
    OrgWrite,
    OrgDelete,

    // 团队级
    TeamRead,
    TeamWrite,
    TeamDelete,

    // Agent 级
    AgentRead,
    AgentExecute,
    AgentModify,

    // 工具级
    ToolExecute(ToolId),
}

/// 成员聚合根
#[derive(Debug, Clone)]
pub struct Membership {
    pub id: MembershipId,
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub team_id: Option<TeamId>,
    pub user_id: UserId,
    pub role: MembershipRole,
    pub status: MembershipStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Membership {
    pub fn invite(
        tenant_id: TenantId,
        organization_id: OrganizationId,
        team_id: Option<TeamId>,
        user_id: UserId,
        role: MembershipRole,
    ) -> Result<Self, DomainError> {
        let membership = Self {
            id: MembershipId::generate(),
            tenant_id,
            organization_id,
            team_id,
            user_id,
            role,
            status: MembershipStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        DomainEvent::publish(MemberInvited {
            membership_id: membership.id.clone(),
            user_id: membership.user_id.clone(),
            role: membership.role.clone(),
        }).await;

        Ok(membership)
    }

    pub fn accept_invite(&mut self) {
        self.status = MembershipStatus::Active;
        DomainEvent::publish(MemberActivated {
            membership_id: self.id.clone(),
        }).await;
    }

    pub fn change_role(&mut self, new_role: MembershipRole) {
        self.role = new_role;
        self.updated_at = Utc::now();
        DomainEvent::publish(RoleChanged {
            membership_id: self.id.clone(),
            old_role: self.role.clone(),
            new_role: new_role.clone(),
        }).await;
    }

    /// 权限检查
    pub fn has_permission(&self, permission: Permission) -> bool {
        match (&self.role, &permission) {
            (MembershipRole::PlatformAdmin, _) => true,
            (MembershipRole::OrgAdmin, Permission::OrgRead) => true,
            (MembershipRole::OrgAdmin, Permission::OrgWrite) => true,
            (MembershipRole::TeamAdmin, Permission::TeamRead) => true,
            (MembershipRole::TeamAdmin, Permission::TeamWrite) => true,
            (MembershipRole::Member, Permission::AgentExecute) => true,
            _ => false,
        }
    }
}
```

### 4.3 工具策略值对象

```rust
// src/domain/member/value_object.rs

#[derive(Debug, Clone)]
pub struct ToolPolicy {
    /// 允许的工具列表（白名单）
    pub allowed_tools: HashSet<ToolId>,
    /// 拒绝的工具列表（黑名单，优先级更高）
    pub denied_tools: HashSet<ToolId>,
    /// 工具执行配额（每日）
    pub daily_quota: Option<u32>,
    /// 高风险工具需要审批
    pub require_approval_for_high_risk: bool,
}

impl ToolPolicy {
    pub fn can_execute(&self, tool_id: &ToolId, risk_level: ToolRiskLevel) -> bool {
        // 黑名单优先
        if self.denied_tools.contains(tool_id) {
            return false;
        }

        // 高风险工具需要审批
        if risk_level == ToolRiskLevel::High && self.require_approval_for_high_risk {
            return false;
        }

        // 白名单检查
        if !self.allowed_tools.is_empty() {
            return self.allowed_tools.contains(tool_id);
        }

        // 无白名单限制时，只允许低风险工具
        risk_level != ToolRiskLevel::High
    }
}
```

---

## 五、基础设施设计

### 5.1 PostgreSQL 数据库设计

```sql
-- 租户表
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 组织表
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100),
    industry VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, slug)
);

-- 团队表
CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    code VARCHAR(50),
    parent_team_id UUID REFERENCES teams(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 用户表
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    external_user_id VARCHAR(255),
    display_name VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 成员关系表（核心权限）
CREATE TABLE memberships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, organization_id, team_id, user_id)
);

-- 索引
CREATE INDEX idx_memberships_user ON memberships(user_id);
CREATE INDEX idx_memberships_tenant_org ON memberships(tenant_id, organization_id);
CREATE INDEX idx_memberships_team ON memberships(team_id);

-- Agent 模板表
CREATE TABLE agent_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    prompt TEXT,
    tool_ids JSONB NOT NULL DEFAULT '[]',
    model_id VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Agent 实例表
CREATE TABLE agent_instances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    template_id UUID NOT NULL REFERENCES agent_templates(id),
    name VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    prompt_override TEXT,
    tool_ids_override JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 工具策略表
CREATE TABLE tool_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    allowed_tools JSONB NOT NULL DEFAULT '[]',
    denied_tools JSONB NOT NULL DEFAULT '[]',
    daily_quota INTEGER,
    require_approval_for_high_risk BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 审计日志表
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,
    detail_json JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_audit_logs_tenant ON audit_logs(tenant_id);
CREATE INDEX idx_audit_logs_created ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);

-- 会话表
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    agent_instance_id UUID REFERENCES agent_instances(id) ON DELETE SET NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    last_active_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 任务表
CREATE TABLE tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    session_id UUID REFERENCES sessions(id) ON DELETE SET NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 1,
    assignee_agent_id UUID REFERENCES agent_instances(id),
    creator_user_id UUID REFERENCES users(id),
    result TEXT,
    error TEXT,
    progress INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

-- 领域事件表（事件溯源）
CREATE TABLE domain_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type VARCHAR(100) NOT NULL,
    aggregate_type VARCHAR(100) NOT NULL,
    aggregate_id UUID NOT NULL,
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed BOOLEAN NOT NULL DEFAULT FALSE
);

-- 索引
CREATE INDEX idx_domain_events_aggregate ON domain_events(aggregate_type, aggregate_id);
CREATE INDEX idx_domain_events_unprocessed ON domain_events(processed) WHERE NOT processed;
```

### 5.2 Kafka 主题设计

```
# 领域事件主题
topic: bee.domain.events
  partitions: 6
  retention: 7d

  # 事件类型
  - tenant.created
  - tenant.deleted
  - tenant.suspended

  - organization.created
  - organization.deleted

  - team.created
  - team.deleted

  - member.invited
  - member.activated
  - member.deactivated
  - member.role_changed

  - agent.template_created
  - agent.instance_created
  - agent.instance.deleted

  - tool.policy_changed

  - audit.log_created

# 应用事件主题
topic: bee.app.events
  partitions: 3
  retention: 24h

  # 事件类型
  - user.logged_in
  - user.logged_out

  - session.started
  - session.ended

  - task.queued
  - task.started
  - task.completed
  - task.failed

  - tool.executed
  - tool.failed

# 通知主题
topic: bee.notifications
  partitions: 3
  retention: 1d

  # 事件类型
  - notification.email
  - notification.webhook
  - notification.websocket
```

### 5.3 JWT 认证设计

```rust
// src/infrastructure/auth/claims.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeeClaims {
    /// 用户 ID
    pub sub: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 组织 ID
    pub organization_id: String,
    /// 团队 ID（可选）
    pub team_id: Option<String>,
    /// 角色
    pub role: String,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 过期时间
    pub exp: i64,
    /// 签发时间
    pub iat: i64,
}

impl BeeClaims {
    pub fn new(
        user_id: String,
        tenant_id: String,
        organization_id: String,
        team_id: Option<String>,
        role: MembershipRole,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: user_id,
            tenant_id,
            organization_id,
            team_id,
            role: role.to_string(),
            permissions: Self::role_to_permissions(&role),
            exp: now + 86400, // 24 小时
            iat: now,
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission) ||
        self.role == "PlatformAdmin"
    }
}

// src/infrastructure/auth/jwt.rs

pub struct JwtService {
    secret: JwtSecret,
    expiry_secs: u64,
}

impl JwtService {
    pub fn sign(&self, claims: BeeClaims) -> Result<String, AuthError> {
        let header = json!({ "alg": "HS256", "typ": "JWT" });
        let header_b64 = BASE64_URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());

        let claims_b64 = BASE64_URL_SAFE_NO_PAD.encode(
            serde_json::to_string(&claims)?.as_bytes()
        );

        let signature_input = format!("{}.{}", header_b64, claims_b64);
        let signature = self.sign_with_hmac(&signature_input);

        Ok(format!("{}.{}", signature_input, signature))
    }

    pub fn verify(&self, token: &str) -> Result<BeeClaims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidTokenFormat);
        }

        let signature_input = format!("{}.{}", parts[0], parts[1]);
        let expected_signature = self.sign_with_hmac(&signature_input);

        if parts[2] != expected_signature {
            return Err(AuthError::InvalidSignature);
        }

        let claims_bytes = BASE64_URL_SAFE_NO_PAD.decode(parts[1])?;
        let claims: BeeClaims = serde_json::from_slice(&claims_bytes)?;

        if claims.exp < Utc::now().timestamp() {
            return Err(AuthError::TokenExpired);
        }

        Ok(claims)
    }
}
```

---

## 六、应用服务设计

### 6.1 命令处理示例

```rust
// src/application/commands/member/invite_member.rs

#[derive(Debug, Clone)]
pub struct InviteMemberCommand {
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub team_id: Option<TeamId>,
    pub user_id: UserId,
    pub role: MembershipRole,
    pub invited_by: UserId,
}

pub struct InviteMemberHandler {
    member_repo: Arc<dyn MembershipRepository>,
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

#[async_trait]
impl CommandHandler<InviteMemberCommand, InviteMemberResult> for InviteMemberHandler {
    async fn handle(&self, cmd: InviteMemberCommand) -> Result<InviteMemberResult, ApplicationError> {
        // 1. 验证用户是否存在
        let user = self.user_repo.find_by_id(&cmd.user_id).await?
            .ok_or(ApplicationError::UserNotFound)?;

        // 2. 验证邀请人权限
        let inviter_membership = self.member_repo
            .find_by_user_and_org(&cmd.invited_by, &cmd.organization_id)
            .await?
            .ok_or(ApplicationError::PermissionDenied)?;

        if !inviter_membership.has_permission(Permission::TeamWrite) {
            return Err(ApplicationError::PermissionDenied);
        }

        // 3. 检查是否已存在成员关系
        let existing = self.member_repo
            .find_by_user_and_org(&cmd.user_id, &cmd.organization_id)
            .await?;

        if existing.is_some() {
            return Err(ApplicationError::MemberAlreadyExists);
        }

        // 4. 创建成员关系
        let mut membership = Membership::invite(
            cmd.tenant_id,
            cmd.organization_id,
            cmd.team_id,
            cmd.user_id,
            cmd.role,
        )?;

        // 5. 持久化
        self.member_repo.save(&mut membership).await?;

        // 6. 发送邀请邮件（通过事件）
        self.event_publisher.publish(MemberInvitedEvent {
            membership_id: membership.id.clone(),
            user_email: user.email,
            inviter_id: cmd.invited_by,
            organization_name: "".to_string(), // 可从聚合获取
        }).await;

        // 7. 审计日志
        self.event_publisher.publish(AuditLogCreatedEvent {
            tenant_id: cmd.tenant_id,
            action: "MEMBER_INVITED".to_string(),
            resource_type: "MEMBERSHIP".to_string(),
            resource_id: membership.id.to_string(),
            user_id: Some(cmd.invited_by),
        }).await;

        Ok(InviteMemberResult {
            membership_id: membership.id,
            status: membership.status,
        })
    }
}
```

### 6.2 查询处理示例

```rust
// src/application/queries/member/list_members.rs

#[derive(Debug, Clone)]
pub struct ListMembersQuery {
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub team_id: Option<TeamId>,
    pub status_filter: Option<MembershipStatus>,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone)]
pub struct MemberDto {
    pub id: String,
    pub user_id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: String,
    pub status: String,
    pub team_name: Option<String>,
    pub joined_at: DateTime<Utc>,
}

pub struct ListMembersHandler {
    db: Arc<PostgresPool>,
}

#[async_trait]
impl QueryHandler<ListMembersQuery, PaginatedResult<MemberDto>> for ListMembersHandler {
    async fn handle(&self, query: ListMembersQuery) -> Result<PaginatedResult<MemberDto>, ApplicationError> {
        let offset = (query.page - 1) * query.page_size;

        let where_clause = match (&query.team_id, &query.status_filter) {
            (Some(team_id), Some(status)) =>
                "WHERE m.tenant_id = $1 AND m.organization_id = $2 AND m.team_id = $3 AND m.status = $4",
            (Some(team_id), None) =>
                "WHERE m.tenant_id = $1 AND m.organization_id = $2 AND m.team_id = $3",
            (None, Some(status)) =>
                "WHERE m.tenant_id = $1 AND m.organization_id = $2 AND m.status = $3",
            (None, None) =>
                "WHERE m.tenant_id = $1 AND m.organization_id = $2",
        };

        // 查询总数
        let count_sql = format!(
            "SELECT COUNT(*) FROM memberships m {}",
            where_clause
        );

        let total: i64 = sqlx::query_scalar(&count_sql)
            .bind(&query.tenant_id.to_string())
            .bind(&query.organization_id.to_string())
            .bind_optional(&query.team_id.map(|t| t.to_string()))
            .bind_optional(&query.status_filter.map(|s| s.to_string()))
            .fetch_one(&*self.db)
            .await?;

        // 查询数据
        let rows = sqlx::query_as(r#"
            SELECT
                m.id, m.user_id, u.display_name, u.email,
                m.role, m.status, t.name as team_name, m.created_at as joined_at
            FROM memberships m
            JOIN users u ON m.user_id = u.id
            LEFT JOIN teams t ON m.team_id = t.id
            WHERE m.tenant_id = $1 AND m.organization_id = $2
            ORDER BY m.created_at DESC
            LIMIT $3 OFFSET $4
        "#)
        .bind(&query.tenant_id.to_string())
        .bind(&query.organization_id.to_string())
        .bind(query.page_size)
        .bind(offset)
        .fetch_all(&*self.db)
        .await?;

        let items = rows.into_iter().map(|row: MemberRow| {
            MemberDto {
                id: row.id,
                user_id: row.user_id,
                display_name: row.display_name,
                email: row.email,
                role: row.role,
                status: row.status,
                team_name: row.team_name,
                joined_at: row.joined_at,
            }
        }).collect();

        Ok(PaginatedResult {
            items,
            total: total as i32,
            page: query.page,
            page_size: query.page_size,
        })
    }
}
```

---

## 七、Gateway 重新设计

### 7.1 Hub 架构

```rust
// src/interfaces/websocket/hub.rs

pub struct Hub {
    config: HubConfig,

    // 依赖
    session_store: Arc<dyn SessionStore>,
    tenant_resolver: Arc<dyn TenantResolver>,
    auth_service: Arc<JwtService>,
    command_bus: Arc<dyn CommandBus>,
    event_subscriber: Arc<dyn EventSubscriber>,

    // 状态
    connections: Arc<DashMap<ConnectionId, Connection>>,
    shutdown: CancellationToken,
}

impl Hub {
    pub async fn new(config: HubConfig, deps: HubDependencies) -> Self {
        Self {
            config,
            session_store: deps.session_store,
            tenant_resolver: deps.tenant_resolver,
            auth_service: deps.auth_service,
            command_bus: deps.command_bus,
            event_subscriber: deps.event_subscriber,
            connections: Arc::new(DashMap::new()),
            shutdown: CancellationToken::new(),
        }
    }

    pub async fn start(&self) -> Result<(), HubError> {
        // 启动 WebSocket 监听
        let listener = TcpListener::bind(&self.config.bind_addr).await?;

        // 启动事件订阅处理器
        self.start_event_subscriber().await;

        // 启动会话清理定时器
        self.start_session_cleanup().await;

        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => break,

                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let conn = self.create_connection(stream, addr).await;
                            tokio::spawn(async move {
                                conn.handle().await;
                            });
                        }
                        Err(e) => tracing::error!("Accept error: {}", e),
                    }
                }
            }
        }

        Ok(())
    }

    async fn create_connection(
        &self,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Connection {
        Connection {
            stream,
            addr,
            hub: self.clone(),
            session: None,
            auth_state: AuthState::Unauthenticated,
        }
    }
}
```

### 7.2 会话管理

```rust
// src/interfaces/websocket/session.rs

pub struct Session {
    pub id: SessionId,
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub team_id: Option<TeamId>,
    pub user_id: UserId,
    pub agent_instance_id: Option<AgentInstanceId>,

    // 连接信息
    pub connections: HashMap<ConnectionId, ConnectionInfo>,

    // 状态
    pub status: SessionStatus,
    pub cancel_token: Option<CancellationToken>,
    pub last_active: Instant,
}

impl Session {
    pub async fn from_claims(
        claims: &BeeClaims,
        session_store: Arc<dyn SessionStore>,
    ) -> Result<Self, SessionError> {
        // 尝试恢复现有会话
        let existing = session_store
            .find_active_session(&claims.sub)
            .await?;

        if let Some(session) = existing {
            return Ok(session);
        }

        // 创建新会话
        let session = Self {
            id: SessionId::generate(),
            tenant_id: TenantId::new(claims.tenant_id.clone()),
            organization_id: OrganizationId::new(claims.organization_id.clone()),
            team_id: claims.team_id.as_ref().map(TeamId::new),
            user_id: UserId::new(claims.sub.clone()),
            agent_instance_id: None,
            connections: HashMap::new(),
            status: SessionStatus::Idle,
            cancel_token: None,
            last_active: Instant::now(),
        };

        Ok(session)
    }
}
```

---

## 八、迁移计划

### Phase 1: 基础设施准备（2 周）

- [ ] PostgreSQL 数据库初始化
- [ ] Kafka 集群部署（或本地 Docker）
- [ ] SQLx + 迁移框架配置
- [ ] 新领域模型定义

### Phase 2: 领域层实现（3 周）

- [ ] Tenant 聚合 + Repository
- [ ] Organization 聚合 + Repository
- [ ] Team 聚合 + Repository
- [ ] Member 聚合 + Repository（核心权限）
- [ ] Agent 聚合 + Repository
- [ ] 领域事件系统

### Phase 3: 应用层实现（2 周）

- [ ] 命令处理框架
- [ ] 查询处理框架
- [ ] 事件处理器

### Phase 4: 基础设施实现（2 周）

- [ ] PostgreSQL Repository 实现
- [ ] Kafka 事件总线
- [ ] JWT 认证服务

### Phase 5: 接口层实现（2 周）

- [ ] REST API (Axum)
- [ ] WebSocket Gateway
- [ ] 认证中间件

### Phase 6: 迁移与测试（2 周）

- [ ] 数据迁移脚本
- [ ] 集成测试
- [ ] 性能测试

**总预估时间**: 12-14 周

---

## 九、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| PostgreSQL 迁移复杂 | 高 | 分阶段迁移，先读后写，保留回滚能力 |
| Kafka 运维成本 | 中 | 使用托管服务（Confluent/AWS MSK） |
| JWT 安全 | 高 | 使用 HS256/RS256，定期轮换密钥 |
| 领域模型设计不当 | 中 | 先做事件风暴，验证模型再实现 |

---

## 十、下一步

1. **用户确认设计方案** - 本文档
2. **创建实施计划** - 使用 writing-plans skill 创建详细的 implementation plan
3. **分阶段实施** - 按 Phase 逐步推进
