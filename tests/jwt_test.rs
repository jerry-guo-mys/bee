use bee::infrastructure::auth::{BeeClaims, JwtService};

#[test]
fn test_generate_and_validate_token() {
    let service = JwtService::new();

    let claims = BeeClaims::new(
        "user-123",
        Some("tenant-456"),
        Some("org-789"),
        None,
        vec!["Member".to_string()],
    );

    let token = service.generate_token(&claims).unwrap();
    let validated = service.validate_token(&token).unwrap();

    assert_eq!(validated.user_id, claims.user_id);
    assert_eq!(validated.tenant_id, claims.tenant_id);
    assert!(validated.has_role("Member"));
}

#[test]
fn test_token_expiry() {
    let service = JwtService::new();
    let claims = BeeClaims::new("user-123", None, None, None, vec![]);
    let token = service.generate_token(&claims).unwrap();
    assert!(token.len() > 100);
}

#[test]
fn test_has_role() {
    let claims = BeeClaims::new(
        "user-123",
        None,
        None,
        None,
        vec!["Admin".to_string(), "Member".to_string()],
    );

    assert!(claims.has_role("Admin"));
    assert!(claims.has_role("Member"));
    assert!(!claims.has_role("Guest"));
}

#[test]
fn test_has_permission() {
    let mut claims = BeeClaims::new(
        "user-123",
        None,
        None,
        None,
        vec!["Member".to_string()],
    );
    claims.permissions = vec!["read:documents".to_string()];

    assert!(claims.has_permission("read:documents"));
    assert!(!claims.has_permission("write:documents"));

    // PlatformAdmin 角色自动拥有所有权限
    let admin_claims = BeeClaims::new(
        "admin-123",
        None,
        None,
        None,
        vec!["PlatformAdmin".to_string()],
    );
    assert!(admin_claims.has_permission("any:permission"));
}

#[tokio::test]
async fn test_refresh_token() {
    let service = JwtService::new();
    let claims = BeeClaims::new("user-123", None, None, None, vec![]);

    let token1 = service.generate_token(&claims).unwrap();
    let validated1 = service.validate_token(&token1).unwrap();

    // 等待 1 秒确保刷新后的 token 过期时间更大
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let token2 = service.refresh_token(&validated1).unwrap();
    let validated2 = service.validate_token(&token2).unwrap();

    assert_eq!(validated1.user_id, validated2.user_id);
    assert!(validated2.exp > validated1.exp);
}
