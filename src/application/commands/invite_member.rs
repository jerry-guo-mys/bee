use super::handler::{CqrsCommand, CommandHandler};
use crate::domain::common::MembershipRole;
use crate::domain::member::Membership;
use crate::domain::member::value_object::UserEmail;
use crate::domain::tenant::{OrganizationId, TeamId, TenantId, UserId};
use async_trait::async_trait;
use anyhow::Result;

/// 邀请成员命令
#[derive(Debug, Clone)]
pub struct InviteMemberCommand {
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub team_id: Option<TeamId>,
    pub user_id: UserId,
    pub email: String,
    pub role: MembershipRole,
    pub inviter_id: UserId,
}

impl CqrsCommand for InviteMemberCommand {
    type Response = Membership;
}

/// 邀请成员命令处理器
pub struct InviteMemberHandler;

impl InviteMemberHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandHandler<InviteMemberCommand> for InviteMemberHandler {
    type Error = anyhow::Error;

    async fn handle(
        &self,
        command: InviteMemberCommand,
    ) -> Result<<InviteMemberCommand as CqrsCommand>::Response, Self::Error> {
        // 创建成员关系
        let email = UserEmail::new(command.email)
            .map_err(|e| anyhow::anyhow!("Invalid email: {}", e))?;

        let membership = Membership::invite(
            command.tenant_id,
            command.organization_id,
            command.team_id,
            Some(command.user_id),
            email,
            command.role,
        )?;

        // TODO: 保存到数据库
        // TODO: 发送领域事件

        Ok(membership)
    }
}
