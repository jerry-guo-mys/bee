//! PostgreSQL 实现的 MembershipRepository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::common::{MembershipRole, MembershipStatus};
use crate::domain::member::entity::{MemberDomainError, Membership};
use crate::domain::member::value_object::{ToolId, ToolPolicy, ToolRiskLevel, UserEmail};
use crate::domain::tenant::value_object::{MembershipId, OrganizationId, TeamId, TenantId, UserId};
use crate::infrastructure::persistence::postgres::PostgresConnection;

use super::{MembershipFilter, MembershipRepository};

/// PostgreSQL 成员仓库实现
pub struct PostgresMembershipRepository {
    pool: PgPool,
}

impl PostgresMembershipRepository {
    /// 创建新的 PostgreSQL 成员仓库
    pub fn new(conn: &PostgresConnection) -> Self {
        Self {
            pool: conn.pool().clone(),
        }
    }
}

/// 数据库行结构
#[derive(FromRow)]
struct MembershipRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    organization_id: uuid::Uuid,
    team_id: Option<uuid::Uuid>,
    user_id: Option<uuid::Uuid>,
    email: String,
    role: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// 工具策略行结构
#[derive(FromRow)]
struct ToolPolicyRow {
    #[allow(dead_code)]
    membership_id: uuid::Uuid,
    tool_id: String,
    risk_level: String,
    is_allowed: bool,
    note: Option<String>,
}

impl MembershipRow {
    /// 将数据库行转换为 Membership 实体
    fn into_membership(self) -> Result<Membership, MemberDomainError> {
        let id = MembershipId::new(self.id.to_string());
        let tenant_id = TenantId::new(self.tenant_id.to_string());
        let organization_id = OrganizationId::new(self.organization_id.to_string());
        let team_id = self.team_id.map(|id| TeamId::new(id.to_string()));
        let user_id = self.user_id.map(|id| UserId::new(id.to_string()));
        let email = UserEmail::new(self.email)
            .map_err(|e| MemberDomainError::InvalidStatus(e.to_string()))?;
        let role =
            parse_membership_role(&self.role).map_err(|e| MemberDomainError::InvalidRole(e))?;
        let status = parse_membership_status(&self.status)
            .map_err(|e| MemberDomainError::InvalidStatus(e))?;

        Ok(Membership::load(
            id,
            tenant_id,
            organization_id,
            team_id,
            user_id,
            email,
            role,
            status,
            self.created_at,
            self.updated_at,
            Vec::new(), // tool_policies 从单独的表加载
        ))
    }
}

/// 解析 MembershipRole
fn parse_membership_role(role_str: &str) -> Result<MembershipRole, String> {
    match role_str.to_lowercase().as_str() {
        "platform_admin" => Ok(MembershipRole::PlatformAdmin),
        "org_admin" => Ok(MembershipRole::OrgAdmin),
        "team_admin" => Ok(MembershipRole::TeamAdmin),
        "member" => Ok(MembershipRole::Member),
        "viewer" => Ok(MembershipRole::Viewer),
        _ => Err(format!("Invalid membership role: {}", role_str)),
    }
}

/// 解析 MembershipStatus
fn parse_membership_status(status_str: &str) -> Result<MembershipStatus, String> {
    match status_str.to_lowercase().as_str() {
        "pending" => Ok(MembershipStatus::Pending),
        "active" => Ok(MembershipStatus::Active),
        "suspended" => Ok(MembershipStatus::Suspended),
        "removed" => Ok(MembershipStatus::Removed),
        _ => Err(format!("Invalid membership status: {}", status_str)),
    }
}

/// 将 MembershipRole 转换为字符串
fn membership_role_to_string(role: &MembershipRole) -> &'static str {
    match role {
        MembershipRole::PlatformAdmin => "platform_admin",
        MembershipRole::OrgAdmin => "org_admin",
        MembershipRole::TeamAdmin => "team_admin",
        MembershipRole::Member => "member",
        MembershipRole::Viewer => "viewer",
    }
}

/// 将 MembershipStatus 转换为字符串
fn membership_status_to_string(status: &MembershipStatus) -> &'static str {
    match status {
        MembershipStatus::Pending => "pending",
        MembershipStatus::Active => "active",
        MembershipStatus::Suspended => "suspended",
        MembershipStatus::Removed => "removed",
    }
}

