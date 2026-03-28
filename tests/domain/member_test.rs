//! 成员领域单元测试
//!
//! 测试 Membership 聚合根、值对象、领域服务和事件的正确性。

use bee::domain::common::{MembershipRole, MembershipStatus, Permission};
use bee::domain::member::{
    MemberDomainError, MemberDomainService, MemberEvent, Membership,
    MembershipFilter, ToolId, ToolPolicy, ToolRiskLevel, UserEmail,
};
use bee::domain::tenant::value_object::{
    MembershipId, OrganizationId, TeamId, TenantId, UserId,
};

// ============================================================================
// Membership 创建测试
// ============================================================================

#[test]
fn test_membership_invite_creates_pending() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    let membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email.clone(),
        MembershipRole::Member,
    ).unwrap();

    assert_eq!(membership.status(), &MembershipStatus::Pending);
    assert_eq!(membership.role(), &MembershipRole::Member);
    assert_eq!(membership.email(), &email);
    assert!(membership.pending_events().iter().any(|e| matches!(e, MemberEvent::Invited { .. })));
}

#[test]
fn test_membership_invite_with_team() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let team_id = TeamId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    let membership = Membership::invite(
        tenant_id,
        org_id,
        Some(team_id.clone()),
        None,
        email,
        MembershipRole::Member,
    ).unwrap();

    assert!(membership.is_team_level());
    assert!(!membership.is_organization_level());
    assert_eq!(membership.team_id(), Some(&team_id));
}

#[test]
fn test_membership_invite_organization_level() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    let membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::OrgAdmin,
    ).unwrap();

    assert!(membership.is_organization_level());
    assert!(!membership.is_team_level());
}

// ============================================================================
// Membership 状态转换测试
// ============================================================================

#[test]
fn test_membership_accept_invite() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();

    assert_eq!(membership.status(), &MembershipStatus::Pending);

    membership.accept_invite(user_id.clone()).unwrap();

    assert_eq!(membership.status(), &MembershipStatus::Active);
    assert_eq!(membership.user_id(), Some(&user_id));
    assert!(membership.pending_events().iter().any(|e| matches!(e, MemberEvent::InvitationAccepted { .. })));
}

#[test]
fn test_membership_accept_invite_only_pending() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();

    membership.accept_invite(user_id.clone()).unwrap();

    // 再次接受应该失败
    let result = membership.accept_invite(user_id);
    assert!(matches!(result, Err(MemberDomainError::InvalidOperation(_))));
}

#[test]
fn test_membership_suspend() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    assert_eq!(membership.status(), &MembershipStatus::Active);

    membership.suspend("违反规定".to_string()).unwrap();

    assert_eq!(membership.status(), &MembershipStatus::Suspended);
    assert!(membership.pending_events().iter().any(|e| matches!(e, MemberEvent::Suspended { .. })));
}

#[test]
fn test_membership_suspend_only_active() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    // 保持 Pending 状态

    let result = membership.suspend("test".to_string());
    assert!(matches!(result, Err(MemberDomainError::InvalidOperation(_))));
}

#[test]
fn test_membership_remove() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    membership.remove().unwrap();

    assert_eq!(membership.status(), &MembershipStatus::Removed);
    assert!(membership.pending_events().iter().any(|e| matches!(e, MemberEvent::Removed { .. })));
}

#[test]
fn test_membership_remove_idempotent() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();
    membership.remove().unwrap();

    // 再次移除应该失败
    let result = membership.remove();
    assert!(matches!(result, Err(MemberDomainError::InvalidOperation(_))));
}

// ============================================================================
// Membership 角色变更测试
// ============================================================================

#[test]
fn test_membership_change_role() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    assert_eq!(membership.role(), &MembershipRole::Member);

    membership.change_role(MembershipRole::TeamAdmin).unwrap();

    assert_eq!(membership.role(), &MembershipRole::TeamAdmin);
    assert!(membership.pending_events().iter().any(|e| matches!(e, MemberEvent::RoleChanged { .. })));
}

