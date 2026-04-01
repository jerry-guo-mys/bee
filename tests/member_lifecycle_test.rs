//! 成员生命周期集成测试
//!
//! 测试成员从邀请到移除的完整生命周期

use std::sync::Arc;

use bee::application::commands::{
    AcceptInviteCommand, AcceptInviteHandler, CommandHandler,
    SuspendMemberCommand, SuspendMemberHandler,
};
use bee::application::queries::{
    ListMembersQuery, ListMembersHandler, QueryHandler,
};
use bee::domain::common::{MembershipRole, MembershipStatus};
use bee::domain::event::InMemoryEventPublisher;
use bee::domain::member::{
    InMemoryMembershipRepository, MemberDomainService, MembershipRepository,
};
use bee::domain::member::value_object::UserEmail;
use bee::domain::tenant::{OrganizationId, TenantId, UserId};

/// 创建测试用的服务组合
fn create_test_services() -> (
    Arc<MemberDomainService<InMemoryMembershipRepository, InMemoryEventPublisher>>,
    Arc<InMemoryMembershipRepository>,
    Arc<InMemoryEventPublisher>,
) {
    let repo = Arc::new(InMemoryMembershipRepository::new());
    let publisher = Arc::new(InMemoryEventPublisher::new());
    let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
    (service, repo, publisher)
}

