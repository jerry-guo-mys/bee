//! 列出成员查询

use super::handler::{CqrsQuery, QueryHandler};
use crate::domain::common::MembershipStatus;
use crate::domain::member::Membership;
use crate::domain::service::MemberDomainService;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// 列出成员查询
#[derive(Debug, Clone)]
pub struct ListMembersQuery {
    pub tenant_id: crate::domain::tenant::TenantId,
    pub organization_id: crate::domain::tenant::OrganizationId,
    pub team_id: Option<crate::domain::tenant::TeamId>,
    pub status: Option<MembershipStatus>,
    pub limit: usize,
    pub offset: usize,
}

impl CqrsQuery for ListMembersQuery {
    type Response = Vec<Membership>;
}

/// 列出成员查询处理器
pub struct ListMembersHandler<MR, EP> {
    member_service: Arc<MemberDomainService<MR, EP>>,
}

impl<MR, EP> ListMembersHandler<MR, EP>
where
    MR: crate::domain::member::MembershipRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    pub fn new(member_service: Arc<MemberDomainService<MR, EP>>) -> Self {
        Self { member_service }
    }
}

#[async_trait]
impl<MR, EP> QueryHandler<ListMembersQuery> for ListMembersHandler<MR, EP>
where
    MR: crate::domain::member::MembershipRepository + 'static,
    EP: crate::domain::event::DomainEventPublisher + 'static,
{
    type Error = anyhow::Error;

    async fn handle(
        &self,
        query: ListMembersQuery,
    ) -> Result<<ListMembersQuery as CqrsQuery>::Response, Self::Error> {
        // 记录审计日志
        tracing::info!(
            target: "audit",
            event = "members_listed",
            tenant_id = %query.tenant_id,
            organization_id = %query.organization_id,
            team_id = ?query.team_id,
            status = ?query.status,
            "Members listed"
        );

        // 调用领域服务获取成员列表
        let memberships = self
            .member_service
            .get_members_by_organization(&query.organization_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list members: {}", e))?;

        // 过滤和分页
        let mut filtered: Vec<_> = memberships
            .into_iter()
            .filter(|m| {
                // 按团队 ID 过滤
                if let Some(ref team_id) = query.team_id {
                    m.team_id() == Some(team_id)
                } else {
                    true
                }
            })
            .filter(|m| {
                // 按状态过滤
                if let Some(ref status) = query.status {
                    m.status() == status
                } else {
                    true
                }
            })
            .collect();

        // 应用分页
        filtered.sort_by(|a, b| a.created_at().cmp(b.created_at()));
        let paginated = filtered
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect();

        Ok(paginated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::InMemoryEventPublisher;
    use crate::domain::member::InMemoryMembershipRepository;
    use crate::domain::tenant::UserId;

    #[tokio::test]
    async fn test_list_members_handler_success() {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
        let handler = ListMembersHandler::new(service.clone());

        // 创建两个成员
        let org_id = crate::domain::tenant::OrganizationId::generate();
        for i in 0..3 {
            let email = crate::domain::member::value_object::UserEmail::new(format!(
                "test{}@example.com",
                i
            ))
            .unwrap();
            let membership = service
                .invite_member(
                    crate::domain::tenant::TenantId::generate(),
                    org_id.clone(),
                    None,
                    email,
                    crate::domain::common::MembershipRole::Member,
                    UserId::generate(),
                )
                .await
                .unwrap();

            // 接受前两个邀请
            if i < 2 {
                service
                    .accept_invite(membership.id(), UserId::generate())
                    .await
                    .unwrap();
            }
        }

        // 查询所有成员
        let query = ListMembersQuery {
            tenant_id: crate::domain::tenant::TenantId::generate(),
            organization_id: org_id.clone(),
            team_id: None,
            status: None,
            limit: 10,
            offset: 0,
        };

        let result = handler.handle(query).await;
        assert!(result.is_ok());
        let members = result.unwrap();
        assert_eq!(members.len(), 3);
    }

    #[tokio::test]
    async fn test_list_members_handler_filter_by_status() {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
        let handler = ListMembersHandler::new(service.clone());

        // 创建成员
        let org_id = crate::domain::tenant::OrganizationId::generate();
        let email1 =
            crate::domain::member::value_object::UserEmail::new("active@example.com".to_string())
                .unwrap();
        let membership1 = service
            .invite_member(
                crate::domain::tenant::TenantId::generate(),
                org_id.clone(),
                None,
                email1,
                crate::domain::common::MembershipRole::Member,
                UserId::generate(),
            )
            .await
            .unwrap();
        service
            .accept_invite(membership1.id(), UserId::generate())
            .await
            .unwrap();

        let email2 =
            crate::domain::member::value_object::UserEmail::new("pending@example.com".to_string())
                .unwrap();
        let _membership2 = service
            .invite_member(
                crate::domain::tenant::TenantId::generate(),
                org_id.clone(),
                None,
                email2,
                crate::domain::common::MembershipRole::Member,
                UserId::generate(),
            )
            .await
            .unwrap();

        // 只查询 Active 状态的成员
        let query = ListMembersQuery {
            tenant_id: crate::domain::tenant::TenantId::generate(),
            organization_id: org_id.clone(),
            team_id: None,
            status: Some(crate::domain::common::MembershipStatus::Active),
            limit: 10,
            offset: 0,
        };

        let result = handler.handle(query).await;
        assert!(result.is_ok());
        let members = result.unwrap();
        assert_eq!(members.len(), 1);
    }

    #[tokio::test]
    async fn test_list_members_handler_pagination() {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = Arc::new(MemberDomainService::new(repo.clone(), publisher.clone()));
        let handler = ListMembersHandler::new(service.clone());

        // 创建 5 个成员
        let org_id = crate::domain::tenant::OrganizationId::generate();
        for i in 0..5 {
            let email = crate::domain::member::value_object::UserEmail::new(format!(
                "test{}@example.com",
                i
            ))
            .unwrap();
            let membership = service
                .invite_member(
                    crate::domain::tenant::TenantId::generate(),
                    org_id.clone(),
                    None,
                    email,
                    crate::domain::common::MembershipRole::Member,
                    UserId::generate(),
                )
                .await
                .unwrap();

            // 接受所有邀请
            service
                .accept_invite(membership.id(), UserId::generate())
                .await
                .unwrap();
        }

        // 查询第一页（limit=2, offset=0）
        let query1 = ListMembersQuery {
            tenant_id: crate::domain::tenant::TenantId::generate(),
            organization_id: org_id.clone(),
            team_id: None,
            status: None,
            limit: 2,
            offset: 0,
        };
        let result1 = handler.handle(query1).await.unwrap();
        assert_eq!(result1.len(), 2);

        // 查询第二页（limit=2, offset=2）
        let query2 = ListMembersQuery {
            tenant_id: crate::domain::tenant::TenantId::generate(),
            organization_id: org_id.clone(),
            team_id: None,
            status: None,
            limit: 2,
            offset: 2,
        };
        let result2 = handler.handle(query2).await.unwrap();
        assert_eq!(result2.len(), 2);

        // 查询第三页（limit=2, offset=4）
        let query3 = ListMembersQuery {
            tenant_id: crate::domain::tenant::TenantId::generate(),
            organization_id: org_id.clone(),
            team_id: None,
            status: None,
            limit: 2,
            offset: 4,
        };
        let result3 = handler.handle(query3).await.unwrap();
        assert_eq!(result3.len(), 1);
    }
}
