# Gateway & SaaS 架构重构 - Phase 2: 领域服务实现 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现领域层 Repository 的 PostgreSQL 持久化，完成成员管理用例，实现工具策略检查，与 Gateway 集成

**Architecture:** 基于 Phase 1 定义领域模型和 Port trait，实现 PostgreSQL 存储适配器，完成应用层命令处理

**Tech Stack:** Rust, PostgreSQL, SQLx, Kafka, JWT, tokio, axum

**Spec Reference:** `docs/superpowers/specs/2026-03-28-gateway-saas-architecture-design.md`

**Phase 1 Completion:**
- ✅ Cargo 依赖配置
- ✅ 数据库迁移配置
- ✅ 领域层基础类型定义（Tenant 聚合、Member 聚合）
- ✅ PostgreSQL 连接封装
- ✅ Kafka 事件总线
- ✅ JWT 认证服务
- ✅ CQRS 命令/查询框架
- ✅ 审计日志记录器

---

## File Structure

### New Files to Create

**Domain Layer Repositories (实现 Phase 1 的 trait):**
- `src/domain/tenant/repository/postgres.rs`
- `src/domain/member/repository/postgres.rs`

**Application Layer Commands (完整实现):**
- `src/application/commands/create_tenant.rs`
- `src/application/commands/invite_member.rs`
- `src/application/commands/accept_invite.rs`
- `src/application/commands/suspend_member.rs`
- `src/application/commands/change_role.rs`
- `src/application/commands/set_tool_policy.rs`

**Application Layer Queries:**
- `src/application/queries/get_tenant.rs`
- `src/application/queries/list_members.rs`
- `src/application/queries/get_tool_policy.rs`

**Domain Services:**
- `src/domain/service/tenant_service.rs`
- `src/domain/service/member_service.rs`
- `src/domain/service/tool_policy_service.rs`

**Gateway Integration:**
- `src/gateway/auth.rs` - JWT 认证集成
- `src/gateway/session_manager.rs` - 会话管理
- `src/gateway/handler.rs` - WebSocket 消息处理

**Tests:**
- `tests/domain/tenant_repository_test.rs`
- `tests/domain/member_repository_test.rs`
- `tests/application/command_test.rs`
- `tests/application/query_test.rs`
- `tests/gateway/auth_test.rs`

---

## Prerequisites

确保 Phase 1 已完成：
- 数据库迁移已应用
- 领域模型已定义
- PostgreSQL 连接可用
- 事件总线可用

---

## Task 1: Tenant Repository PostgreSQL 实现

**Files:**
- Create: `src/domain/tenant/repository/postgres.rs`
- Modify: `src/domain/tenant/repository.rs`

- [ ] **Step 1: 实现 TenantRepository trait**

```rust
use super::{TenantRepository, TenantError};
use crate::domain::tenant::{Tenant, TenantId};
use crate::infrastructure::persistence::postgres::PostgresConnection;
use sqlx::PgPool;
use async_trait::async_trait;

pub struct PostgresTenantRepository {
    pool: PgPool,
}

impl PostgresTenantRepository {
    pub fn new(conn: &PostgresConnection) -> Self {
        Self {
            pool: conn.pool().clone(),
        }
    }
}

#[async_trait]
impl TenantRepository for PostgresTenantRepository {
    type Error = TenantError;

    async fn save(&self, tenant: &Tenant) -> Result<(), Self::Error> {
        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                name = $2, slug = $3, status = $4, updated_at = $6
            "#,
        )
        .bind(tenant.id().as_str())
        .bind(tenant.name().as_str())
        .bind(tenant.slug().as_str())
        .bind(tenant.status().to_string())
        .bind(tenant.created_at())
        .bind(tenant.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                TenantError::AlreadyExists
            }
            _ => TenantError::DatabaseError(e.to_string()),
        })?;

        Ok(())
    }

    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, Self::Error> {
        let tenant = sqlx::query_as::<_, TenantRow>(
            "SELECT * FROM tenants WHERE id = $1",
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        Ok(tenant.map(|row| row.into_tenant()).transpose()?)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, Self::Error> {
        // 类似实现
    }

    async fn delete(&self, id: &TenantId) -> Result<(), Self::Error> {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

- [ ] **Step 2: 定义 SQLx Row 映射**

```rust
#[derive(sqlx::FromRow)]
struct TenantRow {
    id: String,
    name: String,
    slug: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TenantRow {
    fn into_tenant(self) -> Result<Tenant, TenantError> {
        // 转换为 Tenant 聚合根
    }
}
```

- [ ] **Step 3: 编写集成测试**

```rust
#[tokio::test]
async fn test_save_and_find_tenant() {
    // 创建测试数据库连接
    // 保存 Tenant
    // 查找并验证
}
```

- [ ] **Step 4: Commit**

```bash
git add src/domain/tenant/repository/postgres.rs tests/
git commit -m "feat(domain): implement PostgresTenantRepository"
```

---

## Task 2: Member Repository PostgreSQL 实现

**Files:**
- Create: `src/domain/member/repository/postgres.rs`
- Modify: `src/domain/member/repository.rs`

- [ ] **Step 1: 实现 MembershipRepository trait**

```rust
use super::{MembershipRepository, MemberDomainError, MembershipFilter};
use crate::domain::member::Membership;
use crate::infrastructure::persistence::postgres::PostgresConnection;
use sqlx::PgPool;
use async_trait::async_trait;

pub struct PostgresMembershipRepository {
    pool: PgPool,
}

impl PostgresMembershipRepository {
    pub fn new(conn: &PostgresConnection) -> Self {
        Self { pool: conn.pool().clone() }
    }
}

#[async_trait]
impl MembershipRepository for PostgresMembershipRepository {
    type Error = MemberDomainError;