#[test]
fn test_membership_change_role_removed_member() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();
    membership.remove().unwrap();

    let result = membership.change_role(MembershipRole::TeamAdmin);
    assert!(matches!(result, Err(MemberDomainError::InvalidOperation(_))));
}

// ============================================================================
// 工具策略测试
// ============================================================================

#[test]
fn test_membership_add_tool_policy() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    let policy = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::Medium,
        true,
    );

    membership.add_tool_policy(policy).unwrap();

    assert_eq!(membership.tool_policies().len(), 1);
    assert_eq!(membership.tool_policies()[0].tool_id().as_str(), "shell");
    assert_eq!(membership.tool_policies()[0].risk_level(), ToolRiskLevel::Medium);
}

#[test]
fn test_membership_add_tool_policy_replaces_existing() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    let policy1 = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::Low,
        true,
    );
    let policy2 = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::High,
        false,
    );

    membership.add_tool_policy(policy1).unwrap();
    membership.add_tool_policy(policy2).unwrap();

    assert_eq!(membership.tool_policies().len(), 1);
    assert_eq!(membership.tool_policies()[0].risk_level(), ToolRiskLevel::High);
    assert!(!membership.tool_policies()[0].is_allowed());
}

#[test]
fn test_membership_remove_tool_policy() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    let policy = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::Medium,
        true,
    );
    membership.add_tool_policy(policy).unwrap();

    membership.remove_tool_policy(&ToolId::from_str("shell")).unwrap();

    assert_eq!(membership.tool_policies().len(), 0);
}

#[test]
fn test_membership_remove_non_existent_tool_policy() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    let result = membership.remove_tool_policy(&ToolId::from_str("non_existent"));
    assert!(matches!(result, Err(MemberDomainError::InvalidOperation(_))));
}

// ============================================================================
// 工具执行权限测试
// ============================================================================

#[test]
fn test_membership_can_execute_tool_with_policy() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    // 添加策略允许 shell 工具（Medium 风险）
    let policy = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::Medium,
        true,
    );
    membership.add_tool_policy(policy).unwrap();

    // 可以执行低风险工具
    assert!(membership.can_execute_tool(&ToolId::from_str("shell"), ToolRiskLevel::Low));
    // 可以执行同等风险工具
    assert!(membership.can_execute_tool(&ToolId::from_str("shell"), ToolRiskLevel::Medium));
    // 不能执行高风险工具
    assert!(!membership.can_execute_tool(&ToolId::from_str("shell"), ToolRiskLevel::High));
}

#[test]
fn test_membership_can_execute_tool_inactive() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    let membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    // 保持 Pending 状态

    assert!(!membership.can_execute_tool(&ToolId::from_str("shell"), ToolRiskLevel::Low));
}

#[test]
fn test_membership_can_execute_tool_by_role() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();

    // PlatformAdmin 可以执行所有工具
    let mut admin = create_active_membership(&tenant_id, &org_id, MembershipRole::PlatformAdmin);
    assert!(admin.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Critical));

    // OrgAdmin 可以执行 High 及以下
    let mut org_admin = create_active_membership(&tenant_id, &org_id, MembershipRole::OrgAdmin);
    assert!(org_admin.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::High));
    assert!(!org_admin.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Critical));

    // TeamAdmin 可以执行 Medium 及以下
    let mut team_admin = create_active_membership(&tenant_id, &org_id, MembershipRole::TeamAdmin);
    assert!(team_admin.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Medium));
    assert!(!team_admin.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::High));

    // Member 只能执行 Low
    let mut member = create_active_membership(&tenant_id, &org_id, MembershipRole::Member);
    assert!(member.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Low));
    assert!(!member.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Medium));

    // Viewer 不能执行任何工具
    let mut viewer = create_active_membership(&tenant_id, &org_id, MembershipRole::Viewer);
    assert!(!viewer.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Low));
}

