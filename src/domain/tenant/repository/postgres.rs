//! PostgreSQL 实现的 TenantRepository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::common::TenantStatus;
use crate::domain::tenant::entity::Tenant;
use crate::domain::tenant::value_object::{TenantError, TenantId, TenantName, TenantSlug};
use crate::infrastructure::persistence::postgres::PostgresConnection;

/// PostgreSQL 租户仓库实现
pub struct PostgresTenantRepository {
    pool: PgPool,
}

impl PostgresTenantRepository {
    /// 创建新的 PostgreSQL 租户仓库
    pub fn new(conn: &PostgresConnection) -> Self {
        Self {
            pool: conn.pool().clone(),
        }
    }
}

/// 数据库行结构
#[derive(FromRow)]
struct TenantRow {
    id: uuid::Uuid,
    name: String,
    slug: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TenantRow {
    /// 将数据库行转换为 Tenant 实体
    fn into_tenant(self) -> Result<Tenant, TenantError> {
        let id = TenantId::new(self.id.to_string());
        let name = TenantName::new(self.name)
            .map_err(|e| TenantError::InvalidName(e.to_string()))?;
        let slug = TenantSlug::new(self.slug)
            .map_err(|e| TenantError::InvalidSlug(e.to_string()))?;
        let status = self
            .status
            .parse::<TenantStatus>()
            .map_err(|e| TenantError::InvalidStatus(e))?;

        Ok(Tenant::load(
            id,
            name,
            slug,
            status,
            self.created_at,
            self.updated_at,
            Vec::new(), // organizations 从单独的表加载
        ))
    }
}

#[async_trait]
impl super::TenantRepository for PostgresTenantRepository {
    async fn save(&self, tenant: &Tenant) -> Result<(), TenantError> {
        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                name = $2, slug = $3, status = $4, updated_at = $6
            "#,
        )
        .bind(tenant.id().as_str().parse::<uuid::Uuid>().map_err(|e| {
            TenantError::DatabaseError(format!("Invalid UUID: {}", e))
        })?)
        .bind(tenant.name().as_str())
        .bind(tenant.slug().as_str())
        .bind(tenant.status().to_string())
        .bind(tenant.created_at())
        .bind(tenant.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                TenantError::AlreadyExists(tenant.id().to_string())
            }
            _ => TenantError::DatabaseError(e.to_string()),
        })?;

        Ok(())
    }

    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError> {
        let tenant = sqlx::query_as::<_, TenantRow>(
            "SELECT * FROM tenants WHERE id = $1",
        )
        .bind(id.as_str().parse::<uuid::Uuid>().map_err(|e| {
            TenantError::DatabaseError(format!("Invalid UUID: {}", e))
        })?)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TenantError::DatabaseError(e.to_string()))?;

        match tenant {
            Some(row) => row.into_tenant().map(Some),
            None => Ok(None),
        }
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, TenantError> {
        let tenant = sqlx::query_as::<_, TenantRow>(
            "SELECT * FROM tenants WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TenantError::DatabaseError(e.to_string()))?;

        match tenant {
            Some(row) => row.into_tenant().map(Some),
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &TenantId) -> Result<(), TenantError> {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id.as_str().parse::<uuid::Uuid>().map_err(|e| {
                TenantError::DatabaseError(format!("Invalid UUID: {}", e))
            })?)
            .execute(&self.pool)
            .await
            .map_err(|e| TenantError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tenant::repository::TenantRepository;

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
    async fn test_save_and_find_tenant() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTenantRepository::new(&conn);

        // 创建测试租户
        let name = TenantName::new("Test Tenant".to_string()).unwrap();
        let slug = TenantSlug::new("test-tenant".to_string()).unwrap();
        let tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

        // 保存
        repo.save(&tenant).await.unwrap();

        // 根据 ID 查找
        let found = repo.find_by_id(tenant.id()).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id(), tenant.id());
        assert_eq!(found.name().as_str(), "Test Tenant");
        assert_eq!(found.slug().as_str(), "test-tenant");
        assert_eq!(found.status(), &TenantStatus::Active);

        // 根据 slug 查找
        let found_by_slug = repo.find_by_slug("test-tenant").await.unwrap();
        assert!(found_by_slug.is_some());
        assert_eq!(found_by_slug.unwrap().id(), tenant.id());

        // 清理
        repo.delete(tenant.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_find_nonexistent_tenant() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTenantRepository::new(&conn);

        let nonexistent_id = TenantId::generate();
        let found = repo.find_by_id(&nonexistent_id).await.unwrap();
        assert!(found.is_none());

        let found = repo.find_by_slug("nonexistent-slug").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_update_tenant() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTenantRepository::new(&conn);

        // 创建并保存
        let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();
        repo.save(&tenant).await.unwrap();

        // 修改并保存
        tenant.suspend();
        repo.save(&tenant).await.unwrap();

        // 验证更新
        let found = repo.find_by_id(tenant.id()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().status(), &TenantStatus::Suspended);

        // 清理
        repo.delete(tenant.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTenantRepository::new(&conn);

        // 创建并保存
        let tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();
        repo.save(&tenant).await.unwrap();

        // 删除
        repo.delete(tenant.id()).await.unwrap();

        // 验证已删除
        let found = repo.find_by_id(tenant.id()).await.unwrap();
        assert!(found.is_none());
    }
}