    async fn save(&self, membership: &Membership) -> Result<(), Self::Error> {
        // UPSERT 逻辑
    }

    async fn find_by_id(&self, id: &MembershipId) -> Result<Option<Membership>, Self::Error> {
        // 查询逻辑
    }

    async fn find_by_user(&self, user_id: &UserId) -> Result<Vec<Membership>, Self::Error> {
        // 查询用户的所有成员关系
    }

    async fn find_by_organization(&self, org_id: &OrganizationId) -> Result<Vec<Membership>, Self::Error> {
        // 查询组织的所有成员
    }

    async fn find_by_filter(&self, filter: &MembershipFilter) -> Result<Vec<Membership>, Self::Error> {
        // 动态查询构建
    }

    async fn delete(&self, id: &MembershipId) -> Result<(), Self::Error> {
        // 删除逻辑
    }
}
```

- [ ] **Step 2: 编写集成测试**

- [ ] **Step 3: Commit**

```bash
git add src/domain/member/repository/postgres.rs tests/
git commit -m "feat(domain): implement PostgresMembershipRepository"
```

---

## Task 3: 租户服务层

**Files:**
- Create: `src/domain/service/tenant_service.rs`

- [ ] **Step 1: 实现 TenantDomainService**

```rust
pub struct TenantDomainService<TR: TenantRepository> {
    tenant_repo: Arc<TR>,
    event_publisher: Arc<dyn EventPublisher<Error = EventBusError>>,
}

impl<TR: TenantRepository> TenantDomainService<TR> {
    /// 创建租户
    pub async fn create_tenant(
        &self,
        name: TenantName,
        slug: TenantSlug,
    ) -> Result<Tenant, TenantError> {
        // 检查 slug 是否已存在
        if self.tenant_repo.find_by_slug(slug.as_str()).await?.is_some() {
            return Err(TenantError::InvalidSlug("Slug already exists".into()));
        }

        // 创建租户
        let tenant = Tenant::create(name, slug)?;

        // 保存
        self.tenant_repo.save(&tenant).await?;

        // 发布领域事件
        for event in tenant.events() {
            self.event_publisher.publish(&event).await?;
        }

        Ok(tenant)
    }

    /// 暂停租户
    pub async fn suspend_tenant(&self, id: &TenantId, reason: &str) -> Result<(), TenantError> {
        // 实现
    }

