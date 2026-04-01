use super::logger::AuditLog;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::error::Error;

/// 审计日志 Repository trait
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    type Error: Error + Send + Sync;

    /// 保存审计日志
    async fn save(&self, log: &AuditLog) -> Result<(), Self::Error>;

    /// 批量保存
    async fn save_batch(&self, logs: &[AuditLog]) -> Result<(), Self::Error> {
        for log in logs {
            self.save(log).await?;
        }
        Ok(())
    }

    /// 按租户查询
    async fn find_by_tenant(
        &self,
        tenant_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<AuditLog>, Self::Error>;

    /// 按资源查询
    async fn find_by_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        limit: usize,
    ) -> Result<Vec<AuditLog>, Self::Error>;
}

#[cfg(feature = "postgres")]
pub mod postgres {
    use super::*;
    use crate::infrastructure::persistence::postgres::PostgresConnection;
    use sqlx::PgPool;

    /// PostgreSQL 审计日志 Repository
    pub struct PostgresAuditLogRepository {
        pool: PgPool,
    }

    impl PostgresAuditLogRepository {
        pub fn new(conn: &PostgresConnection) -> Self {
            Self {
                pool: conn.pool().clone(),
            }
        }
    }

    #[async_trait]
    impl AuditLogRepository for PostgresAuditLogRepository {
        type Error = sqlx::Error;

        async fn save(&self, log: &AuditLog) -> Result<(), Self::Error> {
            sqlx::query(
                r#"
                INSERT INTO audit_logs
                    (id, tenant_id, organization_id, team_id, user_id, action, resource_type, resource_id, detail_json, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(&log.id)
            .bind(&log.tenant_id)
            .bind(&log.organization_id)
            .bind(&log.team_id)
            .bind(&log.user_id)
            .bind(&log.action)
            .bind(&log.resource_type)
            .bind(&log.resource_id)
            .bind(&log.detail_json)
            .bind(log.created_at)
            .execute(&self.pool)
            .await?;

            Ok(())
        }

        async fn find_by_tenant(
            &self,
            tenant_id: &str,
            from: Option<DateTime<Utc>>,
            to: Option<DateTime<Utc>>,
            limit: usize,
        ) -> Result<Vec<AuditLog>, Self::Error> {
            let mut sql = String::from("SELECT * FROM audit_logs WHERE tenant_id = $1");
            let mut param_idx = 2;

            let from_idx = if from.is_some() {
                sql.push_str(" AND created_at >= $");
                sql.push_str(&param_idx.to_string());
                param_idx += 1;
                Some(param_idx - 1)
            } else {
                None
            };

            let to_idx = if to.is_some() {
                sql.push_str(" AND created_at <= $");
                sql.push_str(&param_idx.to_string());
                param_idx += 1;
                Some(param_idx - 1)
            } else {
                None
            };

            sql.push_str(" ORDER BY created_at DESC LIMIT $");
            sql.push_str(&param_idx.to_string());

            let mut query = sqlx::query_as::<_, AuditLog>(&sql);
            query = query.bind(tenant_id);

            if let Some(_) = from_idx {
                if let Some(from_val) = from {
                    query = query.bind(from_val);
                }
            }

            if let Some(_) = to_idx {
                if let Some(to_val) = to {
                    query = query.bind(to_val);
                }
            }

            query = query.bind(limit as i64);

            query.fetch_all(&self.pool).await
        }

        async fn find_by_resource(
            &self,
            resource_type: &str,
            resource_id: &str,
            limit: usize,
        ) -> Result<Vec<AuditLog>, Self::Error> {
            sqlx::query_as::<_, AuditLog>(
                r#"SELECT * FROM audit_logs WHERE resource_type = $1 AND resource_id = $2 ORDER BY created_at DESC LIMIT $3"#,
            )
            .bind(resource_type)
            .bind(resource_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        }
    }
}
