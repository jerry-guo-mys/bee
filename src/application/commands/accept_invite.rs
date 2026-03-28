//! 接受邀请命令

use super::handler::{CommandHandler, CqrsCommand};
use crate::domain::service::MemberDomainService;
use crate::domain::tenant::{MembershipId, UserId};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 接受邀请命令
#[derive(Debug, Clone)]
pub struct AcceptInviteCommand {
    pub membership_id: MembershipId,
    pub user_id: UserId,
}

impl CqrsCommand for AcceptInviteCommand {
    type Response = ();
}

/// 接受邀请命令处理器
pub struct AcceptInviteHandler<MR, EP> {
    member_service: Arc<MemberDomainService<MR, EP>>,
}

impl<MR, EP> AcceptInviteHandler<MR, EP>
where
    MR: crate::domain::member::MembershipRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    pub fn new(member_service: Arc<MemberDomainService<MR, EP>>) -> Self {
        Self { member_service }
    }
}

#[async_trait]
impl<MR, EP> CommandHandler<AcceptInviteCommand> for AcceptInviteHandler<MR, EP>
where
    MR: crate::domain::member::MembershipRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        command: AcceptInviteCommand,
    ) -> Result<<AcceptInviteCommand as CqrsCommand>::Response, Self::Error> {
        // 记录审计日志
        tracing::info!(
            target: "audit",
            event = "invite_accepted",
            user_id = %command.user_id,
            membership_id = %command.membership_id,
            "Invite accepted by {}",
            command.user_id
        );

        // 调用领域服务接受邀请
        self.member_service
            .accept_invite(&command.membership_id, command.user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to accept invite: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::InMemoryEventPublisher;
    use crate::domain::member::InMemoryMembershipRepository;

    #[tokio::test]
    async fn test_accept_invite_handler_success() {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
        let handler = AcceptInviteHandler::new(service.clone());

        // 先创建一个邀请
        let email = crate::domain::member::value_object::UserEmail::new("test@example.com".to_string()).unwrap();
        let membership = service
            .invite_member(
                crate::domain::tenant::TenantId::generate(),
                crate::domain::tenant::OrganizationId::generate(),
                None,
                email,
                crate::domain::common::MembershipRole::Member,
                UserId::generate(),
            )
            .await
            .unwrap();

        // 接受邀请
        let command = AcceptInviteCommand {
            membership_id: membership.id().clone(),
            user_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_ok());

        // 验证成员状态已变为 Active
        let updated = service.get_member_by_id(membership.id()).await.unwrap().unwrap();
        assert_eq!(updated.status(), &crate::domain::common::MembershipStatus::Active);
    }

    #[tokio::test]
    async fn test_accept_invite_handler_not_found() {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
        let handler = AcceptInviteHandler::new(service.clone());

        let command = AcceptInviteCommand {
            membership_id: MembershipId::generate(),
            user_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_accept_invite_handler_already_accepted() {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
        let handler = AcceptInviteHandler::new(service.clone());

        // 先创建一个邀请并接受
        let email = crate::domain::member::value_object::UserEmail::new("test@example.com".to_string()).unwrap();
        let membership = service
            .invite_member(
                crate::domain::tenant::TenantId::generate(),
                crate::domain::tenant::OrganizationId::generate(),
                None,
                email,
                crate::domain::common::MembershipRole::Member,
                UserId::generate(),
            )
            .await
            .unwrap();

        // 第一次接受
        service
            .accept_invite(membership.id(), UserId::generate())
            .await
            .unwrap();

        // 再次接受应该失败
        let command = AcceptInviteCommand {
            membership_id: membership.id().clone(),
            user_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("只有待处理的邀请才能被接受"));
    }
}
