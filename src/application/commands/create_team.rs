//! 创建团队命令

use super::handler::{CommandHandler, CqrsCommand};
use crate::domain::service::TeamDomainService;
use crate::domain::team::Team;
use crate::domain::tenant::UserId;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 创建团队命令
#[derive(Debug, Clone)]
pub struct CreateTeamCommand {
    pub tenant_id: String,
    pub organization_id: String,
    pub name: String,
    pub code: Option<String>,
    pub parent_team_id: Option<String>,
    pub creator_id: UserId,
}

impl CqrsCommand for CreateTeamCommand {
    type Response = Team;
}

/// 创建团队命令处理器
pub struct CreateTeamHandler<TR, EP> {
    team_service: Arc<TeamDomainService<TR, EP>>,
}

impl<TR, EP> CreateTeamHandler<TR, EP>
where
    TR: crate::domain::team::TeamRepository<Error = crate::domain::team::TeamError> + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    pub fn new(team_service: Arc<TeamDomainService<TR, EP>>) -> Self {
        Self { team_service }
    }
}

#[async_trait]
impl<TR, EP> CommandHandler<CreateTeamCommand> for CreateTeamHandler<TR, EP>
where
    TR: crate::domain::team::TeamRepository<Error = crate::domain::team::TeamError> + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        command: CreateTeamCommand,
    ) -> Result<<CreateTeamCommand as CqrsCommand>::Response, Self::Error> {
        let tenant_id = crate::domain::tenant::TenantId::new(command.tenant_id);
        let organization_id = crate::domain::tenant::OrganizationId::new(command.organization_id);
        let parent_team_id = command
            .parent_team_id
            .map(|id| crate::domain::team::TeamId::new(id));

        tracing::info!(
            target: "audit",
            event = "team_created",
            creator_id = %command.creator_id,
            tenant_id = %tenant_id,
            organization_id = %organization_id,
            team_name = %command.name,
            "Team created by {}",
            command.creator_id
        );

        let team = self
            .team_service
            .create_team(
                tenant_id,
                organization_id,
                command.name,
                command.code,
                None,
                parent_team_id,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create team: {}", e))?;

        Ok(team)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::InMemoryEventPublisher;
    use crate::domain::team::{InMemoryTeamRepository, TeamDomainService};

    #[tokio::test]
    async fn test_create_team_handler_success() {
        let repo = Arc::new(InMemoryTeamRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(TeamDomainService::new(repo.clone(), publisher.clone()));
        let handler = CreateTeamHandler::new(service);

        let command = CreateTeamCommand {
            tenant_id: "tenant-1".to_string(),
            organization_id: "org-1".to_string(),
            name: "Test Team".to_string(),
            code: Some("TEST".to_string()),
            parent_team_id: None,
            creator_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_ok());

        let team = result.unwrap();
        assert_eq!(team.name().as_str(), "Test Team");
    }
}
