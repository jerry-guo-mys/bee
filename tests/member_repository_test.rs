//! MembershipRepository PostgreSQL 集成测试
//!
//! 测试 PostgresMembershipRepository 的实现正确性。

#![cfg(feature = "postgres")]

use bee::domain::common::{MembershipRole, MembershipStatus};
use bee::domain::member::{
    MemberDomainError, Membership, MembershipFilter,
    ToolId, ToolPolicy, ToolRiskLevel, UserEmail,
};
use bee::domain::member::repository::MembershipRepository;
use bee::domain::tenant::value_object::{
    MembershipId, OrganizationId, TeamId, TenantId, UserId,
};
use bee::infrastructure::persistence::postgres::PostgresConnection;

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

/// 创建测试用的 MembershipRepository
fn create_repo(conn: &PostgresConnection) -> impl MembershipRepository<Error = MemberDomainError> {
    bee::domain::member::PostgresMembershipRepository::new(conn)
}

// ============================================================================
// 基础 CRUD 测试
// ============================================================================

#[tokio::test]
async fn test_save_and_find_membership() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

    // 创建测试数据
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    // 创建 Membership
    let mut membership = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        Some(UserId::generate()),
        email.clone(),
        MembershipRole::Member,
    ).unwrap();

    // 接受邀请
    let user_id = UserId::generate();
    membership.accept_invite(user_id.clone()).unwrap();

    // 保存
    repo.save(&membership).await.unwrap();

    // 根据 ID 查找
    let found = repo.find_by_id(membership.id()).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id(), membership.id());
    assert_eq!(found.status(), &MembershipStatus::Active);
    assert_eq!(found.user_id(), Some(&user_id));
    assert_eq!(found.email(), &email);

    // 清理
    repo.delete(membership.id()).await.unwrap();
}

#[tokio::test]
async fn test_find_nonexistent_membership() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

    let nonexistent_id = MembershipId::generate();
    let found = repo.find_by_id(&nonexistent_id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_update_membership() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

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
    ).unwrap();
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

#[tokio::test]
async fn test_delete_membership() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

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
    ).unwrap();

    // 保存
    repo.save(&membership).await.unwrap();

    // 删除
    repo.delete(membership.id()).await.unwrap();

    // 验证已删除
    let found = repo.find_by_id(membership.id()).await.unwrap();
    assert!(found.is_none());
}

// ============================================================================
// 查询方法测试
// ============================================================================

#[tokio::test]
async fn test_find_by_user() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

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
    ).unwrap();
    membership1.accept_invite(user_id.clone()).unwrap();

    let mut membership2 = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        Some(user_id.clone()),
        email.clone(),
        MembershipRole::OrgAdmin,
    ).unwrap();
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
    let repo = create_repo(&conn);

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
    ).unwrap();
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
async fn test_find_by_team() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let team_id = TeamId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    // 创建 Membership
    let mut membership = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        Some(team_id.clone()),
        None,
        email.clone(),
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(UserId::generate()).unwrap();

    // 保存
    repo.save(&membership).await.unwrap();

    // 根据团队查找
    let founds = repo.find_by_team(&team_id).await.unwrap();
    assert_eq!(founds.len(), 1);
    assert_eq!(founds[0].id(), membership.id());

    // 清理
    repo.delete(membership.id()).await.unwrap();
}

#[tokio::test]
async fn test_find_by_tenant() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

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
    ).unwrap();
    membership.accept_invite(UserId::generate()).unwrap();

    // 保存
    repo.save(&membership).await.unwrap();

    // 根据租户查找
    let founds = repo.find_by_tenant(&tenant_id).await.unwrap();
    assert_eq!(founds.len(), 1);
    assert_eq!(founds[0].id(), membership.id());

    // 清理
    repo.delete(membership.id()).await.unwrap();
}

// ============================================================================
// 工具策略测试
// ============================================================================

#[tokio::test]
async fn test_save_and_load_tool_policies() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

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
    ).unwrap();
    membership.accept_invite(UserId::generate()).unwrap();

    // 添加工具策略
    let policy1 = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::Medium,
        true,
    );
    let policy2 = ToolPolicy::new(
        ToolId::from_str("file_read"),
        ToolRiskLevel::Low,
        true,
    ).with_note("只读访问".to_string());

    membership.add_tool_policy(policy1).unwrap();
    membership.add_tool_policy(policy2).unwrap();

    // 保存
    repo.save(&membership).await.unwrap();

    // 重新加载
    let loaded = repo.find_by_id(membership.id()).await.unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();

    assert_eq!(loaded.tool_policies().len(), 2);
    assert_eq!(loaded.tool_policies()[0].tool_id().as_str(), "shell");
    assert_eq!(loaded.tool_policies()[0].risk_level(), ToolRiskLevel::Medium);
    assert_eq!(loaded.tool_policies()[1].tool_id().as_str(), "file_read");

    // 清理
    repo.delete(membership.id()).await.unwrap();
}