/// 加载成员的工具策略列表
async fn load_tool_policies(
    pool: &PgPool,
    membership_id: &MembershipId,
) -> Result<Vec<ToolPolicy>, MemberDomainError> {
    let rows = sqlx::query_as::<_, ToolPolicyRow>(
        "SELECT membership_id, tool_id, risk_level, is_allowed, note
         FROM membership_tool_policies
         WHERE membership_id = $1",
    )
    .bind(
        membership_id
            .as_str()
            .parse::<uuid::Uuid>()
            .map_err(|e| MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e)))?,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

    let policies = rows
        .into_iter()
        .map(|row| {
            let risk_level = ToolRiskLevel::from_str(&row.risk_level).unwrap_or(ToolRiskLevel::Low);
            let mut policy =
                ToolPolicy::new(ToolId::from_str(&row.tool_id), risk_level, row.is_allowed);
            if let Some(note) = row.note {
                policy = policy.with_note(note);
            }
            policy
        })
        .collect();

    Ok(policies)
}

/// 批量加载多个成员的工具策略列表
async fn load_tool_policies_batch(
    pool: &PgPool,
    membership_ids: &[&str],
) -> Result<std::collections::HashMap<String, Vec<ToolPolicy>>, MemberDomainError> {
    if membership_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let rows = sqlx::query_as::<_, ToolPolicyRow>(
        "SELECT membership_id, tool_id, risk_level, is_allowed, note
         FROM membership_tool_policies
         WHERE membership_id = ANY($1)",
    )
    .bind(membership_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

    // Group policies by membership_id
    let mut policies_by_membership: std::collections::HashMap<String, Vec<ToolPolicy>> =
        std::collections::HashMap::new();
    for row in rows {
        let membership_id = row.membership_id.to_string();
        let risk_level = ToolRiskLevel::from_str(&row.risk_level).unwrap_or(ToolRiskLevel::Low);
        let mut policy =
            ToolPolicy::new(ToolId::from_str(&row.tool_id), risk_level, row.is_allowed);
        if let Some(note) = row.note {
            policy = policy.with_note(note);
        }
        policies_by_membership
            .entry(membership_id)
            .or_default()
            .push(policy);
    }

    Ok(policies_by_membership)
}

#[async_trait]
impl MembershipRepository for PostgresMembershipRepository {
    type Error = MemberDomainError;

    async fn save(&self, membership: &Membership) -> Result<(), Self::Error> {
        // UPSERT membership
        let membership_uuid = membership
            .id()
            .as_str()
            .parse::<uuid::Uuid>()
            .map_err(|e| MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO memberships (
                id, tenant_id, organization_id, team_id, user_id, email, role, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                tenant_id = $2,
                organization_id = $3,
                team_id = $4,
                user_id = $5,
                email = $6,
                role = $7,
                status = $8,
                updated_at = $10
            "#,
        )
        .bind(membership_uuid)
        .bind(membership.tenant_id().as_str().parse::<uuid::Uuid>().map_err(|e| {
            MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
        })?)
        .bind(membership.organization_id().as_str().parse::<uuid::Uuid>().map_err(|e| {
            MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
        })?)
        .bind(membership.team_id().as_ref().map(|id| id.as_str().parse::<uuid::Uuid>()).transpose().map_err(|e| {
            MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
        })?)
        .bind(membership.user_id().as_ref().map(|id| id.as_str().parse::<uuid::Uuid>()).transpose().map_err(|e| {
            MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
        })?)
        .bind(membership.email().as_str())
        .bind(membership_role_to_string(membership.role()))
        .bind(membership_status_to_string(membership.status()))
        .bind(membership.created_at())
        .bind(membership.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                MemberDomainError::AlreadyExists(membership.id().to_string())
            }
            _ => MemberDomainError::DatabaseError(e.to_string()),
        })?;

        // 保存工具策略
        save_tool_policies(&self.pool, membership).await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &MembershipId) -> Result<Option<Membership>, Self::Error> {
        let row =
            sqlx::query_as::<_, MembershipRow>("SELECT * FROM memberships WHERE id = $1")
                .bind(id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => {
                let mut membership = row.into_membership()?;
                // 加载工具策略
                let policies = load_tool_policies(&self.pool, id).await?;
                // 使用反射或重新创建来设置 tool_policies
                // 由于 tool_policies 是私有字段，我们通过重新创建来设置
                membership = Membership::load(
                    membership.id().clone(),
                    membership.tenant_id().clone(),
                    membership.organization_id().clone(),
                    membership.team_id().cloned(),
                    membership.user_id().cloned(),
                    membership.email().clone(),
                    membership.role().clone(),
                    membership.status().clone(),
                    *membership.created_at(),
                    *membership.updated_at(),
                    policies,
                );
                Ok(Some(membership))
            }
            None => Ok(None),
        }
    }

    async fn find_by_user(&self, user_id: &UserId) -> Result<Vec<Membership>, Self::Error> {
        let rows =
            sqlx::query_as::<_, MembershipRow>("SELECT * FROM memberships WHERE user_id = $1")
                .bind(user_id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Convert rows to memberships first
        let mut memberships: Vec<Membership> = rows
            .into_iter()
            .map(|row| row.into_membership())
            .collect::<Result<Vec<_>, _>>()?;

        // Collect membership IDs for batch loading
        let membership_ids: Vec<&str> = memberships.iter().map(|m| m.id().as_str()).collect();

        // Load all tool policies in ONE query
        let mut policies_by_membership =
            load_tool_policies_batch(&self.pool, &membership_ids).await?;

        // Assign policies to memberships
        for membership in &mut memberships {
            let policies = policies_by_membership
                .remove(membership.id().as_str())
                .unwrap_or_default();
            *membership = Membership::load(
                membership.id().clone(),
                membership.tenant_id().clone(),
                membership.organization_id().clone(),
                membership.team_id().cloned(),
                membership.user_id().cloned(),
                membership.email().clone(),
                membership.role().clone(),
                membership.status().clone(),
                *membership.created_at(),
                *membership.updated_at(),
                policies,
            );
        }

        Ok(memberships)
    }

    async fn find_by_organization(
        &self,
        org_id: &OrganizationId,
    ) -> Result<Vec<Membership>, Self::Error> {
        let rows = sqlx::query_as::<_, MembershipRow>(
            "SELECT * FROM memberships WHERE organization_id = $1",
        )
        .bind(
            org_id
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Convert rows to memberships first
        let mut memberships: Vec<Membership> = rows
            .into_iter()
            .map(|row| row.into_membership())
            .collect::<Result<Vec<_>, _>>()?;

        // Collect membership IDs for batch loading
        let membership_ids: Vec<&str> = memberships.iter().map(|m| m.id().as_str()).collect();

        // Load all tool policies in ONE query
        let mut policies_by_membership =
            load_tool_policies_batch(&self.pool, &membership_ids).await?;

        // Assign policies to memberships
        for membership in &mut memberships {
            let policies = policies_by_membership
                .remove(membership.id().as_str())
                .unwrap_or_default();
            *membership = Membership::load(
                membership.id().clone(),
                membership.tenant_id().clone(),
                membership.organization_id().clone(),
                membership.team_id().cloned(),
                membership.user_id().cloned(),
                membership.email().clone(),
                membership.role().clone(),
                membership.status().clone(),
                *membership.created_at(),
                *membership.updated_at(),
                policies,
            );
        }

        Ok(memberships)
    }

    async fn find_by_team(&self, team_id: &TeamId) -> Result<Vec<Membership>, Self::Error> {
        let rows =
            sqlx::query_as::<_, MembershipRow>("SELECT * FROM memberships WHERE team_id = $1")
                .bind(team_id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Convert rows to memberships first
        let mut memberships: Vec<Membership> = rows
            .into_iter()
            .map(|row| row.into_membership())
            .collect::<Result<Vec<_>, _>>()?;

        // Collect membership IDs for batch loading
        let membership_ids: Vec<&str> = memberships.iter().map(|m| m.id().as_str()).collect();

        // Load all tool policies in ONE query
        let mut policies_by_membership =
            load_tool_policies_batch(&self.pool, &membership_ids).await?;

        // Assign policies to memberships
        for membership in &mut memberships {
            let policies = policies_by_membership
                .remove(membership.id().as_str())
                .unwrap_or_default();
            *membership = Membership::load(
                membership.id().clone(),
                membership.tenant_id().clone(),
                membership.organization_id().clone(),
                membership.team_id().cloned(),
                membership.user_id().cloned(),
                membership.email().clone(),
                membership.role().clone(),
                membership.status().clone(),
                *membership.created_at(),
                *membership.updated_at(),
                policies,
            );
        }

        Ok(memberships)
    }

    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Membership>, Self::Error> {
        let rows =
            sqlx::query_as::<_, MembershipRow>("SELECT * FROM memberships WHERE tenant_id = $1")
                .bind(tenant_id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Convert rows to memberships first
        let mut memberships: Vec<Membership> = rows
            .into_iter()
            .map(|row| row.into_membership())
            .collect::<Result<Vec<_>, _>>()?;

        // Collect membership IDs for batch loading
        let membership_ids: Vec<&str> = memberships.iter().map(|m| m.id().as_str()).collect();

        // Load all tool policies in ONE query
        let mut policies_by_membership =
            load_tool_policies_batch(&self.pool, &membership_ids).await?;

        // Assign policies to memberships
        for membership in &mut memberships {
            let policies = policies_by_membership
                .remove(membership.id().as_str())
                .unwrap_or_default();
            *membership = Membership::load(
                membership.id().clone(),
                membership.tenant_id().clone(),
                membership.organization_id().clone(),
                membership.team_id().cloned(),
                membership.user_id().cloned(),
                membership.email().clone(),
                membership.role().clone(),
                membership.status().clone(),
                *membership.created_at(),
                *membership.updated_at(),
                policies,
            );
        }

        Ok(memberships)
    }

    async fn find_by_filter(
        &self,
        filter: &MembershipFilter,
    ) -> Result<Vec<Membership>, Self::Error> {
        use sqlx::QueryBuilder;

        let mut query = QueryBuilder::new("SELECT * FROM memberships WHERE 1=1");

        if let Some(ref tenant_id) = filter.tenant_id {
            query.push(" AND tenant_id = ");
            query.push_bind(
                tenant_id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?,
            );
        }

        if let Some(ref organization_id) = filter.organization_id {
            query.push(" AND organization_id = ");
            query.push_bind(
                organization_id
                    .as_str()
                    .parse::<uuid::Uuid>()
                    .map_err(|e| {
                        MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                    })?,
            );
        }

        if let Some(ref team_id) = filter.team_id {
            query.push(" AND team_id = ");
            query.push_bind(
                team_id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?,
            );
        }

        if let Some(ref user_id) = filter.user_id {
            query.push(" AND user_id = ");
            query.push_bind(
                user_id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?,
            );
        }

        if let Some(ref role) = filter.role {
            query.push(" AND role = ");
            query.push_bind(membership_role_to_string(role));
        }

        if let Some(ref status) = filter.status {
            query.push(" AND status = ");
            query.push_bind(membership_status_to_string(status));
        }

        let rows = query
            .build_query_as::<MembershipRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Convert rows to memberships first
        let mut memberships: Vec<Membership> = rows
            .into_iter()
            .map(|row| row.into_membership())
            .collect::<Result<Vec<_>, _>>()?;

        // Collect membership IDs for batch loading
        let membership_ids: Vec<&str> = memberships.iter().map(|m| m.id().as_str()).collect();

        // Load all tool policies in ONE query
        let mut policies_by_membership =
            load_tool_policies_batch(&self.pool, &membership_ids).await?;

        // Assign policies to memberships
        for membership in &mut memberships {
            let policies = policies_by_membership
                .remove(membership.id().as_str())
                .unwrap_or_default();
            *membership = Membership::load(
                membership.id().clone(),
                membership.tenant_id().clone(),
                membership.organization_id().clone(),
                membership.team_id().cloned(),
                membership.user_id().cloned(),
                membership.email().clone(),
                membership.role().clone(),
                membership.status().clone(),
                *membership.created_at(),
                *membership.updated_at(),
                policies,
            );
        }

        Ok(memberships)
    }

    async fn delete(&self, id: &MembershipId) -> Result<(), Self::Error> {
        // 先删除关联的工具策略
        sqlx::query("DELETE FROM membership_tool_policies WHERE membership_id = $1")
            .bind(
                id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

        // 删除成员
        let result =
            sqlx::query("DELETE FROM memberships WHERE id = $1")
                .bind(id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e))
                })?)
                .execute(&self.pool)
                .await
                .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(MemberDomainError::NotFound(id.to_string()));
        }

        Ok(())
    }
}