fn create_active_membership(
    tenant_id: &TenantId,
    org_id: &OrganizationId,
    role: MembershipRole,
) -> Membership {
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let mut membership = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        None,
        None,
        email,
        role,
    ).unwrap();
    membership.accept_invite(UserId::generate()).unwrap();
    membership
}

// ============================================================================
// 权限检查测试
// ============================================================================

#[test]
fn test_membership_has_permission() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    let user_id = UserId::generate();

    let mut membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    membership.accept_invite(user_id).unwrap();

    // Member 有 AgentRead 权限
    assert!(membership.has_permission(&Permission::AgentRead));
    // Member 有 AgentExecute 权限
    assert!(membership.has_permission(&Permission::AgentExecute));
    // Member 没有 OrgWrite 权限
    assert!(!membership.has_permission(&Permission::OrgWrite));
    // Member 没有 TeamWrite 权限
    assert!(!membership.has_permission(&Permission::TeamWrite));
}

#[test]
fn test_membership_has_permission_inactive() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    let membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    // 保持 Pending 状态

    // Pending 状态成员没有权限
    assert!(!membership.has_permission(&Permission::AgentRead));
}

// ============================================================================
// MemberDomainService 测试
// ============================================================================

#[test]
fn test_member_domain_service_check_permission() {
    let service = MemberDomainService::new();
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let membership = create_active_membership(&tenant_id, &org_id, MembershipRole::Member);

    // Member 有 AgentRead 权限
    let result = service.check_permission(&membership, &Permission::AgentRead);
    assert!(result.is_ok());

    // Member 没有 OrgWrite 权限
    let result = service.check_permission(&membership, &Permission::OrgWrite);
    assert_eq!(result, Err(bee::domain::member::PermissionError::InsufficientRole));
}

#[test]
fn test_member_domain_service_check_permission_inactive() {
    let service = MemberDomainService::new();
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let email = UserEmail::new("test@example.com".to_string()).unwrap();

    let membership = Membership::invite(
        tenant_id,
        org_id,
        None,
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    // 保持 Pending 状态

    let result = service.check_permission(&membership, &Permission::AgentRead);
    assert_eq!(result, Err(bee::domain::member::PermissionError::InactiveMember));
}

#[test]
fn test_member_domain_service_check_permission_tool_execute() {
    let service = MemberDomainService::new();
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();

    let mut membership = create_active_membership(&tenant_id, &org_id, MembershipRole::Member);

    // 添加 shell 工具策略（Medium 风险）
    let policy = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::Medium,
        true,
    );
    membership.add_tool_policy(policy).unwrap();

    // 可以执行 shell 工具
    let result = service.check_permission(&membership, &Permission::ToolExecute("shell".to_string()));
    assert!(result.is_ok());
}

#[test]
fn test_member_domain_service_can_execute_tool() {
    let service = MemberDomainService::new();
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let membership = create_active_membership(&tenant_id, &org_id, MembershipRole::Member);

    let tool_id = ToolId::from_str("shell");

    // 没有策略时，Member 只能执行 Low 风险
    assert!(!service.can_execute_tool(&membership, &tool_id, ToolRiskLevel::Medium));
    assert!(service.can_execute_tool(&membership, &tool_id, ToolRiskLevel::Low));
}

#[test]
fn test_member_domain_service_is_admin() {
    let service = MemberDomainService::new();
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();

    let admin = create_active_membership(&tenant_id, &org_id, MembershipRole::OrgAdmin);
    assert!(service.is_admin(&admin));

    let member = create_active_membership(&tenant_id, &org_id, MembershipRole::Member);
    assert!(!service.is_admin(&member));
}

