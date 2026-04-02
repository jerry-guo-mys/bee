//! Organization 领域服务
//!
//! 提供组织相关的领域服务，协调 Repository 和 Event Publisher。

use std::sync::Arc;

use crate::domain::event::{DomainEventPublisher, InMemoryEventPublisher};
use crate::domain::organization::{
    Organization, OrganizationError, OrganizationEvent, OrganizationId, OrganizationName,
    OrganizationRepository, OrganizationSlug,
};
use crate::domain::tenant::TenantId;

/// Organization 领域服务
///
/// 负责协调组织的创建、更新、删除等业务操作，
/// 并确保领域事件的正确发布。
pub struct OrganizationDomainService<OR, EP> {
    org_repo: Arc<OR>,
    event_publisher: Arc<EP>,
}

impl<OR, EP> OrganizationDomainService<OR, EP>
where
    OR: OrganizationRepository<Error = OrganizationError> + 'static,
    EP: DomainEventPublisher + 'static,
{
    /// 创建新的组织领域服务实例
    ///
    /// # Arguments
    /// * `org_repo` - 组织 Repository
    /// * `event_publisher` - 领域事件发布器
    pub fn new(org_repo: Arc<OR>, event_publisher: Arc<EP>) -> Self {
        Self {
            org_repo,
            event_publisher,
        }
    }

    /// 创建组织
    ///
    /// # Arguments
    /// * `tenant_id` - 所属租户 ID
    /// * `name` - 组织名称
    /// * `slug` - 组织 slug (URL 友好的标识符)
    ///
    /// # Returns
    /// * `Result<Organization, OrganizationError>` - 创建成功返回组织实例，失败返回错误
    pub async fn create_organization(
        &self,
        tenant_id: TenantId,
        name: String,
        slug: String,
    ) -> Result<Organization, OrganizationError> {
        // 验证名称和 slug（会返回错误如果无效）
        let _name = OrganizationName::new(name.clone())?;
        let _slug = OrganizationSlug::new(slug.clone())?;

        // 检查 slug 是否已存在
        let existing = self
            .org_repo
            .find_by_slug(&tenant_id, &_slug.as_str())
            .await?;
        if existing.is_some() {
            return Err(OrganizationError::AlreadyExists(format!(
                "Organization with slug '{}' already exists in this tenant",
                slug
            )));
        }

        // 创建组织
        let org = Organization::create(tenant_id.clone(), name.clone(), slug.clone())?;
        let org_id = org.id().clone();

        // 保存组织
        self.org_repo.save(&org).await?;

        // 发布 OrganizationCreated 事件
        let created_event = OrganizationEvent::Created(
            crate::domain::organization::OrganizationCreated::new(org_id, tenant_id, name, slug),
        );
        self.event_publisher.publish(created_event).await;

        Ok(org)
    }

    /// 更新组织名称
    ///
    /// # Arguments
    /// * `id` - 组织 ID
    /// * `name` - 新的组织名称
    ///
    /// # Returns
    /// * `Result<(), OrganizationError>` - 成功返回 Ok，失败返回错误
    pub async fn update_organization_name(
        &self,
        id: &OrganizationId,
        name: String,
    ) -> Result<(), OrganizationError> {
        let mut org = self
            .org_repo
            .find_by_id(id)
            .await?
            .ok_or(OrganizationError::NotFound("Organization not found".into()))?;

        let old_name = org.name().to_string();
        org.update_name(name.clone())?;
        self.org_repo.save(&org).await?;

        // 发布 OrganizationUpdated 事件
        let updated_event = OrganizationEvent::Updated(
            crate::domain::organization::OrganizationUpdated::new(id.clone(), name),
        );
        self.event_publisher.publish(updated_event).await;

        Ok(())
    }

    /// 删除组织
    ///
    /// # Arguments
    /// * `id` - 组织 ID
    ///
    /// # Returns
    /// * `Result<(), OrganizationError>` - 成功返回 Ok，失败返回错误
    pub async fn delete_organization(&self, id: &OrganizationId) -> Result<(), OrganizationError> {
        // 先检查是否存在
        let org = self
            .org_repo
            .find_by_id(id)
            .await?
            .ok_or(OrganizationError::NotFound("Organization not found".into()))?;

        // 发布 OrganizationDeleted 事件
        let deleted_event = OrganizationEvent::Deleted(
            crate::domain::organization::OrganizationDeleted::new(id.clone()),
        );
        self.event_publisher.publish(deleted_event).await;

        // 物理删除
        self.org_repo.delete(id).await?;

        Ok(())
    }

    /// 根据 ID 获取组织
    ///
    /// # Arguments
    /// * `id` - 组织 ID
    ///
    /// # Returns
    /// * `Result<Option<Organization>, OrganizationError>` - 找到返回 Some(Organization)，否则返回 None
    pub async fn get_organization_by_id(
        &self,
        id: &OrganizationId,
    ) -> Result<Option<Organization>, OrganizationError> {
        self.org_repo.find_by_id(id).await
    }

    /// 根据租户 ID 获取所有组织
    ///
    /// # Arguments
    /// * `tenant_id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<Vec<Organization>, OrganizationError>` - 组织列表
    pub async fn get_organizations_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<Organization>, OrganizationError> {
        self.org_repo.find_by_tenant(tenant_id).await
    }

    /// 根据 slug 获取组织
    ///
    /// # Arguments
    /// * `tenant_id` - 租户 ID
    /// * `slug` - 组织 slug
    ///
    /// # Returns
    /// * `Result<Option<Organization>, OrganizationError>` - 找到返回 Some(Organization)，否则返回 None
    pub async fn get_organization_by_slug(
        &self,
        tenant_id: &TenantId,
        slug: &str,
    ) -> Result<Option<Organization>, OrganizationError> {
        self.org_repo.find_by_slug(tenant_id, slug).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::organization::InMemoryOrganizationRepository;

    fn create_service() -> (
        OrganizationDomainService<InMemoryOrganizationRepository, InMemoryEventPublisher>,
        Arc<InMemoryOrganizationRepository>,
        Arc<InMemoryEventPublisher>,
    ) {
        let repo = Arc::new(InMemoryOrganizationRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = OrganizationDomainService::new(repo.clone(), publisher.clone());
        (service, repo, publisher)
    }

    #[tokio::test]
    async fn test_create_organization_success() {
        let (service, _repo, publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org = service
            .create_organization(
                tenant_id.clone(),
                "Test Organization".to_string(),
                "test-org".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(org.name().as_str(), "Test Organization");
        assert_eq!(org.slug().as_str(), "test-org");
        assert_eq!(org.tenant_id(), &tenant_id);

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "organization.created");
    }

    #[tokio::test]
    async fn test_create_organization_duplicate_slug() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();

        // 创建第一个组织
        service
            .create_organization(
                tenant_id.clone(),
                "Org 1".to_string(),
                "test-slug".to_string(),
            )
            .await
            .unwrap();

        // 尝试创建重复 slug 的组织
        let result = service
            .create_organization(
                tenant_id.clone(),
                "Org 2".to_string(),
                "test-slug".to_string(),
            )
            .await;

        assert!(matches!(result, Err(OrganizationError::AlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_create_organization_invalid_name() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let result = service
            .create_organization(tenant_id, "".to_string(), "test-org".to_string())
            .await;

        assert!(matches!(result, Err(OrganizationError::InvalidName(_))));
    }

    #[tokio::test]
    async fn test_update_organization_name() {
        let (service, _repo, publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org = service
            .create_organization(
                tenant_id.clone(),
                "Test Organization".to_string(),
                "test-org".to_string(),
            )
            .await
            .unwrap();

        // 更新名称
        service
            .update_organization_name(org.id(), "Updated Organization".to_string())
            .await
            .unwrap();

        // 验证更新
        let updated = service
            .get_organization_by_id(org.id())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name().as_str(), "Updated Organization");

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 2); // Created + Updated
        assert_eq!(events[1].event_type(), "organization.updated");
    }

    #[tokio::test]
    async fn test_delete_organization() {
        let (service, _repo, publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org = service
            .create_organization(
                tenant_id.clone(),
                "Test Organization".to_string(),
                "test-org".to_string(),
            )
            .await
            .unwrap();
        let org_id = org.id().clone();

        // 删除组织
        service.delete_organization(&org_id).await.unwrap();

        // 验证组织已删除
        let found = service.get_organization_by_id(&org_id).await.unwrap();
        assert!(found.is_none());

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 2); // Created + Deleted
        assert_eq!(events[1].event_type(), "organization.deleted");
    }

    #[tokio::test]
    async fn test_get_organization_not_found() {
        let (service, _repo, _publisher) = create_service();

        let non_existent_id = OrganizationId::generate();
        let result = service.get_organization_by_id(&non_existent_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_organizations_by_tenant() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        service
            .create_organization(tenant_id.clone(), "Org 1".to_string(), "org-1".to_string())
            .await
            .unwrap();
        service
            .create_organization(tenant_id.clone(), "Org 2".to_string(), "org-2".to_string())
            .await
            .unwrap();

        let orgs = service
            .get_organizations_by_tenant(&tenant_id)
            .await
            .unwrap();
        assert_eq!(orgs.len(), 2);

        // 不同租户应该返回空
        let other_tenant = TenantId::generate();
        let orgs = service
            .get_organizations_by_tenant(&other_tenant)
            .await
            .unwrap();
        assert_eq!(orgs.len(), 0);
    }

    #[tokio::test]
    async fn test_get_organization_by_slug() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        service
            .create_organization(
                tenant_id.clone(),
                "Test Organization".to_string(),
                "test-org".to_string(),
            )
            .await
            .unwrap();

        let org = service
            .get_organization_by_slug(&tenant_id, "test-org")
            .await
            .unwrap();
        assert!(org.is_some());
        assert_eq!(org.unwrap().slug().as_str(), "test-org");
    }
}
