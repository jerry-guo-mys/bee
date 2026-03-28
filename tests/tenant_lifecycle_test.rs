//! 租户生命周期集成测试
//!
//! 测试租户从创建到归档/删除的完整生命周期

use std::sync::Arc;

use bee_agents::application::commands::{
    CreateTenantCommand, CreateTenantHandler,
};
use bee_agents::application::queries::{
    GetTenantQuery, GetTenantHandler,
};
use bee_agents::domain::event::{DomainEventPublisher, InMemoryEventPublisher};
use bee_agents::domain::tenant::{
    InMemoryTenantRepository, TenantDomainService, TenantId, TenantRepository, UserId,
};

/// 创建测试用的服务组合
fn create_test_services() -> (
    Arc<TenantDomainService<InMemoryTenantRepository, InMemoryEventPublisher>>,
    Arc<InMemoryTenantRepository>,
    Arc<InMemoryEventPublisher>,
) {
    let repo = Arc::new(InMemoryTenantRepository::new());
    let publisher = Arc::new(InMemoryEventPublisher::new());
    let service = Arc::new(TenantDomainService::new(repo.clone(), publisher.clone()));
    (service, repo, publisher)
}

#[tokio::test]
async fn test_tenant_lifecycle_create_and_query() {
    // 创建服务
    let (tenant_service, _repo, _publisher) = create_test_services();

    // 创建命令处理器
    let create_handler = CreateTenantHandler::new(tenant_service.clone());

    // 创建租户
    let command = CreateTenantCommand {
        name: "Test Organization".to_string(),
        slug: "test-org".to_string(),
        creator_id: UserId::generate(),
    };

    let result = create_handler.handle(command).await;
    assert!(result.is_ok(), "Tenant creation should succeed");

    let tenant = result.unwrap();
    assert_eq!(tenant.name().as_str(), "Test Organization");
    assert_eq!(tenant.slug().as_str(), "test-org");
    assert_eq!(tenant.status(), &bee_agents::domain::common::TenantStatus::Active);

    // 创建查询处理器
    let query_handler = GetTenantHandler::new(tenant_service.clone());

    // 查询租户
    let query = GetTenantQuery {
        tenant_id: tenant.id().clone(),
    };

    let query_result = query_handler.handle(query).await;
    assert!(query_result.is_ok());
    let found = query_result.unwrap();
    assert!(found.is_some());

    let found_tenant = found.unwrap();
    assert_eq!(found_tenant.id(), tenant.id());
    assert_eq!(found_tenant.name().as_str(), "Test Organization");
}

#[tokio::test]
async fn test_tenant_lifecycle_suspend_and_restore() {
    let (tenant_service, _repo, _publisher) = create_test_services();

    // 创建租户
    let tenant = tenant_service
        .create_tenant("Suspend Test".to_string(), "suspend-test".to_string())
        .await
        .unwrap();

    // 验证初始状态为 Active
    assert_eq!(tenant.status(), &bee_agents::domain::common::TenantStatus::Active);

    // 暂停租户
    tenant_service.suspend_tenant(tenant.id()).await.unwrap();

    // 验证状态变为 Suspended
    let suspended = tenant_service
        .get_tenant_by_id(tenant.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(suspended.status(), &bee_agents::domain::common::TenantStatus::Suspended);

    // 恢复租户
    tenant_service.restore_tenant(tenant.id()).await.unwrap();

    // 验证状态恢复为 Active
    let restored = tenant_service
        .get_tenant_by_id(tenant.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored.status(), &bee_agents::domain::common::TenantStatus::Active);
}

#[tokio::test]
async fn test_tenant_lifecycle_archive() {
    let (tenant_service, _repo, _publisher) = create_test_services();

    // 创建租户
    let tenant = tenant_service
        .create_tenant("Archive Test".to_string(), "archive-test".to_string())
        .await
        .unwrap();

    // 归档租户
    tenant_service.archive_tenant(tenant.id()).await.unwrap();

    // 验证状态变为 Archived
    let archived = tenant_service
        .get_tenant_by_id(tenant.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(archived.status(), &bee_agents::domain::common::TenantStatus::Archived);

    // 验证 Archived 状态不能恢复
    tenant_service.restore_tenant(tenant.id()).await.unwrap();
    let still_archived = tenant_service
        .get_tenant_by_id(tenant.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(still_archived.status(), &bee_agents::domain::common::TenantStatus::Archived);
}

#[tokio::test]
async fn test_tenant_lifecycle_duplicate_slug_error() {
    let (tenant_service, _repo, _publisher) = create_test_services();
    let create_handler = CreateTenantHandler::new(tenant_service.clone());

    // 创建第一个租户
    let command1 = CreateTenantCommand {
        name: "Tenant 1".to_string(),
        slug: "duplicate-slug".to_string(),
        creator_id: UserId::generate(),
    };

    let result1 = create_handler.handle(command1).await;
    assert!(result1.is_ok());

    // 尝试创建重复 slug 的租户
    let command2 = CreateTenantCommand {
        name: "Tenant 2".to_string(),
        slug: "duplicate-slug".to_string(),
        creator_id: UserId::generate(),
    };

    let result2 = create_handler.handle(command2).await;
    assert!(result2.is_err());
    assert!(result2.unwrap_err().to_string().contains("already exists"));
}

#[tokio::test]
async fn test_tenant_lifecycle_invalid_data() {
    let (tenant_service, _repo, _publisher) = create_test_services();
    let create_handler = CreateTenantHandler::new(tenant_service.clone());

    // 测试空名称
    let command = CreateTenantCommand {
        name: "".to_string(),
        slug: "valid-slug".to_string(),
        creator_id: UserId::generate(),
    };

    let result = create_handler.handle(command).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid name"));

    // 测试无效 slug（大写字母）
    let command = CreateTenantCommand {
        name: "Valid Name".to_string(),
        slug: "INVALID_SLUG".to_string(),
        creator_id: UserId::generate(),
    };

    let result = create_handler.handle(command).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid slug"));
}

#[tokio::test]
async fn test_tenant_lifecycle_events_published() {
    let (tenant_service, _repo, publisher) = create_test_services();

    // 创建租户
    let tenant = tenant_service
        .create_tenant("Event Test".to_string(), "event-test".to_string())
        .await
        .unwrap();

    // 验证 Created 事件已发布
    let events = publisher.get_events().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type(), "tenant.created");

    // 暂停租户
    tenant_service.suspend_tenant(tenant.id()).await.unwrap();

    // 验证 Suspended 事件已发布
    let events = publisher.get_events().await;
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type(), "tenant.suspended");

    // 恢复租户
    tenant_service.restore_tenant(tenant.id()).await.unwrap();

    // 验证 Restored 事件已发布
    let events = publisher.get_events().await;
    assert_eq!(events.len(), 3);
    assert_eq!(events[2].event_type(), "tenant.restored");
}