#[test]
fn test_member_domain_service_is_platform_admin() {
    let service = MemberDomainService::new();
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();

    let platform_admin = create_active_membership(&tenant_id, &org_id, MembershipRole::PlatformAdmin);
    assert!(service.is_platform_admin(&platform_admin));

    let org_admin = create_active_membership(&tenant_id, &org_id, MembershipRole::OrgAdmin);
    assert!(!service.is_platform_admin(&org_admin));
}

#[test]
fn test_member_domain_service_can_manage_member() {
    let service = MemberDomainService::new();

    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let other_org_id = OrganizationId::generate();

    // 创建组织管理员
    let admin = create_active_membership(&tenant_id, &org_id, MembershipRole::OrgAdmin);

    // 创建同组织成员
    let member = create_active_membership(&tenant_id, &org_id, MembershipRole::Member);

    // 创建不同组织的成员
    let other_member = create_active_membership(&tenant_id, &other_org_id, MembershipRole::Member);

    // 组织管理员可以管理同组织成员
    assert!(service.can_manage_member(&admin, &member));

    // 不能管理其他组织的成员
    assert!(!service.can_manage_member(&admin, &other_member));
}

#[test]
fn test_member_domain_service_team_admin_can_manage_member() {
    let service = MemberDomainService::new();

    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let team_id = TeamId::generate();
    let other_team_id = TeamId::generate();

    // 创建团队管理员
    let email = UserEmail::new("team_admin@example.com".to_string()).unwrap();
    let mut team_admin = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        Some(team_id.clone()),
        None,
        email,
        MembershipRole::TeamAdmin,
    ).unwrap();
    team_admin.accept_invite(UserId::generate()).unwrap();

    // 创建同团队成员
    let email = UserEmail::new("team_member@example.com".to_string()).unwrap();
    let mut team_member = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        Some(team_id.clone()),
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    team_member.accept_invite(UserId::generate()).unwrap();

    // 创建不同团队成员
    let email = UserEmail::new("other_team@example.com".to_string()).unwrap();
    let mut other_team_member = Membership::invite(
        tenant_id.clone(),
        org_id.clone(),
        Some(other_team_id),
        None,
        email,
        MembershipRole::Member,
    ).unwrap();
    other_team_member.accept_invite(UserId::generate()).unwrap();

    // TeamAdmin 可以管理同团队成员
    assert!(service.can_manage_member(&team_admin, &team_member));

    // TeamAdmin 不能管理不同团队成员
    assert!(!service.can_manage_member(&team_admin, &other_team_member));
}

// ============================================================================
// UserEmail 值对象测试
// ============================================================================

#[test]
fn test_user_email_validation() {
    // 有效邮箱
    assert!(UserEmail::new("test@example.com".to_string()).is_ok());
    assert!(UserEmail::new("user.name+tag@domain.co.uk".to_string()).is_ok());

    // 空邮箱
    assert!(UserEmail::new("".to_string()).is_err());

    // 没有 @
    assert!(UserEmail::new("testexample.com".to_string()).is_err());

    // 没有域名
    assert!(UserEmail::new("test@".to_string()).is_err());

    // 没有用户名
    assert!(UserEmail::new("@example.com".to_string()).is_err());

    // 没有顶级域名
    assert!(UserEmail::new("test@example".to_string()).is_err());

    // 域名以 . 开头或结尾
    assert!(UserEmail::new("test@.example.com".to_string()).is_err());
    assert!(UserEmail::new("test@example.com.".to_string()).is_err());
}

#[test]
fn test_user_email_lowercase() {
    let email = UserEmail::new("TEST@EXAMPLE.COM".to_string()).unwrap();
    assert_eq!(email.as_str(), "test@example.com");
}

#[test]
fn test_user_email_trim() {
    let email = UserEmail::new("  test@example.com  ".to_string()).unwrap();
    assert_eq!(email.as_str(), "test@example.com");
}

