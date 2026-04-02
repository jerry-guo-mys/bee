//! 创建组织命令

use super::handler::{CommandHandler, CqrsCommand};
use crate::domain::organization::Organization;
use crate::domain::service::OrganizationDomainService;
use crate::domain::tenant::UserId;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 创建组织命令
#[derive(Debug, Clone)]
pub struct CreateOrganizationCommand {
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub creator_id: UserId,
}

impl CqrsCommand for CreateOrganizationCommand {
    type Response = Organization;
}

/// 创建组织命令处理器
pub struct CreateOrganizationHandler<OR, EP> {
    org_service: Arc<OrganizationDomainService<OR, EP>>,
}

impl<OR, EP> CreateOrganizationHandler<OR, EP>
where
    OR: crate::domain::organization::OrganizationRepository<
            Error = crate::domain::organization::OrganizationError,
        > + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    pub fn new(org_service: Arc<OrganizationDomainService<OR, EP>>) -> Self {
        Self { org_service }
    }
}

#[async_trait]
impl<OR, EP> CommandHandler<CreateOrganizationCommand> for CreateOrganizationHandler<OR, EP>
where
    OR: crate::domain::organization::OrganizationRepository<
            Error = crate::domain::organization::OrganizationError,
        > + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        command: CreateOrganizationCommand,
    ) -> Result<<CreateOrganizationCommand as CqrsCommand>::Response, Self::Error> {
        let tenant_id = crate::domain::tenant::TenantId::new(command.tenant_id);

        tracing::info!(
            target: "audit",
            event = "organization_created",
            creator_id = %command.creator_id,
            tenant_id = %tenant_id,
            organization_name = %command.name,
            organization_slug = %command.slug,
            "Organization created by {}",
            command.creator_id
        );

        let org = self
            .org_service
            .create_organization(tenant_id, command.name, command.slug)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create organization: {}", e))?;

        Ok(org)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::InMemoryEventPublisher;
    use crate::domain::organization::{InMemoryOrganizationRepository, OrganizationDomainService};

    #[tokio::test]
    async fn test_create_organization_handler_success() {
        let repo = Arc::new(InMemoryOrganizationRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(OrganizationDomainService::new(
            repo.clone(),
            publisher.clone(),
        ));
        let handler = CreateOrganizationHandler::new(service);

        let command = CreateOrganizationCommand {
            tenant_id: "tenant-1".to_string(),
            name: "Test Org".to_string(),
            slug: "test-org".to_string(),
            creator_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_ok());

        let org = result.unwrap();
        assert_eq!(org.name().as_str(), "Test Org");
        assert_eq!(org.slug().as_str(), "test-org");
    }
}
