//! 获取组织查询

use super::handler::{CqrsQuery, QueryHandler};
use crate::domain::organization::Organization;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 获取组织查询
#[derive(Debug, Clone)]
pub struct GetOrganizationQuery {
    pub tenant_id: String,
    pub organization_id: String,
}

impl CqrsQuery for GetOrganizationQuery {
    type Response = Option<Organization>;
}

/// 获取组织查询处理器
pub struct GetOrganizationHandler<OR> {
    org_repo: Arc<OR>,
}

impl<OR> GetOrganizationHandler<OR>
where
    OR: crate::domain::organization::OrganizationRepository<
            Error = crate::domain::organization::OrganizationError,
        > + 'static,
{
    pub fn new(org_repo: Arc<OR>) -> Self {
        Self { org_repo }
    }
}

#[async_trait]
impl<OR> QueryHandler<GetOrganizationQuery> for GetOrganizationHandler<OR>
where
    OR: crate::domain::organization::OrganizationRepository<
            Error = crate::domain::organization::OrganizationError,
        > + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        query: GetOrganizationQuery,
    ) -> Result<<GetOrganizationQuery as CqrsQuery>::Response, Self::Error> {
        let organization_id =
            crate::domain::organization::OrganizationId::new(query.organization_id);

        let org = self
            .org_repo
            .find_by_id(&organization_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get organization: {}", e))?;

        Ok(org)
    }
}