#[test]
fn test_user_email_display() {
    let email = UserEmail::new("test@example.com".to_string()).unwrap();
    assert_eq!(format!("{}", email), "test@example.com");
}

// ============================================================================
// ToolRiskLevel 测试
// ============================================================================

#[test]
fn test_tool_risk_level_ordering() {
    assert!(ToolRiskLevel::Low < ToolRiskLevel::Medium);
    assert!(ToolRiskLevel::Medium < ToolRiskLevel::High);
    assert!(ToolRiskLevel::High < ToolRiskLevel::Critical);
}

#[test]
fn test_tool_risk_level_from_str() {
    assert_eq!(ToolRiskLevel::from_str("low").unwrap(), ToolRiskLevel::Low);
    assert_eq!(ToolRiskLevel::from_str("medium").unwrap(), ToolRiskLevel::Medium);
    assert_eq!(ToolRiskLevel::from_str("high").unwrap(), ToolRiskLevel::High);
    assert_eq!(ToolRiskLevel::from_str("critical").unwrap(), ToolRiskLevel::Critical);
    assert!(ToolRiskLevel::from_str("invalid").is_err());
}

#[test]
fn test_tool_risk_level_is_at_most() {
    assert!(ToolRiskLevel::Low.is_at_most(ToolRiskLevel::Low));
    assert!(ToolRiskLevel::Low.is_at_most(ToolRiskLevel::Medium));
    assert!(ToolRiskLevel::Medium.is_at_most(ToolRiskLevel::High));
    assert!(!ToolRiskLevel::High.is_at_most(ToolRiskLevel::Medium));
    assert!(!ToolRiskLevel::Critical.is_at_most(ToolRiskLevel::Low));
}

// ============================================================================
// ToolPolicy 测试
// ============================================================================

#[test]
fn test_tool_policy_can_execute() {
    let policy = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::Medium,
        true,
    );

    // 可以执行低风险工具
    assert!(policy.can_execute(ToolRiskLevel::Low));
    // 可以执行同等风险工具
    assert!(policy.can_execute(ToolRiskLevel::Medium));
    // 不能执行高风险工具
    assert!(!policy.can_execute(ToolRiskLevel::High));
}

#[test]
fn test_tool_policy_not_allowed() {
    let policy = ToolPolicy::new(
        ToolId::from_str("dangerous_tool"),
        ToolRiskLevel::Critical,
        false,
    );

    // 即使风险等级足够，但不允许执行
    assert!(!policy.can_execute(ToolRiskLevel::Low));
}

#[test]
fn test_tool_policy_with_note() {
    let policy = ToolPolicy::new(
        ToolId::from_str("shell"),
        ToolRiskLevel::High,
        true,
    ).with_note("仅管理员可用".to_string());

    assert_eq!(policy.note(), Some("仅管理员可用"));
}

// ============================================================================
// MembershipFilter 测试
// ============================================================================

#[test]
fn test_membership_filter_builder() {
    let tenant_id = TenantId::generate();
    let org_id = OrganizationId::generate();
    let user_id = UserId::generate();

    let filter = MembershipFilter::new()
        .with_tenant_id(tenant_id.clone())
        .with_organization_id(org_id.clone())
        .with_user_id(user_id.clone())
        .with_role(MembershipRole::OrgAdmin)
        .with_status(MembershipStatus::Active);

    assert_eq!(filter.tenant_id, Some(tenant_id));
    assert_eq!(filter.organization_id, Some(org_id));
    assert_eq!(filter.user_id, Some(user_id));
    assert_eq!(filter.role, Some(MembershipRole::OrgAdmin));
    assert_eq!(filter.status, Some(MembershipStatus::Active));
}

#[test]
fn test_membership_filter_is_empty() {
    let empty_filter = MembershipFilter::new();
    assert!(empty_filter.is_empty());

    let filter = MembershipFilter::new().with_tenant_id(TenantId::generate());
    assert!(!filter.is_empty());
}