/// 保存成员的工具策略
async fn save_tool_policies(
    pool: &PgPool,
    membership: &Membership,
) -> Result<(), MemberDomainError> {
    let membership_uuid = membership
        .id()
        .as_str()
        .parse::<uuid::Uuid>()
        .map_err(|e| MemberDomainError::DatabaseError(format!("Invalid UUID: {}", e)))?;

    // 先删除现有的策略
    sqlx::query("DELETE FROM membership_tool_policies WHERE membership_id = $1")
        .bind(membership_uuid)
        .execute(pool)
        .await
        .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;

    // 插入新策略
    for policy in membership.tool_policies() {
        sqlx::query(
            r#"
            INSERT INTO membership_tool_policies (membership_id, tool_id, risk_level, is_allowed, note)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(membership_uuid)
        .bind(policy.tool_id().as_str())
        .bind(match policy.risk_level() {
            ToolRiskLevel::Low => "low",
            ToolRiskLevel::Medium => "medium",
            ToolRiskLevel::High => "high",
            ToolRiskLevel::Critical => "critical",
        })
        .bind(policy.is_allowed())
        .bind(policy.note())
        .execute(pool)
        .await
        .map_err(|e| MemberDomainError::DatabaseError(e.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::member::repository::MembershipRepository;

    /// 获取测试数据库连接
    /// 需要设置 DATABASE_URL 环境变量
    async fn get_test_connection() -> Option<PostgresConnection> {
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                eprintln!("DATABASE_URL not set, skipping test");
                return None;
            }
        };

        PostgresConnection::new(&database_url).await.ok()
    }

    #[tokio::test]
    async fn test_save_and_find_membership() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresMembershipRepository::new(&conn);

        // 创建测试数据
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        // 创建 Membership
        let mut membership = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::Member,
        )
        .unwrap();

        // 接受邀请
        let user_id = UserId::generate();
        membership.accept_invite(user_id.clone()).unwrap();

        // 添加工具策略
        let policy = ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::Medium, true);
        membership.add_tool_policy(policy).unwrap();

        // 保存
        repo.save(&membership).await.unwrap();

        // 根据 ID 查找
        let found = repo.find_by_id(membership.id()).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id(), membership.id());
        assert_eq!(found.status(), &MembershipStatus::Active);
        assert_eq!(found.user_id(), Some(&user_id));
        assert_eq!(found.tool_policies().len(), 1);
        assert_eq!(found.tool_policies()[0].tool_id().as_str(), "shell");

        // 清理
        repo.delete(membership.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresMembershipRepository::new(&conn);

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let user_id = UserId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        // 创建两个 Membership
        let mut membership1 = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            Some(user_id.clone()),
            email.clone(),
            MembershipRole::Member,
        )
        .unwrap();
        membership1.accept_invite(user_id.clone()).unwrap();

        let mut membership2 = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            Some(user_id.clone()),
            email.clone(),
            MembershipRole::OrgAdmin,
        )
        .unwrap();
        membership2.accept_invite(user_id.clone()).unwrap();

        // 保存
        repo.save(&membership1).await.unwrap();
        repo.save(&membership2).await.unwrap();

        // 根据用户查找
        let founds = repo.find_by_user(&user_id).await.unwrap();
        assert_eq!(founds.len(), 2);

        // 清理
        repo.delete(membership1.id()).await.unwrap();
        repo.delete(membership2.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_find_by_organization() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresMembershipRepository::new(&conn);

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        // 创建 Membership
        let mut membership = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::Member,
        )
        .unwrap();
        membership.accept_invite(UserId::generate()).unwrap();

        // 保存
        repo.save(&membership).await.unwrap();

        // 根据组织查找
        let founds = repo.find_by_organization(&org_id).await.unwrap();
        assert_eq!(founds.len(), 1);
        assert_eq!(founds[0].id(), membership.id());

        // 清理
        repo.delete(membership.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_membership() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresMembershipRepository::new(&conn);

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        // 创建 Membership
        let membership = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::Member,
        )
        .unwrap();

        // 保存
        repo.save(&membership).await.unwrap();

        // 删除
        repo.delete(membership.id()).await.unwrap();

        // 验证已删除
        let found = repo.find_by_id(membership.id()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_update_membership() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresMembershipRepository::new(&conn);

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        // 创建 Membership
        let mut membership = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::Member,
        )
        .unwrap();
        membership.accept_invite(UserId::generate()).unwrap();

        // 保存
        repo.save(&membership).await.unwrap();

        // 修改角色
        membership.change_role(MembershipRole::OrgAdmin).unwrap();
        repo.save(&membership).await.unwrap();

        // 验证更新
        let found = repo.find_by_id(membership.id()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().role(), &MembershipRole::OrgAdmin);

        // 清理
        repo.delete(membership.id()).await.unwrap();
    }
}
