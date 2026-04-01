use async_trait::async_trait;
use bee::infrastructure::audit::{AuditLog, AuditLogRepository};
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

/// Mock repository for testing
struct MockAuditLogRepository {
    logs: Arc<Mutex<Vec<AuditLog>>>,
}

impl MockAuditLogRepository {
    fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl AuditLogRepository for MockAuditLogRepository {
    type Error = std::io::Error;

    async fn save(&self, log: &AuditLog) -> Result<(), Self::Error> {
        self.logs.lock().unwrap().push(log.clone());
        Ok(())
    }

    async fn find_by_tenant(
        &self,
        _tenant_id: &str,
        _from: Option<DateTime<Utc>>,
        _to: Option<DateTime<Utc>>,
        _limit: usize,
    ) -> Result<Vec<AuditLog>, Self::Error> {
        Ok(self.logs.lock().unwrap().clone())
    }

    async fn find_by_resource(
        &self,
        _resource_type: &str,
        _resource_id: &str,
        _limit: usize,
    ) -> Result<Vec<AuditLog>, Self::Error> {
        Ok(self.logs.lock().unwrap().clone())
    }
}

#[tokio::test]
async fn test_audit_logger() {
    let repository = MockAuditLogRepository::new();
    let logger = bee::infrastructure::audit::AuditLogger::new(repository);

    let log = AuditLog::new("tenant-1", "MEMBER_INVITE", "membership", "member-123")
        .with_organization("org-456")
        .with_user("user-789")
        .with_detail(serde_json::json!({
            "role": "Member",
            "inviter": "admin-1"
        }));

    let result = logger.log(&log).await;
    assert!(result.is_ok());
}

#[test]
fn test_audit_log_builder() {
    let log = AuditLog::new("tenant-1", "CREATE", "tenant", "tenant-1")
        .with_organization("org-1")
        .with_team("team-1")
        .with_user("user-1")
        .with_detail(serde_json::json!({"key": "value"}));

    assert_eq!(log.tenant_id, "tenant-1");
    assert_eq!(log.action, "CREATE");
    assert!(log.organization_id.is_some());
    assert!(log.team_id.is_some());
    assert!(log.user_id.is_some());
    assert!(log.detail_json.is_some());
}
