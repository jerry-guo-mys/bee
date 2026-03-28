use bee::infrastructure::audit::{AuditLog, AuditLogger};

#[tokio::test]
async fn test_audit_logger() {
    let logger = AuditLogger::new();

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
