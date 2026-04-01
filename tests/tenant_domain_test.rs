//! 租户领域单元测试
//!
//! 测试 Tenant 聚合根、值对象和领域事件的正确性。

use bee::domain::common::{MembershipRole, Permission, TenantStatus};
use bee::domain::tenant::{
    AgentId, InMemoryTenantRepository, MembershipId, OrganizationId, TeamId, Tenant, TenantError,
    TenantId, TenantName, TenantRepository, TenantSlug, UserId,
};

// ==================== Tenant 创建测试 ====================

#[test]
fn test_tenant_create_success() {
    let tenant = Tenant::create("My Organization".to_string(), "my-org".to_string()).unwrap();

    assert_eq!(tenant.name().as_str(), "My Organization");
    assert_eq!(tenant.slug().as_str(), "my-org");
    assert_eq!(tenant.status(), &TenantStatus::Active);
    assert!(tenant.organizations().is_empty());
}

#[test]
fn test_tenant_create_with_whitespace_name() {
    let tenant =
        Tenant::create("  Trimmed Name  ".to_string(), "trimmed-name".to_string()).unwrap();

    assert_eq!(tenant.name().as_str(), "Trimmed Name");
}

#[test]
fn test_tenant_create_invalid_name_empty() {
    let result = Tenant::create("".to_string(), "valid-slug".to_string());
    assert!(matches!(result, Err(TenantError::InvalidName(_))));
}

#[test]
fn test_tenant_create_invalid_name_whitespace_only() {
    let result = Tenant::create("   ".to_string(), "valid-slug".to_string());
    assert!(matches!(result, Err(TenantError::InvalidName(_))));
}

#[test]
fn test_tenant_create_invalid_slug_uppercase() {
    // TenantSlug 会自动将输入转换为小写，所以大写字母不会导致错误
    let tenant = Tenant::create("Valid Name".to_string(), "INVALID".to_string());
    assert!(tenant.is_ok());
    assert_eq!(tenant.unwrap().slug().as_str(), "invalid");
}

#[test]
fn test_tenant_create_invalid_slug_special_chars() {
    let result = Tenant::create("Valid Name".to_string(), "invalid_slug".to_string());
    assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
}

#[test]
fn test_tenant_create_invalid_slug_starts_with_hyphen() {
    let result = Tenant::create("Valid Name".to_string(), "-invalid".to_string());
    assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
}

#[test]
fn test_tenant_create_invalid_slug_ends_with_hyphen() {
    let result = Tenant::create("Valid Name".to_string(), "invalid-".to_string());
    assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
}

// ==================== Tenant 状态转换测试 ====================

#[test]
fn test_tenant_suspend() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    assert!(tenant.is_active());

    tenant.suspend();
    assert!(tenant.is_suspended());
    assert_eq!(tenant.status(), &TenantStatus::Suspended);
}

#[test]
fn test_tenant_restore() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    tenant.suspend();
    assert!(tenant.is_suspended());

    tenant.restore();
    assert!(tenant.is_active());
    assert_eq!(tenant.status(), &TenantStatus::Active);
}

#[test]
fn test_tenant_archive() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    assert!(tenant.is_active());

    tenant.archive();
    assert!(tenant.is_archived());
    assert_eq!(tenant.status(), &TenantStatus::Archived);
}

#[test]
fn test_tenant_archive_from_suspended() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    tenant.suspend();
    assert!(tenant.is_suspended());

    tenant.archive();
    assert!(tenant.is_archived());
}

#[test]
fn test_tenant_cannot_restore_from_archived() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    tenant.archive();
    tenant.restore();

    assert!(tenant.is_archived());
}

#[test]
fn test_tenant_state_transitions_complete() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    // Active -> Suspended -> Active
    tenant.suspend();
    assert!(tenant.is_suspended());

    tenant.restore();
    assert!(tenant.is_active());

    // Active -> Archived (terminal state)
    tenant.archive();
    assert!(tenant.is_archived());

    // 尝试从 Archived 转换到其他状态应该无效
    tenant.suspend();
    assert!(tenant.is_archived());

    tenant.restore();
    assert!(tenant.is_archived());
}

// ==================== Tenant 组织管理测试 ====================

