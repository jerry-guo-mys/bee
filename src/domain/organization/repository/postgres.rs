//! PostgreSQL 实现的 OrganizationRepository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::organization::entity::Organization;
use crate::domain::organization::value_object::{
    OrganizationError, OrganizationId, OrganizationName, OrganizationSlug,
};
use crate::domain::tenant::TenantId;
use crate::infrastructure::persistence::postgres::PostgresConnection;

/// PostgreSQL 组织仓库实现
pub struct PostgresOrganizationRepository {
    pool: PgPool,
}

impl PostgresOrganizationRepository {
    /// 创建新的 PostgreSQL 组织仓库
    pub fn new(conn: &PostgresConnection) -> Self {
        Self {
            pool: conn.pool().clone(),
        }
    }
}

/// 数据库行结构
#[derive(FromRow)]
struct OrganizationRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    name: String,
    slug: String,
    industry: Option<String>,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl OrganizationRow {
    /// 将数据库行转换为 Organization 实体
    fn into_organization(self) -> Result<Organization, OrganizationError> {
        let id = OrganizationId::new(self.id.to_string());
        let tenant_id = TenantId::new(self.tenant_id.to_string());
        let name = OrganizationName::new(self.name)
            .map_err(|e| OrganizationError::InvalidName(e.to_string()))?;
        let slug = OrganizationSlug::new(self.slug)
            .map_err(|e| OrganizationError::InvalidSlug(e.to_string()))?;

        Ok(Organization::load(
            id,
            tenant_id,
            name,
            slug,
            self.industry,
            self.description,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[async_trait]
impl super::OrganizationRepository for PostgresOrganizationRepository {
    type Error = OrganizationError;

    async fn save(&self, org: &Organization) -> Result<(), Self::Error> {
        sqlx::query(
            r#"
            INSERT INTO organizations (id, tenant_id, name, slug, industry, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                name = $3, slug = $4, industry = $5, description = $6, updated_at = $8
            "#,
        )
        .bind(
            org.id()
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| OrganizationError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .bind(
            org.tenant_id()
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| OrganizationError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .bind(org.name().as_str())
        .bind(org.slug().as_str())
        .bind(org.industry())
        .bind(org.description())
        .bind(org.created_at())
        .bind(org.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                OrganizationError::AlreadyExists(org.id().to_string())
            }
            _ => OrganizationError::DatabaseError(e.to_string()),
        })?;

        Ok(())
    }

    async fn find_by_id(&self, id: &OrganizationId) -> Result<Option<Organization>, Self::Error> {
        let org =
            sqlx::query_as::<_, OrganizationRow>("SELECT * FROM organizations WHERE id = $1")
                .bind(id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    OrganizationError::DatabaseError(format!("Invalid UUID: {}", e))
                })?)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| OrganizationError::DatabaseError(e.to_string()))?;

        match org {
            Some(row) => row.into_organization().map(Some),
            None => Ok(None),
        }
    }

    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Organization>, Self::Error> {
        let orgs = sqlx::query_as::<_, OrganizationRow>(
            "SELECT * FROM organizations WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(
            tenant_id
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| OrganizationError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OrganizationError::DatabaseError(e.to_string()))?;

        orgs.into_iter()
            .map(|row| row.into_organization())
            .collect()
    }

    async fn find_by_slug(
        &self,
        tenant_id: &TenantId,
        slug: &str,
    ) -> Result<Option<Organization>, Self::Error> {
        let org = sqlx::query_as::<_, OrganizationRow>(
            "SELECT * FROM organizations WHERE tenant_id = $1 AND slug = $2",
        )
        .bind(
            tenant_id
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| OrganizationError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OrganizationError::DatabaseError(e.to_string()))?;

        match org {
            Some(row) => row.into_organization().map(Some),
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &OrganizationId) -> Result<(), Self::Error> {
        let result =
            sqlx::query("DELETE FROM organizations WHERE id = $1")
                .bind(id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                    OrganizationError::DatabaseError(format!("Invalid UUID: {}", e))
                })?)
                .execute(&self.pool)
                .await
                .map_err(|e| OrganizationError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(OrganizationError::NotFound(id.to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::organization::repository::OrganizationRepository;

    /// 获取测试数据库连接
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
    async fn test_save_and_find_organization() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresOrganizationRepository::new(&conn);

        let tenant_id = TenantId::generate();
        let org = Organization::create(
            tenant_id.clone(),
            "Test Organization".to_string(),
            "test-org".to_string(),
        )
        .unwrap();

        // 保存
        repo.save(&org).await.unwrap();

        // 根据 ID 查找
        let found = repo.find_by_id(org.id()).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id(), org.id());
        assert_eq!(found.name().as_str(), "Test Organization");
        assert_eq!(found.slug().as_str(), "test-org");

        // 根据 slug 查找
        let found_by_slug = repo.find_by_slug(&tenant_id, "test-org").await.unwrap();
        assert!(found_by_slug.is_some());
        assert_eq!(found_by_slug.unwrap().id(), org.id());

        // 清理
        repo.delete(org.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_find_by_tenant() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresOrganizationRepository::new(&conn);

        let tenant_id = TenantId::generate();
        let org1 =
            Organization::create(tenant_id.clone(), "Org 1".to_string(), "org-1".to_string())
                .unwrap();
        let org2 =
            Organization::create(tenant_id.clone(), "Org 2".to_string(), "org-2".to_string())
                .unwrap();

        repo.save(&org1).await.unwrap();
        repo.save(&org2).await.unwrap();

        let orgs = repo.find_by_tenant(&tenant_id).await.unwrap();
        assert_eq!(orgs.len(), 2);

        // 清理
        repo.delete(org1.id()).await.unwrap();
        repo.delete(org2.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_find_nonexistent_organization() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresOrganizationRepository::new(&conn);

        let nonexistent_id = OrganizationId::generate();
        let found = repo.find_by_id(&nonexistent_id).await.unwrap();
        assert!(found.is_none());

        let found = repo
            .find_by_slug(&TenantId::generate(), "nonexistent")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_update_organization() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresOrganizationRepository::new(&conn);

        let tenant_id = TenantId::generate();
        let mut org = Organization::create(
            tenant_id.clone(),
            "Test Organization".to_string(),
            "test-org".to_string(),
        )
        .unwrap();

        repo.save(&org).await.unwrap();

        // 修改并保存
        org.update_name("Updated Organization".to_string()).unwrap();
        repo.save(&org).await.unwrap();

        // 验证更新
        let found = repo.find_by_id(org.id()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name().as_str(), "Updated Organization");

        // 清理
        repo.delete(org.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_organization() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresOrganizationRepository::new(&conn);

        let org = Organization::create(
            TenantId::generate(),
            "Test Organization".to_string(),
            "test-org".to_string(),
        )
        .unwrap();

        repo.save(&org).await.unwrap();
        repo.delete(org.id()).await.unwrap();

        let found = repo.find_by_id(org.id()).await.unwrap();
        assert!(found.is_none());
    }
}
