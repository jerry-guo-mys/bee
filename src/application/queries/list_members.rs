use super::handler::{CqrsQuery, QueryHandler};
use crate::domain::common::MembershipStatus;
use crate::domain::member::Membership;
use crate::domain::tenant::{OrganizationId, TeamId, TenantId};
use async_trait::async_trait;
use anyhow::Result;

/// 列出成员查询
#[derive(Debug, Clone)]
pub struct ListMembersQuery {
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub team_id: Option<TeamId>,
    pub status: Option<MembershipStatus>,
    pub limit: usize,
    pub offset: usize,
}

impl CqrsQuery for ListMembersQuery {
    type Response = Vec<Membership>;
}

/// 列出成员查询处理器
pub struct ListMembersHandler;

impl ListMembersHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl QueryHandler<ListMembersQuery> for ListMembersHandler {
    type Error = anyhow::Error;

    async fn handle(
        &self,
        _query: ListMembersQuery,
    ) -> Result<<ListMembersQuery as CqrsQuery>::Response, Self::Error> {
        // TODO: 从数据库查询
        Ok(vec![]) // 占位实现
    }
}