#[test]
fn test_tenant_add_organization() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    assert_eq!(tenant.organizations().len(), 0);

    let org_id = OrganizationId::generate();
    tenant.add_organization(org_id.clone());

    assert_eq!(tenant.organizations().len(), 1);
    assert!(tenant.organizations().contains(&org_id));
}

#[test]
fn test_tenant_add_duplicate_organization() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    let org_id = OrganizationId::generate();
    tenant.add_organization(org_id.clone());
    tenant.add_organization(org_id.clone());

    assert_eq!(tenant.organizations().len(), 1);
}

#[test]
fn test_tenant_remove_organization() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    let org_id = OrganizationId::generate();
    tenant.add_organization(org_id.clone());
    assert_eq!(tenant.organizations().len(), 1);

    tenant.remove_organization(&org_id);
    assert_eq!(tenant.organizations().len(), 0);
}

#[test]
fn test_tenant_remove_non_existent_organization() {
    let mut tenant = Tenant::create("Test Tenant".to_string(), "test-tenant".to_string()).unwrap();

    let org_id = OrganizationId::generate();
    tenant.remove_organization(&org_id);

    assert_eq!(tenant.organizations().len(), 0);
}

// ==================== 值对象测试 ====================

#[test]
fn test_tenant_id_generation() {
    let id1 = TenantId::generate();
    let id2 = TenantId::generate();

    assert_ne!(id1, id2);
    assert_eq!(id1.as_str().len(), 36);
}

#[test]
fn test_tenant_id_from_str() {
    let id = TenantId::from_str("custom-id-12345");
    assert_eq!(id.as_str(), "custom-id-12345");
}

#[test]
fn test_organization_id() {
    let id1 = OrganizationId::generate();
    let id2 = OrganizationId::from_str("org-custom");

    assert_ne!(id1.as_str(), id2.as_str());
    assert_eq!(id2.as_str(), "org-custom");
}

#[test]
fn test_team_id() {
    let id = TeamId::generate();
    assert_eq!(id.as_str().len(), 36);
}

#[test]
fn test_user_id() {
    let id = UserId::generate();
    assert_eq!(id.as_str().len(), 36);

    let id2 = UserId::new("user-custom".to_string());
    assert_eq!(id2.as_str(), "user-custom");
}

#[test]
fn test_agent_id() {
    let id = AgentId::generate();
    assert_eq!(id.as_str().len(), 36);
}

#[test]
fn test_membership_id() {
    let id = MembershipId::generate();
    assert_eq!(id.as_str().len(), 36);
}

#[test]
fn test_tenant_name_validation() {
    // 有效名称
    assert!(TenantName::new("Valid Name".to_string()).is_ok());

    // 空名称
    assert!(TenantName::new("".to_string()).is_err());

    // 只有空格
    assert!(TenantName::new("   ".to_string()).is_err());

    // 太长
    let long_name = "a".repeat(300);
    assert!(TenantName::new(long_name).is_err());
}

#[test]
fn test_tenant_slug_validation() {
    // 有效 slug
    assert!(TenantSlug::new("valid-slug-123".to_string()).is_ok());

    // 自动转换为小写
    let slug = TenantSlug::new("UPPERCASE".to_string()).unwrap();
    assert_eq!(slug.as_str(), "uppercase");

    // 空 slug
    assert!(TenantSlug::new("".to_string()).is_err());

    // 特殊字符
    assert!(TenantSlug::new("invalid_slug".to_string()).is_err());
    assert!(TenantSlug::new("invalid.slug".to_string()).is_err());
    assert!(TenantSlug::new("invalid slug".to_string()).is_err());

    // 连字符位置
    assert!(TenantSlug::new("-invalid".to_string()).is_err());
    assert!(TenantSlug::new("invalid-".to_string()).is_err());
}

// ==================== MembershipRole 权限测试 ====================

#[test]
fn test_membership_role_platform_admin() {
    let admin = MembershipRole::PlatformAdmin;

    assert!(admin.has_permission(&Permission::TenantRead));
    assert!(admin.has_permission(&Permission::TenantWrite));
    assert!(admin.has_permission(&Permission::TenantDelete));
    assert!(admin.has_permission(&Permission::OrgRead));
    assert!(admin.has_permission(&Permission::OrgWrite));
    assert!(admin.has_permission(&Permission::TeamRead));
    assert!(admin.has_permission(&Permission::TeamWrite));
    assert!(admin.has_permission(&Permission::AgentRead));
    assert!(admin.has_permission(&Permission::AgentExecute));
    assert!(admin.has_permission(&Permission::ToolExecute("shell".to_string())));
}

