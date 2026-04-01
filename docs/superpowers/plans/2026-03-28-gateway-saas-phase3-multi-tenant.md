# Gateway & SaaS 架构重构 - Phase 3: 多租户与权限 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现完整的四级多租户层级（租户 → 组织 → 团队），完成 RBAC 权限系统，实现工具策略持久化和执行，完善审计日志记录

**Architecture:** 基于 Phase 1-2 完成的领域模型和 Repository，实现 Organization 和 Team 聚合根的持久化和服务层，完善权限检查

**Tech Stack:** Rust, PostgreSQL, SQLx, Kafka, JWT, tokio, axum

**Spec Reference:** `docs/superpowers/specs/2026-03-28-gateway-saas-architecture-design.md`

**Phase 2 Completion:**
- ✅ Tenant Repository PostgreSQL 实现
- ✅ Member Repository PostgreSQL 实现
- ✅ TenantDomainService
- ✅ MemberDomainService
- ✅ ToolPolicyService
- ✅ 应用层命令/查询处理
- ✅ Gateway JWT 认证集成
- ✅ 集成测试

---

## File Structure

### New Files to Create

**Domain Layer:**
- `src/domain/organization/mod.rs`, `entity.rs`, `value_object.rs`, `repository.rs`, `event.rs`
- `src/domain/team/mod.rs`, `entity.rs`, `value_object.rs`, `repository.rs`, `event.rs`

**Domain Services:**
- `src/domain/service/organization_service.rs`
- `src/domain/service/team_service.rs`
- `src/domain/service/rbac_service.rs`

**Repository Implementations:**
- `src/domain/organization/repository/postgres.rs`
- `src/domain/team/repository/postgres.rs`

**Application Layer Commands:**
- `src/application/commands/create_organization.rs`
- `src/application/commands/create_team.rs`
- `src/application/commands/assign_role.rs`
- `src/application/commands/set_tool_policy.rs`

**Application Layer Queries:**
- `src/application/queries/get_organization.rs`
- `src/application/queries/list_teams.rs`
- `src/application/queries/get_tool_policy.rs`

**Infrastructure:**
- `src/infrastructure/audit/postgres.rs` - 审计日志 PostgreSQL 实现

**Tests:**
- `tests/integration/rbac_test.rs`
- `tests/integration/multi_tenant_test.rs`

---

## Prerequisites

确保 Phase 1-2 已完成：
- 数据库迁移已应用（包含 organizations, teams 表）
- Tenant 和 Member 领域模型已定义
- Repository 和服务层模式已建立

---

## Task 1: Organization 领域模型

**Files:**
- Create: `src/domain/organization/mod.rs`
- Create: `src/domain/organization/entity.rs`
- Create: `src/domain/organization/value_object.rs`
- Create: `src/domain/organization/repository.rs`
- Create: `src/domain/organization/event.rs`

- [ ] **Step 1: 创建值对象**

```rust
// src/domain/organization/value_object.rs
use crate::domain::tenant::TenantId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationId(pub String);

impl OrganizationId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct OrganizationName(pub String);

impl OrganizationName {
    pub fn new(name: String) -> Result<Self, OrganizationError> {
        if name.trim().is_empty() {
            return Err(OrganizationError::InvalidName("Name cannot be empty"));
        }
        if name.len() > 255 {
            return Err(OrganizationError::InvalidName("Name too long (max 255 chars)"));
        }
        Ok(Self(name))
    }
}

#[derive(Debug, Clone)]
pub struct OrganizationSlug(pub String);
// Similar validation as TenantSlug
```

- [ ] **Step 2: 创建聚合根**

```rust
// src/domain/organization/entity.rs
use crate::domain::tenant::TenantId;
use crate::domain::organization::{OrganizationId, OrganizationName, OrganizationSlug};

pub struct Organization {
    id: OrganizationId,
    tenant_id: TenantId,
    name: OrganizationName,
    slug: OrganizationSlug,
    industry: Option<String>,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Organization {
    pub fn create(
        tenant_id: TenantId,
        name: OrganizationName,
        slug: OrganizationSlug,
    ) -> Result<Self, OrganizationError> {
        // Create organization
    }

    pub fn update_name(&mut self, name: OrganizationName) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    // Other methods
}
```

