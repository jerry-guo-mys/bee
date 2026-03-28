//! 租户领域服务
//!
//! 提供租户相关的领域服务，协调 Repository 和 Event Publisher。

use std::sync::Arc;

use crate::domain::tenant::{
    DomainEventPublisher, Tenant, TenantError, TenantId, TenantName, TenantRepository, TenantSlug,
};

/// 租户领域服务
///
/// 负责协调租户的创建、暂停、恢复等业务操作，
/// 并确保领域事件的正确发布。
pub struct TenantDomainService<TR, EP> {
    tenant_repo: Arc<TR>,
    event_publisher: Arc<EP>,
}

impl<TR, EP> TenantDomainService<TR, EP>
where
    TR: TenantRepository + 'static,
    EP: DomainEventPublisher + 'static,
{
    /// 创建新的租户领域服务实例
    ///
    /// # Arguments
    /// * `tenant_repo` - 租户 Repository
    /// * `event_publisher` - 领域事件发布器
    pub fn new(tenant_repo: Arc<TR>, event_publisher: Arc<EP>) -> Self {
        Self {
            tenant_repo,
            event_publisher,
        }
    }

    /// 创建租户
    ///
    /// # Arguments
    /// * `name` - 租户名称
    /// * `slug` - 租户 slug (URL 友好的标识符)
    ///
    /// # Returns
    /// * `Result<Tenant, TenantError>` - 创建成功返回租户实例，失败返回错误
    pub async fn create_tenant(&self, name: String, slug: String) -> Result<Tenant, TenantError> {
        // 验证名称和 slug（会返回错误如果无效）
        let _name = TenantName::new(name.clone())?;
        let _slug = TenantSlug::new(slug.clone())?;

        // 检查 slug 是否已存在
        if self.tenant_repo.exists_by_slug(&_slug.as_str()).await? {
            return Err(TenantError::AlreadyExists(format!(
                "Tenant with slug '{}' already exists",
                slug
            )));
        }

        // 创建租户（使用 String 参数）
        let tenant = Tenant::create(name, slug)?;

        // 保存租户
        self.tenant_repo.save(&tenant).await?;

        // 发布 TenantCreated 事件
        let created_event =
            crate::domain::tenant::TenantEvent::Created(crate::domain::tenant::TenantCreated::new(
                tenant.id().clone(),
                tenant.name().to_string(),
                tenant.slug().to_string(),
            ));
        self.event_publisher.publish(created_event).await;

        Ok(tenant)
    }

    /// 暂停租户
    ///
    /// 暂停后，租户下的所有资源将被冻结，用户无法访问。
    ///
    /// # Arguments
    /// * `id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<(), TenantError>` - 成功返回 Ok，失败返回错误
    pub async fn suspend_tenant(&self, id: &TenantId) -> Result<(), TenantError> {
        let mut tenant = self
            .tenant_repo
            .find_by_id(id)
            .await?
            .ok_or(TenantError::NotFound("Tenant not found".into()))?;

        // 检查是否已经是暂停状态
        if tenant.is_suspended() {
            return Ok(());
        }

        tenant.suspend();
        self.tenant_repo.save(&tenant).await?;

        // 发布 TenantSuspended 事件
        let suspended_event = crate::domain::tenant::TenantEvent::Suspended(
            crate::domain::tenant::TenantSuspended::new(tenant.id().clone()),
        );
        self.event_publisher.publish(suspended_event).await;

        Ok(())
    }

    /// 恢复租户
    ///
    /// 从暂停状态恢复到活跃状态。
    ///
    /// # Arguments
    /// * `id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<(), TenantError>` - 成功返回 Ok，失败返回错误
    pub async fn restore_tenant(&self, id: &TenantId) -> Result<(), TenantError> {
        let mut tenant = self
            .tenant_repo
            .find_by_id(id)
            .await?
            .ok_or(TenantError::NotFound("Tenant not found".into()))?;

        // 检查是否已经是活跃状态
        if tenant.is_active() {
            return Ok(());
        }

        tenant.restore();
        self.tenant_repo.save(&tenant).await?;

        // 发布 TenantRestored 事件
        let restored_event = crate::domain::tenant::TenantEvent::Restored(
            crate::domain::tenant::TenantRestored::new(tenant.id().clone()),
        );
        self.event_publisher.publish(restored_event).await;

        Ok(())
    }

    /// 归档租户
    ///
    /// 归档后，租户数据将被保留但不可用，通常用于删除前的软删除状态。
    ///
    /// # Arguments
    /// * `id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<(), TenantError>` - 成功返回 Ok，失败返回错误
    pub async fn archive_tenant(&self, id: &TenantId) -> Result<(), TenantError> {
        let mut tenant = self
            .tenant_repo
            .find_by_id(id)
            .await?
            .ok_or(TenantError::NotFound("Tenant not found".into()))?;

        // 检查是否已经归档
        if tenant.is_archived() {
            return Ok(());
        }

        tenant.archive();
        self.tenant_repo.save(&tenant).await?;

        // 发布 TenantArchived 事件
        let archived_event = crate::domain::tenant::TenantEvent::Archived(
            crate::domain::tenant::TenantArchived::new(tenant.id().clone()),
        );
        self.event_publisher.publish(archived_event).await;

        Ok(())
    }

    /// 删除租户
    ///
    /// 注意：这是硬删除，会永久删除租户数据。
    /// 通常应该使用 `archive_tenant` 进行软删除。
    ///
    /// # Arguments
    /// * `id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<(), TenantError>` - 成功返回 Ok，失败返回错误
    pub async fn delete_tenant(&self, id: &TenantId) -> Result<(), TenantError> {
        // 先归档再删除（确保事件发布）
        self.archive_tenant(id).await?;

        // 物理删除
        self.tenant_repo.delete(id).await?;

        // 发布 TenantDeleted 事件
        let deleted_event = crate::domain::tenant::TenantEvent::Deleted(
            crate::domain::tenant::TenantDeleted::new(id.clone()),
        );
        self.event_publisher.publish(deleted_event).await;

        Ok(())
    }

    /// 根据 ID 获取租户
    ///
    /// # Arguments
    /// * `id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<Option<Tenant>, TenantError>` - 找到返回 Some(Tenant)，否则返回 None
    pub async fn get_tenant_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError> {
        self.tenant_repo.find_by_id(id).await
    }

    /// 根据 slug 获取租户
    ///
    /// # Arguments
    /// * `slug` - 租户 slug
    ///
    /// # Returns
    /// * `Result<Option<Tenant>, TenantError>` - 找到返回 Some(Tenant)，否则返回 None
    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>, TenantError> {
        self.tenant_repo.find_by_slug(slug).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tenant::{InMemoryEventPublisher, InMemoryTenantRepository};

    fn create_service() -> (
        TenantDomainService<InMemoryTenantRepository, InMemoryEventPublisher>,
        Arc<InMemoryTenantRepository>,
        Arc<InMemoryEventPublisher>,
    ) {
        let repo = Arc::new(InMemoryTenantRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = TenantDomainService::new(repo.clone(), publisher.clone());
        (service, repo, publisher)
    }

    #[tokio::test]
    async fn test_create_tenant_success() {
        let (service, _repo, publisher) = create_service();

        let tenant = service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();

        assert_eq!(tenant.name().as_str(), "Test Tenant");
        assert_eq!(tenant.slug().as_str(), "test-tenant");
        assert!(tenant.is_active());

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "tenant.created");
    }

    #[tokio::test]
    async fn test_create_tenant_duplicate_slug() {
        let (service, _repo, _publisher) = create_service();

        // 创建第一个租户
        service
            .create_tenant("Tenant 1".to_string(), "test-slug".to_string())
            .await
            .unwrap();

        // 尝试创建重复 slug 的租户
        let result = service
            .create_tenant("Tenant 2".to_string(), "test-slug".to_string())
            .await;

        assert!(matches!(result, Err(TenantError::AlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_create_tenant_invalid_name() {
        let (service, _repo, _publisher) = create_service();

        let result = service
            .create_tenant("".to_string(), "test-tenant".to_string())
            .await;

        assert!(matches!(result, Err(TenantError::InvalidName(_))));
    }

    #[tokio::test]
    async fn test_create_tenant_invalid_slug() {
        let (service, _repo, _publisher) = create_service();

        let result = service
            .create_tenant("Test Tenant".to_string(), "INVALID_SLUG".to_string())
            .await;

        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
    }

    #[tokio::test]
    async fn test_suspend_tenant() {
        let (service, _repo, publisher) = create_service();

        // 创建租户
        let tenant = service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();

        // 暂停租户
        service.suspend_tenant(tenant.id()).await.unwrap();

        // 验证状态
        let updated = service
            .get_tenant_by_id(tenant.id())
            .await
            .unwrap()
            .unwrap();
        assert!(updated.is_suspended());

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 2); // Created + Suspended
        assert_eq!(events[1].event_type(), "tenant.suspended");
    }

    #[tokio::test]
    async fn test_suspend_already_suspended_tenant() {
        let (service, _repo, _publisher) = create_service();

        // 创建并暂停租户
        let tenant = service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();
        service.suspend_tenant(tenant.id()).await.unwrap();

        // 再次暂停应该无效果（不报错）
        let result = service.suspend_tenant(tenant.id()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_restore_tenant() {
        let (service, _repo, publisher) = create_service();

        // 创建并暂停租户
        let tenant = service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();
        service.suspend_tenant(tenant.id()).await.unwrap();

        // 恢复租户
        service.restore_tenant(tenant.id()).await.unwrap();

        // 验证状态
        let updated = service
            .get_tenant_by_id(tenant.id())
            .await
            .unwrap()
            .unwrap();
        assert!(updated.is_active());

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 3); // Created + Suspended + Restored
        assert_eq!(events[2].event_type(), "tenant.restored");
    }

    #[tokio::test]
    async fn test_restore_already_active_tenant() {
        let (service, _repo, _publisher) = create_service();

        // 创建租户（默认活跃）
        let tenant = service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();

        // 恢复活跃租户应该无效果（不报错）
        let result = service.restore_tenant(tenant.id()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_archive_tenant() {
        let (service, _repo, publisher) = create_service();

        // 创建租户
        let tenant = service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();

        // 归档租户
        service.archive_tenant(tenant.id()).await.unwrap();

        // 验证状态
        let updated = service
            .get_tenant_by_id(tenant.id())
            .await
            .unwrap()
            .unwrap();
        assert!(updated.is_archived());

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 2); // Created + Archived
        assert_eq!(events[1].event_type(), "tenant.archived");
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let (service, _repo, publisher) = create_service();

        // 创建租户
        let tenant = service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();
        let tenant_id = tenant.id().clone();

        // 删除租户
        service.delete_tenant(&tenant_id).await.unwrap();

        // 验证租户已删除
        let found = service.get_tenant_by_id(&tenant_id).await.unwrap();
        assert!(found.is_none());

        // 验证事件已发布（Created + Archived + Deleted）
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 3);
        assert_eq!(events[2].event_type(), "tenant.deleted");
    }

    #[tokio::test]
    async fn test_get_tenant_not_found() {
        let (service, _repo, _publisher) = create_service();

        let non_existent_id = TenantId::generate();
        let result = service.get_tenant_by_id(&non_existent_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_tenant_by_slug() {
        let (service, _repo, _publisher) = create_service();

        service
            .create_tenant("Test Tenant".to_string(), "test-tenant".to_string())
            .await
            .unwrap();

        let tenant = service.get_tenant_by_slug("test-tenant").await.unwrap();
        assert!(tenant.is_some());
        assert_eq!(tenant.unwrap().slug().as_str(), "test-tenant");
    }
}
