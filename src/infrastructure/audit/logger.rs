use super::repository::AuditLogRepository;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
// use sqlx::FromRow;  // Temporarily disabled for compilation check
use std::sync::Arc;

/// 审计日志记录
#[derive(Debug, Clone, Serialize, Deserialize)] // , FromRow)]
pub struct AuditLog {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub team_id: Option<String>,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub detail_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl AuditLog {
    pub fn new(tenant_id: &str, action: &str, resource_type: &str, resource_id: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            organization_id: None,
            team_id: None,
            user_id: None,
            action: action.to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            detail_json: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_organization(mut self, org_id: &str) -> Self {
        self.organization_id = Some(org_id.to_string());
        self
    }

    pub fn with_team(mut self, team_id: &str) -> Self {
        self.team_id = Some(team_id.to_string());
        self
    }

    pub fn with_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    pub fn with_detail(mut self, detail: serde_json::Value) -> Self {
        self.detail_json = Some(detail);
        self
    }
}

/// 审计日志记录器
pub struct AuditLogger<R: AuditLogRepository> {
    repository: Arc<R>,
}

impl<R: AuditLogRepository + 'static> AuditLogger<R> {
    pub fn new(repository: R) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }

    /// 记录审计日志
    pub async fn log(&self, log: &AuditLog) -> Result<(), AuditError> {
        // Save to repository
        self.repository
            .save(log)
            .await
            .map_err(|e| AuditError::Database(Box::new(e)))?;

        // 同时输出到 tracing
        tracing::info!(
            target: "audit",
            tenant_id = %log.tenant_id,
            action = %log.action,
            resource_type = %log.resource_type,
            resource_id = %log.resource_id,
            "Audit log created"
        );

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
