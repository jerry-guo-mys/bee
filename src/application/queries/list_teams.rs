//! 列出团队查询

use super::handler::{CqrsQuery, QueryHandler};
use crate::domain::team::Team;
use crate::domain::tenant::OrganizationId;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 列出团队查询
#[derive(Debug, Clone)]
pub struct ListTeamsQuery {
    pub tenant_id: String,
    pub organization_id: String,
}

impl CqrsQuery for ListTeamsQuery {
    type Response = Vec<Team>;
}

/// 列出团队查询处理器
pub struct ListTeamsHandler<TR> {
    team_repo: Arc<TR>,
}

impl<TR> ListTeamsHandler<TR>
where
    TR: crate::domain::team::TeamRepository<Error = crate::domain::team::TeamError> + 'static,
{
    pub fn new(team_repo: Arc<TR>) -> Self {
        Self { team_repo }
    }
}

#[async_trait]
impl<TR> QueryHandler<ListTeamsQuery> for ListTeamsHandler<TR>
where
    TR: crate::domain::team::TeamRepository<Error = crate::domain::team::TeamError> + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        query: ListTeamsQuery,
    ) -> Result<<ListTeamsQuery as CqrsQuery>::Response, Self::Error> {
        let organization_id = OrganizationId::new(query.organization_id);

        let teams = self
            .team_repo
            .find_by_organization(&organization_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list teams: {}", e))?;

        Ok(teams)
    }
}
