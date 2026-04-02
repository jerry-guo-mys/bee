//! 暂停成员命令

use super::handler::{CommandHandler, CqrsCommand};
use crate::domain::service::MemberDomainService;
use crate::domain::tenant::{MembershipId, UserId};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 暂停成员命令
#[derive(Debug, Clone)]
pub struct SuspendMemberCommand {
    pub membership_id: MembershipId,
    pub reason: String,
    pub operator_id: UserId,
}

impl CqrsCommand for SuspendMemberCommand {
    type Response = ();
}

/// 暂停成员命令处理器
pub struct SuspendMemberHandler<MR, EP> {
    member_service: Arc<MemberDomainService<MR, EP>>,
}

impl<MR, EP> SuspendMemberHandler<MR, EP>
where
    MR: crate::domain::member::MembershipRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    pub fn new(member_service: Arc<MemberDomainService<MR, EP>>) -> Self {
        Self { member_service }
    }
}

#[async_trait]
impl<MR, EP> CommandHandler<SuspendMemberCommand> for SuspendMemberHandler<MR, EP>
where
    MR: crate::domain::member::MembershipRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        command: SuspendMemberCommand,
    ) -> Result<<SuspendMemberCommand as CqrsCommand>::Response, Self::Error> {
        // 记录审计日志
        tracing::info!(
            target: "audit",
            event = "member_suspended",
            operator_id = %command.operator_id,
            membership_id = %command.membership_id,
            reason = %command.reason,
            "Member suspended by {}",
            command.operator_id
        );

        // 调用领域服务暂停成员
        self.member_service
            .suspend_member(&command.membership_id, &command.reason)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to suspend member: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::InMemoryEventPublisher;
    use crate::domain::member::InMemoryMembershipRepository;
    use crate::domain::tenant::UserId;

    #[tokio::test]
    async fn test_suspend_member_handler_success() {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
        let handler = SuspendMemberHandler::new(service.clone());

        // 先创建一个成员
        let email =
            crate::domain::member::value_object::UserEmail::new("test@example.com".to_string())
                .unwrap();
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
        service
            .accept_invite(membership.id(), UserId::generate())
            .await
            .unwrap();

        // 暂停成员
        let command = SuspendMemberCommand {
            membership_id: membership.id().clone(),
            reason: "违反规定".to_string(),
            operator_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_ok());

        // 验证成员已被暂停
        let updated = service
            .get_member_by_id(membership.id())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated.status(),
            &crate::domain::common::MembershipStatus::Suspended
        );
    }

    #[tokio::test]
    async fn test_suspend_member_handler_not_found() {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
        let handler = SuspendMemberHandler::new(service);

        let command = SuspendMemberCommand {
            membership_id: MembershipId::generate(),
            reason: "Test reason".to_string(),
            operator_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