/// 创建测试成员
async fn create_test_member(
    service: &MemberDomainService<InMemoryMembershipRepository, InMemoryEventPublisher>,
    email: &str,
    role: MembershipRole,
) -> bee::domain::member::Membership {
    let user_email = UserEmail::new(email.to_string()).unwrap();
    service
        .invite_member(
            TenantId::generate(),
            OrganizationId::generate(),
            None,
            user_email,
            role,
            UserId::generate(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn test_member_lifecycle_invite_and_accept() {
    let (member_service, _repo, _publisher) = create_test_services();

    // 邀请成员
    let membership = create_test_member(&member_service, "test@example.com", MembershipRole::Member).await;

    // 验证初始状态为 Pending
    assert_eq!(membership.status(), &MembershipStatus::Pending);

    // 接受邀请
    let user_id = UserId::generate();
    member_service
        .accept_invite(membership.id(), user_id.clone())
        .await
        .unwrap();

    // 验证状态变为 Active
    let updated = member_service
        .get_member_by_id(membership.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.status(), &MembershipStatus::Active);
    assert_eq!(updated.user_id(), Some(&user_id));
}

#[tokio::test]
async fn test_member_lifecycle_suspend() {
    let (member_service, _repo, _publisher) = create_test_services();

    // 邀请并接受
    let membership = create_test_member(&member_service, "suspend@example.com", MembershipRole::Member).await;
    member_service
        .accept_invite(membership.id(), UserId::generate())
        .await
        .unwrap();

    // 暂停成员
    member_service
        .suspend_member(membership.id(), "违反规定")
        .await
        .unwrap();

    // 验证状态变为 Suspended
    let suspended = member_service
        .get_member_by_id(membership.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(suspended.status(), &MembershipStatus::Suspended);
}

#[tokio::test]
async fn test_member_lifecycle_remove() {
    let (member_service, _repo, _publisher) = create_test_services();

    // 邀请并接受
    let membership = create_test_member(&member_service, "remove@example.com", MembershipRole::Member).await;
    member_service
        .accept_invite(membership.id(), UserId::generate())
        .await
        .unwrap();

    // 移除成员
    member_service
        .remove_member(membership.id())
        .await
        .unwrap();

    // 验证状态变为 Removed
    let removed = member_service
        .get_member_by_id(membership.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(removed.status(), &MembershipStatus::Removed);
}

#[tokio::test]
async fn test_member_lifecycle_role_change() {
    let (member_service, _repo, _publisher) = create_test_services();

    // 邀请并接受（初始为 Member）
    let membership = create_test_member(&member_service, "rolechange@example.com", MembershipRole::Member).await;
    member_service
        .accept_invite(membership.id(), UserId::generate())
        .await
        .unwrap();

    // 变更角色为 TeamAdmin
    member_service
        .change_role(membership.id(), MembershipRole::TeamAdmin)
        .await
        .unwrap();

    // 验证角色已变更
    let updated = member_service
        .get_member_by_id(membership.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.role(), &MembershipRole::TeamAdmin);
}

#[tokio::test]
async fn test_member_lifecycle_list_members() {
    let (member_service, _repo, _publisher) = create_test_services();

    // 创建查询处理器
    let query_handler = ListMembersHandler::new(member_service.clone());

    // 创建 3 个成员
    let org_id = OrganizationId::generate();
    for i in 0..3 {
        let membership = member_service
            .invite_member(
                TenantId::generate(),
                org_id.clone(),
                None,
                UserEmail::new(format!("member{}@example.com", i)).unwrap(),
                MembershipRole::Member,
                UserId::generate(),
            )
            .await
            .unwrap();

        // 接受前两个成员的邀请
        if i < 2 {
            member_service
                .accept_invite(membership.id(), UserId::generate())
                .await
                .unwrap();
        }
    }

    // 查询所有成员
    let query = ListMembersQuery {
        tenant_id: TenantId::generate(),
        organization_id: org_id.clone(),
        team_id: None,
        status: None,
        limit: 10,
        offset: 0,
    };

    let result = query_handler.handle(query).await;
    assert!(result.is_ok());
    let members = result.unwrap();
    assert_eq!(members.len(), 3);

    // 只查询 Active 状态的成员
    let query = ListMembersQuery {
        tenant_id: TenantId::generate(),
        organization_id: org_id.clone(),
        team_id: None,
        status: Some(MembershipStatus::Active),
        limit: 10,
        offset: 0,
    };

    let result = query_handler.handle(query).await;
    assert!(result.is_ok());
    let members = result.unwrap();
    assert_eq!(members.len(), 2); // 只有 2 个 Active 成员
}

#[tokio::test]
async fn test_member_lifecycle_command_handlers() {
    let (member_service, _repo, _publisher) = create_test_services();

    // 创建命令处理器
    let accept_handler = AcceptInviteHandler::new(member_service.clone());
    let suspend_handler = SuspendMemberHandler::new(member_service.clone());

    // 邀请成员
    let membership = create_test_member(&member_service, "command@example.com", MembershipRole::Member).await;

    // 使用命令处理器接受邀请
    let accept_command = AcceptInviteCommand {
        membership_id: membership.id().clone(),
        user_id: UserId::generate(),
    };
    let result = accept_handler.handle(accept_command).await;
    assert!(result.is_ok());

    // 验证状态变为 Active
    let updated = member_service
        .get_member_by_id(membership.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.status(), &MembershipStatus::Active);

    // 使用命令处理器暂停成员
    let suspend_command = SuspendMemberCommand {
        membership_id: membership.id().clone(),
        reason: "Test suspension".to_string(),
        operator_id: UserId::generate(),
    };
    let result = suspend_handler.handle(suspend_command).await;
    assert!(result.is_ok());

    // 验证状态变为 Suspended
    let suspended = member_service
        .get_member_by_id(membership.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(suspended.status(), &MembershipStatus::Suspended);
}

#[tokio::test]
async fn test_member_lifecycle_cannot_accept_non_pending_invite() {
    let (member_service, _repo, _publisher) = create_test_services();
    let accept_handler = AcceptInviteHandler::new(member_service.clone());

    // 邀请并接受
    let membership = create_test_member(&member_service, "nonpending@example.com", MembershipRole::Member).await;
    member_service
        .accept_invite(membership.id(), UserId::generate())
        .await
        .unwrap();

    // 再次接受应该失败
    let accept_command = AcceptInviteCommand {
        membership_id: membership.id().clone(),
        user_id: UserId::generate(),
    };
    let result = accept_handler.handle(accept_command).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("只有待处理的邀请才能被接受"));
}

#[tokio::test]
async fn test_member_lifecycle_cannot_suspend_non_active_member() {
    let (member_service, _repo, _publisher) = create_test_services();
    let suspend_handler = SuspendMemberHandler::new(member_service.clone());

    // 只邀请不接受（Pending 状态）
    let membership = create_test_member(&member_service, "nonactive@example.com", MembershipRole::Member).await;

    // 暂停 Pending 状态的成员应该失败
    let suspend_command = SuspendMemberCommand {
        membership_id: membership.id().clone(),
        reason: "Test".to_string(),
        operator_id: UserId::generate(),
    };
    let result = suspend_handler.handle(suspend_command).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_member_lifecycle_events_published() {
    let (member_service, _repo, publisher) = create_test_services();

    // 邀请成员
    let membership = create_test_member(&member_service, "events@example.com", MembershipRole::Member).await;

    // 验证 Invited 事件已发布
    let events = publisher.get_events().await;
    assert!(events.iter().any(|e| e.event_type() == "member.invited"));

    // 接受邀请
    member_service
        .accept_invite(membership.id(), UserId::generate())
        .await
        .unwrap();

    // 验证 InvitationAccepted 事件已发布
    let events = publisher.get_events().await;
    assert!(events.iter().any(|e| e.event_type() == "member.invitation_accepted"));

    // 暂停成员
    member_service
        .suspend_member(membership.id(), "Test reason")
        .await
        .unwrap();

    // 验证 Suspended 事件已发布
    let events = publisher.get_events().await;
    assert!(events.iter().any(|e| e.event_type() == "member.suspended"));
}