#[test]
fn test_membership_role_org_admin() {
    let admin = MembershipRole::OrgAdmin;

    assert!(admin.has_permission(&Permission::TenantRead));
    assert!(admin.has_permission(&Permission::OrgRead));
    assert!(admin.has_permission(&Permission::OrgWrite));
    assert!(!admin.has_permission(&Permission::OrgDelete));
    assert!(!admin.has_permission(&Permission::TenantWrite));
}

#[test]
fn test_membership_role_team_admin() {
    let admin = MembershipRole::TeamAdmin;

    assert!(admin.has_permission(&Permission::TeamRead));
    assert!(admin.has_permission(&Permission::TeamWrite));
    assert!(admin.has_permission(&Permission::OrgRead));
    assert!(admin.has_permission(&Permission::AgentRead));
    assert!(admin.has_permission(&Permission::AgentExecute));
    assert!(!admin.has_permission(&Permission::TeamDelete));
}

#[test]
fn test_membership_role_member() {
    let member = MembershipRole::Member;

    assert!(member.has_permission(&Permission::AgentRead));
    assert!(member.has_permission(&Permission::AgentExecute));
    assert!(member.has_permission(&Permission::TeamRead));
    assert!(member.has_permission(&Permission::OrgRead));
    assert!(member.has_permission(&Permission::TenantRead));
    assert!(!member.has_permission(&Permission::TeamWrite));
    assert!(!member.has_permission(&Permission::AgentModify));
}

#[test]
fn test_membership_role_viewer() {
    let viewer = MembershipRole::Viewer;

    assert!(viewer.has_permission(&Permission::TenantRead));
    assert!(viewer.has_permission(&Permission::OrgRead));
    assert!(viewer.has_permission(&Permission::TeamRead));
    assert!(viewer.has_permission(&Permission::AgentRead));
    assert!(!viewer.has_permission(&Permission::TenantWrite));
    assert!(!viewer.has_permission(&Permission::AgentExecute));
}

// ==================== Repository 测试 ====================

#[tokio::test]
async fn test_tenant_repository_save_and_find() {
    let repo = InMemoryTenantRepository::new();

    let tenant = Tenant::create("Repository Test".to_string(), "repo-test".to_string()).unwrap();

    let tenant_id = tenant.id().clone();
    let tenant_slug = tenant.slug().as_str().to_string();

    // 保存
    repo.save(&tenant).await.unwrap();

    // 根据 ID 查找
    let found = repo.find_by_id(&tenant_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name().as_str(), "Repository Test");

    // 根据 slug 查找
    let found = repo.find_by_slug(&tenant_slug).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().slug().as_str(), "repo-test");
}

#[tokio::test]
async fn test_tenant_repository_delete() {
    let repo = InMemoryTenantRepository::new();

    let tenant = Tenant::create("Delete Test".to_string(), "delete-test".to_string()).unwrap();

    let tenant_id = tenant.id().clone();

    repo.save(&tenant).await.unwrap();
    repo.delete(&tenant_id).await.unwrap();

    let found = repo.find_by_id(&tenant_id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_tenant_repository_exists_by_slug() {
    let repo = InMemoryTenantRepository::new();

    let tenant = Tenant::create("Exists Test".to_string(), "exists-test".to_string()).unwrap();

    // 保存前不存在
    assert!(!repo.exists_by_slug("exists-test").await.unwrap());

    repo.save(&tenant).await.unwrap();

    // 保存后存在
    assert!(repo.exists_by_slug("exists-test").await.unwrap());
    assert!(!repo.exists_by_slug("non-existent").await.unwrap());
}

#[tokio::test]
async fn test_tenant_repository_update() {
    let repo = InMemoryTenantRepository::new();

    let mut tenant = Tenant::create("Update Test".to_string(), "update-test".to_string()).unwrap();

    repo.save(&tenant).await.unwrap();

    // 修改租户
    tenant.add_organization(OrganizationId::generate());
    repo.save(&tenant).await.unwrap();

    let found = repo.find_by_id(tenant.id()).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().organizations().len(), 1);
}
