//! 创建租户命令

use super::handler::{CommandHandler, CqrsCommand};
use crate::domain::service::TenantDomainService;
use crate::domain::tenant::{Tenant, UserId};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 创建租户命令
#[derive(Debug, Clone)]
pub struct CreateTenantCommand {
    pub name: String,
    pub slug: String,
    pub creator_id: UserId,
}

impl CqrsCommand for CreateTenantCommand {
    type Response = Tenant;
}

/// 创建租户命令处理器
pub struct CreateTenantHandler<TR, EP> {
    tenant_service: Arc<TenantDomainService<TR, EP>>,
}

impl<TR, EP> CreateTenantHandler<TR, EP>
where
    TR: crate::domain::tenant::TenantRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    pub fn new(tenant_service: Arc<TenantDomainService<TR, EP>>) -> Self {
        Self { tenant_service }
    }
}

#[async_trait]
impl<TR, EP> CommandHandler<CreateTenantCommand> for CreateTenantHandler<TR, EP>
where
    TR: crate::domain::tenant::TenantRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        command: CreateTenantCommand,
    ) -> Result<<CreateTenantCommand as CqrsCommand>::Response, Self::Error> {
        // 记录审计日志
        tracing::info!(
            target: "audit",
            event = "tenant_created",
            creator_id = %command.creator_id,
            tenant_name = %command.name,
            tenant_slug = %command.slug,
            "Tenant created by {}",
            command.creator_id
        );

        // 调用领域服务创建租户
        let tenant = self
            .tenant_service
            .create_tenant(command.name, command.slug)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create tenant: {}", e))?;

        Ok(tenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::InMemoryEventPublisher;
    use crate::domain::tenant::InMemoryTenantRepository;

    #[tokio::test]
    async fn test_create_tenant_handler_success() {
        let repo = Arc::new(InMemoryTenantRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(TenantDomainService::new(repo.clone(), publisher.clone()));
        let handler = CreateTenantHandler::new(service);

        let command = CreateTenantCommand {
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            creator_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_ok());

        let tenant = result.unwrap();
        assert_eq!(tenant.name().as_str(), "Test Tenant");
        assert_eq!(tenant.slug().as_str(), "test-tenant");
    }

    #[tokio::test]
    async fn test_create_tenant_handler_duplicate_slug() {
        let repo = Arc::new(InMemoryTenantRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(TenantDomainService::new(repo.clone(), publisher.clone()));
        let handler = CreateTenantHandler::new(service);

        // 创建第一个租户
        let command1 = CreateTenantCommand {
            name: "Tenant 1".to_string(),
            slug: "test-slug".to_string(),
            creator_id: UserId::generate(),
        };
        let result1 = handler.handle(command1).await;
        assert!(result1.is_ok());

        // 尝试创建重复 slug 的租户
        let command2 = CreateTenantCommand {
            name: "Tenant 2".to_string(),
            slug: "test-slug".to_string(),
            creator_id: UserId::generate(),
        };
        let result2 = handler.handle(command2).await;
        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_tenant_handler_invalid_name() {
        let repo = Arc::new(InMemoryTenantRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(TenantDomainService::new(repo.clone(), publisher.clone()));
        let handler = CreateTenantHandler::new(service);

        let command = CreateTenantCommand {
            name: "".to_string(),
            slug: "test-tenant".to_string(),
            creator_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("无效"));
    }

    #[tokio::test]
    async fn test_create_tenant_handler_invalid_slug() {
        let repo = Arc::new(InMemoryTenantRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(TenantDomainService::new(repo.clone(), publisher.clone()));
        let handler = CreateTenantHandler::new(service);

        let command = CreateTenantCommand {
            name: "Test Tenant".to_string(),
            slug: "INVALID_SLUG".to_string(),
            creator_id: UserId::generate(),
        };

        let result = handler.handle(command).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("无效"));
    }
}