    /// 恢复租户
    pub async fn restore_tenant(&self, id: &TenantId) -> Result<(), TenantError> {
        // 实现
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/domain/service/
git commit -m "feat(domain): add TenantDomainService"
```

---

## Task 4: 成员服务层

**Files:**
- Create: `src/domain/service/member_service.rs`

- [ ] **Step 1: 实现 MemberDomainService**

```rust
pub struct MemberDomainService<MR: MembershipRepository> {
    membership_repo: Arc<MR>,
    event_publisher: Arc<dyn EventPublisher<Error = EventBusError>>,
}

impl<MR: MembershipRepository> MemberDomainService<MR> {
    /// 邀请成员
    pub async fn invite_member(
        &self,
        tenant_id: TenantId,
        organization_id: OrganizationId,
        team_id: Option<TeamId>,
        email: UserEmail,
        role: MembershipRole,
        inviter_id: UserId,
    ) -> Result<Membership, MemberDomainError> {
        // 检查是否已存在成员关系
        // 创建邀请
        let membership = Membership::invite(...)?;

        // 保存
        self.membership_repo.save(&membership).await?;

        // 发布事件
        for event in membership.events() {
            self.event_publisher.publish(&event).await?;
        }

        Ok(membership)
    }

    /// 接受邀请
    pub async fn accept_invite(
        &self,
        membership_id: &MembershipId,
        user_id: UserId,
    ) -> Result<(), MemberDomainError> {
        // 实现
    }

    /// 暂停成员
    pub async fn suspend_member(
        &self,
        membership_id: &MembershipId,
        reason: &str,
    ) -> Result<(), MemberDomainError> {
        // 实现
    }

    /// 变更角色
    pub async fn change_role(
        &self,
        membership_id: &MembershipId,
        new_role: MembershipRole,
    ) -> Result<(), MemberDomainError> {
        // 实现
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/domain/service/member_service.rs
git commit -m "feat(domain): add MemberDomainService"
```

---

## Task 5: 工具策略服务

**Files:**
- Create: `src/domain/service/tool_policy_service.rs`

- [ ] **Step 1: 实现 ToolPolicyService**

```rust
pub struct ToolPolicyService {
    // 可能需要 Repository 来持久化策略
}

impl ToolPolicyService {
    /// 检查是否可以执行工具
    pub fn can_execute_tool(
        &self,
        role: &MembershipRole,
        tool_id: &ToolId,
        risk_level: &ToolRiskLevel,
        policy: &ToolPolicy,
    ) -> Result<(), PermissionError> {
        // 实现权限检查逻辑
    }

    /// 设置工具策略
    pub fn set_tool_policy(
        &self,
        tenant_id: &TenantId,
        organization_id: Option<&OrganizationId>,
        team_id: Option<&TeamId>,
        allowed_tools: Vec<ToolId>,
        denied_tools: Vec<ToolId>,
    ) -> Result<ToolPolicy, PolicyError> {
        // 实现
    }
}
```

- [ ] **Step 2: Commit**

---

## Task 6: 应用层命令处理

**Files:**
- Create: `src/application/commands/create_tenant.rs`
- Create: `src/application/commands/invite_member.rs` (完整实现)
- Create: `src/application/commands/accept_invite.rs`
- Create: `src/application/commands/suspend_member.rs`

- [ ] **Step 1: 实现 CreateTenantCommand**

```rust
#[derive(Debug, Clone)]
pub struct CreateTenantCommand {
    pub name: String,
    pub slug: String,
    pub creator_id: UserId,
}

impl Command for CreateTenantCommand {
    type Response = Tenant;
}

pub struct CreateTenantHandler<TR, EP> {
    tenant_service: Arc<TenantDomainService<TR>>,
    _event_publisher: Arc<EP>,
}

#[async_trait]
impl<TR, EP> CommandHandler<CreateTenantCommand> for CreateTenantHandler<TR, EP>
where
    TR: TenantRepository + 'static,
    EP: EventPublisher + 'static,
{
    type Error = Box<dyn Error + Send + Sync>;

    async fn handle(&self, command: CreateTenantCommand) -> Result<Tenant, Self::Error> {
        let name = TenantName::new(command.name)?;
        let slug = TenantSlug::new(command.slug)?;

        let tenant = self.tenant_service.create_tenant(name, slug).await?;

        // 记录审计日志
        // self.audit_logger.log(...).await?;

        Ok(tenant)
    }
}
```

- [ ] **Step 2: 实现其他命令**

- [ ] **Step 3: 编写集成测试**

- [ ] **Step 4: Commit**

```bash
git add src/application/commands/ tests/
git commit -m "feat(application): implement tenant and member commands"
```

---

## Task 7: 应用层查询处理

**Files:**
- Create: `src/application/queries/get_tenant.rs`
- Create: `src/application/queries/list_members.rs` (完整实现)

- [ ] **Step 1: 实现 GetTenantQuery**

```rust
#[derive(Debug, Clone)]
pub struct GetTenantQuery {
    pub tenant_id: TenantId,
}

impl Query for GetTenantQuery {
    type Response = Option<Tenant>;
}

pub struct GetTenantHandler<TR> {
    tenant_repo: Arc<TR>,
}

#[async_trait]
impl<TR> QueryHandler<GetTenantQuery> for GetTenantHandler<TR>
where
    TR: TenantRepository + 'static,
{
    type Error = Box<dyn Error + Send + Sync>;

    async fn handle(&self, query: GetTenantQuery) -> Result<Option<Tenant>, Self::Error> {
        Ok(self.tenant_repo.find_by_id(&query.tenant_id).await?)
    }
}
```

- [ ] **Step 2: 实现 ListMembersQuery**

- [ ] **Step 3: Commit**

---

## Task 8: Gateway 集成

**Files:**
- Create: `src/gateway/auth.rs`
- Create: `src/gateway/session_manager.rs`
- Create: `src/gateway/handler.rs`

- [ ] **Step 1: 实现 JWT 认证集成**

```rust
use crate::infrastructure::auth::{BeeClaims, JwtService};

pub struct GatewayAuth {
    jwt_service: Arc<JwtService>,
}

impl GatewayAuth {
    pub fn new(jwt_service: Arc<JwtService>) -> Self {
        Self { jwt_service }
    }

    pub fn authenticate(&self, token: &str) -> Result<BeeClaims, AuthError> {
        self.jwt_service.validate_token(token)
    }

    pub fn generate_token(&self, claims: BeeClaims) -> Result<String, AuthError> {
        self.jwt_service.generate_token(&claims)
    }
}
```

- [ ] **Step 2: 实现会话管理**

```rust
pub struct SessionManager {
    sessions: DashMap<String, SessionInfo>,
}

impl SessionManager {
    pub fn create_session(&self, user_id: &str, tenant_id: &str) -> String {
        // 生成 session_id
    }

    pub fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.get(session_id).map(|s| s.clone())
    }
}
```

- [ ] **Step 3: 实现 WebSocket 消息处理**

```rust
pub struct WebSocketHandler {
    auth: Arc<GatewayAuth>,
    session_manager: Arc<SessionManager>,
    command_bus: Arc<CommandBus>,
    query_bus: Arc<QueryBus>,
}

impl WebSocketHandler {
    pub async fn handle_message(&self, msg: GatewayMessage) -> Result<GatewayMessage, HandlerError> {
        match msg.message {
            MessageType::Auth { token, client_info } => {
                // 认证处理
            }
            MessageType::UserMessage { content, .. } => {
                // 调用 command bus 处理
            }
            // ...
        }
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add src/gateway/ tests/gateway/
git commit -m "feat(gateway): integrate JWT auth and session management"
```

---

## Task 9: 集成测试

**Files:**
- Create: `tests/integration/tenant_lifecycle_test.rs`
- Create: `tests/integration/member_lifecycle_test.rs`
- Create: `tests/integration/gateway_test.rs`

- [ ] **Step 1: 租户生命周期测试**

```rust
#[tokio::test]
async fn test_tenant_lifecycle() {
    // 1. 创建租户
    // 2. 查询租户
    // 3. 暂停租户
    // 4. 恢复租户
    // 5. 删除租户
}
```

- [ ] **Step 2: 成员生命周期测试**

- [ ] **Step 3: Gateway 集成测试**

- [ ] **Step 4: Commit**

```bash
git add tests/integration/
git commit -m "test(integration): add end-to-end lifecycle tests"
```

---

## Phase 2 完成检查清单

- [ ] TenantRepository PostgreSQL 实现完成
- [ ] MembershipRepository PostgreSQL 实现完成
- [ ] TenantDomainService 完成
- [ ] MemberDomainService 完成
- [ ] ToolPolicyService 完成
- [ ] 应用层命令（CreateTenant, InviteMember, etc.）完成
- [ ] 应用层查询（GetTenant, ListMembers）完成
- [ ] Gateway JWT 认证集成完成
- [ ] Gateway 会话管理完成
- [ ] 所有集成测试通过
- [ ] 代码格式化通过
- [ ] Clippy 检查通过

---

## Phase 2 完成后的下一步

Phase 2 完成后，继续进行 Phase 3：多租户与权限

Phase 3 将实现：
- 完整的多租户层级（租户 → 组织 → 团队）
- RBAC 权限系统完整实现
- 工具策略的持久化和执行
- 审计日志的完整记录