- [ ] **Step 3: 创建 Repository trait**

```rust
// src/domain/organization/repository.rs
#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    type Error;

    async fn save(&self, org: &Organization) -> Result<(), Self::Error>;
    async fn find_by_id(&self, id: &OrganizationId) -> Result<Option<Organization>, Self::Error>;
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Organization>, Self::Error>;
    async fn find_by_slug(&self, tenant_id: &TenantId, slug: &str) -> Result<Option<Organization>, Self::Error>;
    async fn delete(&self, id: &OrganizationId) -> Result<(), Self::Error>;
}
```

- [ ] **Step 4: 创建领域事件**

```rust
// src/domain/organization/event.rs
pub enum OrganizationEvent {
    Created { id: OrganizationId, tenant_id: TenantId, name: String },
    Updated { id: OrganizationId, name: String },
    Deleted { id: OrganizationId },
}
```

- [ ] **Step 5: Commit**

```bash
git add src/domain/organization/
git commit -m "feat(domain): add Organization aggregate"
```

---

## Task 2: Team 领域模型

**Files:**
- Create: `src/domain/team/mod.rs`
- Create: `src/domain/team/entity.rs`
- Create: `src/domain/team/value_object.rs`
- Create: `src/domain/team/repository.rs`
- Create: `src/domain/team/event.rs`

- [ ] **Step 1: 创建值对象**

```rust
// Similar pattern as Organization
pub struct TeamId(pub String);
pub struct TeamName(pub String);
pub struct TeamCode(pub String);
```

- [ ] **Step 2: 创建聚合根**

```rust
pub struct Team {
    id: TeamId,
    tenant_id: TenantId,
    organization_id: OrganizationId,
    name: TeamName,
    code: Option<TeamCode>,
    description: Option<String>,
    parent_team_id: Option<TeamId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

- [ ] **Step 3: 创建 Repository trait**

- [ ] **Step 4: Commit**

```bash
git add src/domain/team/
git commit -m "feat(domain): add Team aggregate"
```

---

## Task 3: Organization Repository PostgreSQL 实现

**Files:**
- Create: `src/domain/organization/repository/postgres.rs`
- Modify: `src/domain/organization/repository.rs`

- [ ] **Step 1: 实现 PostgresOrganizationRepository**

```rust
pub struct PostgresOrganizationRepository {
    pool: PgPool,
}

#[async_trait]
impl OrganizationRepository for PostgresOrganizationRepository {
    type Error = OrganizationError;

    async fn save(&self, org: &Organization) -> Result<(), Self::Error> {
        // UPSERT logic
    }

    async fn find_by_id(&self, id: &OrganizationId) -> Result<Option<Organization>, Self::Error> {
        // Query by ID
    }

    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Organization>, Self::Error> {
        // Query all organizations in a tenant
    }
}
```

- [ ] **Step 2: 编写集成测试**

- [ ] **Step 3: Commit**

```bash
git add src/domain/organization/repository/postgres.rs tests/
git commit -m "feat(domain): implement PostgresOrganizationRepository"
```

---

## Task 4: Team Repository PostgreSQL 实现

**Files:**
- Create: `src/domain/team/repository/postgres.rs`
- Modify: `src/domain/team/repository.rs`

- [ ] **Step 1: 实现 PostgresTeamRepository**

- [ ] **Step 2: 编写集成测试**

- [ ] **Step 3: Commit**

```bash
git add src/domain/team/repository/postgres.rs tests/
git commit -m "feat(domain): implement PostgresTeamRepository"
```

---

## Task 5: Organization 服务层

**Files:**
- Create: `src/domain/service/organization_service.rs`

- [ ] **Step 1: 实现 OrganizationDomainService**

```rust
pub struct OrganizationDomainService<OR, EP> {
    org_repo: Arc<OR>,
    event_publisher: Arc<EP>,
}