#[tokio::test]
async fn test_delete_membership_cascades_to_tool_policies() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    // 创建 Membership 并添加工具策略
    let mut membership = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email.clone(),
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(UserId::generate()).unwrap();

    let policy = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::Medium,
        true,
    );
    membership.add_tool_policy(policy).unwrap();

    // 保存
    repo.save(&membership).await.unwrap();

    // 删除
    repo.delete(membership.id()).await.unwrap();

    // 验证成员和工具策略都已删除
    let found = repo.find_by_id(membership.id()).await.unwrap();
    assert!(found.is_none());
}

// ============================================================================
// 过滤器测试
// ============================================================================

#[tokio::test]
async fn test_find_by_filter_with_role() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    // 创建不同角色的 Membership
    let mut member = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email.clone(),
        MembershipRole::Member,
    ).unwrap();
    member.accept_invite(UserId::generate()).unwrap();

    let mut admin = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email.clone(),
        MembershipRole::OrgAdmin,
    ).unwrap();
    admin.accept_invite(UserId::generate()).unwrap();

    // 保存
    repo.save(&member).await.unwrap();
    repo.save(&admin).await.unwrap();

    // 按角色过滤
    let filter = MembershipFilter::new()
        .with_role(MembershipRole::OrgAdmin);

    let founds = repo.find_by_filter(&filter).await.unwrap();
    assert_eq!(founds.len(), 1);
    assert_eq!(founds[0].role(), &MembershipRole::OrgAdmin);

    // 清理
    repo.delete(member.id()).await.unwrap();
    repo.delete(admin.id()).await.unwrap();
}

#[tokio::test]
async fn test_find_by_filter_with_status() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    // 创建不同状态的 Membership
    let mut active = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email.clone(),
        MembershipRole::Member,
    ).unwrap();
    active.accept_invite(UserId::generate()).unwrap();

    let pending = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email.clone(),
        MembershipRole::Member,
    ).unwrap();
    // 保持 Pending 状态

    // 保存
    repo.save(&active).await.unwrap();
    repo.save(&pending).await.unwrap();

    // 按状态过滤
    let filter = MembershipFilter::new()
        .with_status(MembershipStatus::Active);

    let founds = repo.find_by_filter(&filter).await.unwrap();
    assert_eq!(founds.len(), 1);
    assert_eq!(founds[0].status(), &MembershipStatus::Active);

    // 清理
    repo.delete(active.id()).await.unwrap();
    repo.delete(pending.id()).await.unwrap();
}

#[tokio::test]
async fn test_find_by_filter_combined() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let team_id = TeamId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    // 创建多个 Membership
    let mut member1 = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        Some(team_id.clone()),
        None,
        email.clone(),
        MembershipRole::Member,
    ).unwrap();
    member1.accept_invite(UserId::generate()).unwrap();

    let mut member2 = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email.clone(),
        MembershipRole::OrgAdmin,
    ).unwrap();
    member2.accept_invite(UserId::generate()).unwrap();

    // 保存
    repo.save(&member1).await.unwrap();
    repo.save(&member2).await.unwrap();

    // 组合过滤
    let filter = MembershipFilter::new()
        .with_tenant_id(tenant_id.clone())
        .with_organization_id(org_id.clone())
        .with_team_id(team_id.clone())
        .with_status(MembershipStatus::Active);

    let founds = repo.find_by_filter(&filter).await.unwrap();
    assert_eq!(founds.len(), 1);
    assert_eq!(founds[0].id(), member1.id());

    // 清理
    repo.delete(member1.id()).await.unwrap();
    repo.delete(member2.id()).await.unwrap();
}

// ============================================================================
// 存在性和计数测试
// ============================================================================

#[tokio::test]
async fn test_membership_exists() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

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
    ).unwrap();

    // 初始不存在
    let exists = repo.exists(membership.id()).await.unwrap();
    assert!(!exists);

    // 保存后存在
    repo.save(&membership).await.unwrap();
    let exists = repo.exists(membership.id()).await.unwrap();
    assert!(exists);

    // 删除后不存在
    repo.delete(membership.id()).await.unwrap();
    let exists = repo.exists(membership.id()).await.unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn test_count_memberships() {
    let conn = match get_test_connection().await {
        Some(c) => c,
        None => return,
    };
    let repo = create_repo(&conn);

    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    // 创建多个 Membership
    let mut member1 = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email.clone(),
        MembershipRole::Member,
    ).unwrap();
    member1.accept_invite(UserId::generate()).unwrap();

    let mut member2 = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email.clone(),
        MembershipRole::OrgAdmin,
    ).unwrap();
    member2.accept_invite(UserId::generate()).unwrap();

    // 保存
    repo.save(&member1).await.unwrap();
    repo.save(&member2).await.unwrap();

    // 计数
    let filter = MembershipFilter::new()
        .with_organization_id(org_id.clone());
    let count = repo.count(&filter).await.unwrap();
    assert_eq!(count, 2);

    // 按角色计数
    let filter = MembershipFilter::new()
        .with_role(MembershipRole::Member);
    let count = repo.count(&filter).await.unwrap();
    assert_eq!(count, 1);

    // 清理
    repo.delete(member1.id()).await.unwrap();
    repo.delete(member2.id()).await.unwrap();
}
