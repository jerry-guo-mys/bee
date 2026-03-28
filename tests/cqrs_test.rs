use bee::application::commands::{
    invite_member::{InviteMemberCommand, InviteMemberHandler},
    CqrsCommand, CommandHandler,
};
use bee::application::queries::{
    list_members::{ListMembersQuery, ListMembersHandler},
    CqrsQuery, QueryHandler,
};
use bee::domain::common::MembershipRole;
use bee::domain::tenant::{TenantId, OrganizationId, UserId};

#[tokio::test]
async fn test_invite_member_command() {
    let handler = InviteMemberHandler::new();

    let command = InviteMemberCommand {
        tenant_id: TenantId::new("tenant-1".to_string()),
        organization_id: OrganizationId::new("org-1".to_string()),
        team_id: None,
        user_id: UserId::new("user-1".to_string()),
        email: "test@example.com".to_string(),
        role: MembershipRole::Member,
        inviter_id: UserId::new("admin-1".to_string()),
    };

    let result = handler.handle(command).await;
    assert!(result.is_ok(), "Should create membership");
}

#[tokio::test]
async fn test_invite_member_invalid_email() {
    let handler = InviteMemberHandler::new();

    let command = InviteMemberCommand {
        tenant_id: TenantId::new("tenant-1".to_string()),
        organization_id: OrganizationId::new("org-1".to_string()),
        team_id: None,
        user_id: UserId::new("user-1".to_string()),
        email: "not-an-email".to_string(),
        role: MembershipRole::Member,
        inviter_id: UserId::new("admin-1".to_string()),
    };

    let result = handler.handle(command).await;
    assert!(result.is_err(), "Should fail with invalid email");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid email"), "Error should mention invalid email");
}

#[tokio::test]
async fn test_invite_member_empty_email() {
    let handler = InviteMemberHandler::new();

    let command = InviteMemberCommand {
        tenant_id: TenantId::new("tenant-1".to_string()),
        organization_id: OrganizationId::new("org-1".to_string()),
        team_id: None,
        user_id: UserId::new("user-1".to_string()),
        email: "".to_string(),
        role: MembershipRole::Member,
        inviter_id: UserId::new("admin-1".to_string()),
    };

    let result = handler.handle(command).await;
    assert!(result.is_err(), "Should fail with empty email");
}

#[tokio::test]
async fn test_list_members_query() {
    let handler = ListMembersHandler::new();

    let query = ListMembersQuery {
        tenant_id: TenantId::new("tenant-1".to_string()),
        organization_id: OrganizationId::new("org-1".to_string()),
        team_id: None,
        status: None,
        limit: 10,
        offset: 0,
    };

    let result = handler.handle(query).await;
    assert!(result.is_ok(), "Should return empty list");
    assert!(result.unwrap().is_empty());
}