impl<OR, EP> OrganizationDomainService<OR, EP>
where
    OR: OrganizationRepository + 'static,
    EP: EventPublisher + 'static,
{
    pub fn create_organization(
        &self,
        tenant_id: TenantId,
        name: OrganizationName,
        slug: OrganizationSlug,
    ) -> Result<Organization, OrganizationError> {
        // Create and save
    }
}
```

- [ ] **Step 2: Commit**

---

## Task 6: Team 服务层

**Files:**
- Create: `src/domain/service/team_service.rs`

- [ ] **Step 1: 实现 TeamDomainService**

- [ ] **Step 2: Commit**

---

## Task 7: RBAC 服务层

**Files:**
- Create: `src/domain/service/rbac_service.rs`

- [ ] **Step 1: 实现 RbacService**

```rust
pub struct RbacService<MR, TPS> {
    membership_repo: Arc<MR>,
    tool_policy_service: Arc<TPS>,
}

impl<MR, TPS> RbacService<MR, TPS>
where
    MR: MembershipRepository + 'static,
    TPS: ToolPolicyService + 'static,
{
    /// 检查用户是否有权限
    pub async fn check_permission(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        permission: &Permission,
    ) -> Result<bool, RbacError> {
        // Find user's memberships
        // Check role permissions
        // Check tool policies
    }

    /// 分配角色
    pub async fn assign_role(
        &self,
        membership_id: &MembershipId,
        new_role: MembershipRole,
        assigner_id: UserId,
    ) -> Result<(), RbacError> {
        // Validate assigner has permission
        // Update membership role
        // Publish event
    }
}
```

- [ ] **Step 2: Commit**

---

## Task 8: 应用层命令处理

**Files:**
- Create: `src/application/commands/create_organization.rs`
- Create: `src/application/commands/create_team.rs`
- Create: `src/application/commands/assign_role.rs`
- Create: `src/application/commands/set_tool_policy.rs`

- [ ] **Step 1: 实现命令处理器**

- [ ] **Step 2: Commit**

```bash
git add src/application/commands/
git commit -m "feat(application): add organization and team commands"
```

---

## Task 9: 审计日志 PostgreSQL 实现

**Files:**
- Create: `src/infrastructure/audit/postgres.rs`

- [ ] **Step 1: 实现 PostgresAuditLogRepository**

```rust
pub struct PostgresAuditLogRepository {
    pool: PgPool,
}

#[async_trait]
impl AuditLogRepository for PostgresAuditLogRepository {
    type Error = sqlx::Error;

    async fn save(&self, log: &AuditLog) -> Result<(), Self::Error> {
        // INSERT logic
    }

    async fn find_by_tenant(
        &self,
        tenant_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<AuditLog>, Self::Error> {
        // Query with timestamp filtering
    }
}
```

- [ ] **Step 2: 更新 AuditLogger 使用 Repository**

- [ ] **Step 3: Commit**

```bash
git add src/infrastructure/audit/postgres.rs
git commit -m "feat(infra): implement PostgresAuditLogRepository"
```

---

## Task 10: 集成测试

**Files:**
- Create: `tests/integration/rbac_test.rs`
- Create: `tests/integration/multi_tenant_test.rs`

- [ ] **Step 1: RBAC 测试**

```rust
#[tokio::test]
async fn test_rbac_permission_check() {
    // Create tenant, org, team
    // Create memberships with different roles
    // Test permission checks for each role
}
```

- [ ] **Step 2: 多租户隔离测试**

```rust
#[tokio::test]
async fn test_multi_tenant_isolation() {
    // Create two tenants
    // Verify data isolation
    // Verify cross-tenant access is blocked
}
```

- [ ] **Step 3: Commit**

```bash
git add tests/integration/
git commit -m "test(integration): add RBAC and multi-tenant tests"
```

---

## Phase 3 完成检查清单

- [ ] Organization 领域模型完成
- [ ] Team 领域模型完成
- [ ] OrganizationRepository PostgreSQL 实现完成
- [ ] TeamRepository PostgreSQL 实现完成
- [ ] OrganizationDomainService 完成
- [ ] TeamDomainService 完成
- [ ] RbacService 完成
- [ ] 应用层命令完成
- [ ] 审计日志 PostgreSQL 实现完成
- [ ] 所有集成测试通过
- [ ] 代码格式化通过
- [ ] Clippy 检查通过

---

## Phase 3 完成后的下一步

Phase 3 完成后，继续进行 Phase 4：Gateway 与前端集成

Phase 4 将实现：
- WebSocket Gateway 完整消息处理
- 前端 UI（可选 React 或简单 HTML）
- 完整的 E2E 测试
- 性能优化和文档
