//! 获取租户查询

use super::handler::{CqrsQuery, QueryHandler};
use crate::domain::service::TenantDomainService;
use crate::domain::tenant::{Tenant, TenantId};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 获取租户查询
#[derive(Debug, Clone)]
pub struct GetTenantQuery {
    pub tenant_id: TenantId,
}

impl CqrsQuery for GetTenantQuery {
    type Response = Option<Tenant>;
}

/// 获取租户查询处理器
pub struct GetTenantHandler<TR, EP> {
    tenant_service: Arc<TenantDomainService<TR, EP>>,
}

impl<TR, EP> GetTenantHandler<TR, EP>
where
    TR: crate::domain::tenant::TenantRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    pub fn new(tenant_service: Arc<TenantDomainService<TR, EP>>) -> Self {
        Self { tenant_service }
    }
}

#[async_trait]
impl<TR, EP> QueryHandler<GetTenantQuery> for GetTenantHandler<TR, EP>
where
    TR: crate::domain::tenant::TenantRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        query: GetTenantQuery,
    ) -> Result<<GetTenantQuery as CqrsQuery>::Response, Self::Error> {
        // 记录审计日志
        tracing::info!(
            target: "audit",
            event = "tenant_queried",
            tenant_id = %query.tenant_id,
            "Tenant queried"
        );

        // 调用领域服务获取租户
        let tenant = self
            .tenant_service
            .get_tenant_by_id(&query.tenant_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get tenant: {}", e))?;

        Ok(tenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::InMemoryEventPublisher;
    use crate::domain::tenant::InMemoryTenantRepository;

    #[tokio::test]
    async fn test_get_tenant_handler_success() {
        let repo = Arc::new(InMemoryTenantRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(TenantDomainService::new(repo.clone(), publisher.clone()));
        let handler = GetTenantHandler::new(service.clone());

        // 先创建租户
        let tenant = service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();

        // 查询租户
        let query = GetTenantQuery {
            tenant_id: tenant.id().clone(),
        };

        let result = handler.handle(query).await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name().as_str(), "Test Tenant");
    }

    #[tokio::test]
    async fn test_get_tenant_handler_not_found() {
        let repo = Arc::new(InMemoryTenantRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(TenantDomainService::new(repo.clone(), publisher.clone()));
        let handler = GetTenantHandler::new(service);

        let query = GetTenantQuery {
            tenant_id: TenantId::generate(),
        };

        let result = handler.handle(query).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
