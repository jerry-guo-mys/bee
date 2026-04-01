//! 租户领域事件定义
//!
//! 领域事件表示领域中发生的重要事情，用于解耦模块间通信。

use crate::domain::common::now;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use super::value_object::TenantId;

/// 领域事件 trait
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
    /// 事件发生的聚合根 ID
    fn aggregate_id(&self) -> &str;

    /// 事件发生的聚合根类型
    fn aggregate_type(&self) -> &str;

    /// 事件类型
    fn event_type(&self) -> &str;

    /// 事件发生时间
    fn occurred_at(&self) -> DateTime<Utc>;

    /// 序列化为 JSON 字符串（用于持久化）
    fn to_json(&self) -> String;
}

/// 租户事件枚举
#[derive(Debug, Clone)]
pub enum TenantEvent {
    Created(TenantCreated),
    Suspended(TenantSuspended),
    Restored(TenantRestored),
    Archived(TenantArchived),
    Deleted(TenantDeleted),
}

impl DomainEvent for TenantEvent {
    fn aggregate_id(&self) -> &str {
        match self {
            TenantEvent::Created(e) => e.tenant_id.as_str(),
            TenantEvent::Suspended(e) => e.tenant_id.as_str(),
            TenantEvent::Restored(e) => e.tenant_id.as_str(),
            TenantEvent::Archived(e) => e.tenant_id.as_str(),
            TenantEvent::Deleted(e) => e.tenant_id.as_str(),
        }
    }

    fn aggregate_type(&self) -> &str {
        "tenant"
    }

    fn event_type(&self) -> &str {
        match self {
            TenantEvent::Created(_) => "tenant.created",
            TenantEvent::Suspended(_) => "tenant.suspended",
            TenantEvent::Restored(_) => "tenant.restored",
            TenantEvent::Archived(_) => "tenant.archived",
            TenantEvent::Deleted(_) => "tenant.deleted",
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            TenantEvent::Created(e) => e.occurred_at,
            TenantEvent::Suspended(e) => e.occurred_at,
            TenantEvent::Restored(e) => e.occurred_at,
            TenantEvent::Archived(e) => e.occurred_at,
            TenantEvent::Deleted(e) => e.occurred_at,
        }
    }

    fn to_json(&self) -> String {
        match self {
            TenantEvent::Created(e) => {
                format!(
                    r#"{{"tenant_id":"{}","name":"{}","slug":"{}","status":"active"}}"#,
                    e.tenant_id, e.name, e.slug
                )
            }
            TenantEvent::Suspended(e) => {
                format!(r#"{{"tenant_id":"{}"}}"#, e.tenant_id)
            }
            TenantEvent::Restored(e) => {
                format!(r#"{{"tenant_id":"{}"}}"#, e.tenant_id)
            }
            TenantEvent::Archived(e) => {
                format!(r#"{{"tenant_id":"{}"}}"#, e.tenant_id)
            }
            TenantEvent::Deleted(e) => {
                format!(r#"{{"tenant_id":"{}"}}"#, e.tenant_id)
            }
        }
    }
}

/// 租户创建事件
#[derive(Debug, Clone)]
pub struct TenantCreated {
    pub tenant_id: TenantId,
    pub name: String,
    pub slug: String,
    pub occurred_at: DateTime<Utc>,
}

impl TenantCreated {
    pub fn new(tenant_id: TenantId, name: String, slug: String) -> Self {
        Self {
            tenant_id,
            name,
            slug,
            occurred_at: now(),
        }
    }
}

/// 租户暂停事件
#[derive(Debug, Clone)]
pub struct TenantSuspended {
    pub tenant_id: TenantId,
    pub occurred_at: DateTime<Utc>,
}

impl TenantSuspended {
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            occurred_at: now(),
        }
    }
}

/// 租户恢复事件
#[derive(Debug, Clone)]
pub struct TenantRestored {
    pub tenant_id: TenantId,
    pub occurred_at: DateTime<Utc>,
}

impl TenantRestored {
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            occurred_at: now(),
        }
    }
}

/// 租户归档事件
#[derive(Debug, Clone)]
pub struct TenantArchived {
    pub tenant_id: TenantId,
    pub occurred_at: DateTime<Utc>,
}

impl TenantArchived {
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            occurred_at: now(),
        }
    }
}

/// 租户删除事件
#[derive(Debug, Clone)]
pub struct TenantDeleted {
    pub tenant_id: TenantId,
    pub occurred_at: DateTime<Utc>,
}

impl TenantDeleted {
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            occurred_at: now(),
        }
    }
}

/// 领域事件发布器 trait
#[async_trait::async_trait]
pub trait DomainEventPublisher: Send + Sync {
    /// 发布单个事件
    async fn publish(&self, event: impl DomainEvent + 'static);

    /// 发布多个事件（批量）
    async fn publish_all(&self, events: Vec<Box<dyn DomainEvent>>);
}

/// 内存事件发布器（用于测试）
#[derive(Default)]
pub struct InMemoryEventPublisher {
    events: Arc<tokio::sync::Mutex<Vec<Box<dyn DomainEvent>>>>,
}

impl InMemoryEventPublisher {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_events(&self) -> Vec<Box<dyn DomainEvent>> {
        let guard = self.events.lock().await;
        // Clone by serializing to JSON and back (for test purposes)
        // Or simply return a new Vec with cloned event metadata
        guard
            .iter()
            .map(|e| {
                // For simplicity in tests, we clone by recreating based on event type
                match e.event_type() {
                    "tenant.created" => {
                        let _json = e.to_json();
                        // Parse back minimal info for testing
                        Box::new(TenantEvent::Created(TenantCreated {
                            tenant_id: TenantId::from_str(e.aggregate_id()),
                            name: String::new(),
                            slug: String::new(),
                            occurred_at: e.occurred_at(),
                        })) as Box<dyn DomainEvent>
                    }
                    "tenant.suspended" => Box::new(TenantEvent::Suspended(TenantSuspended {
                        tenant_id: TenantId::from_str(e.aggregate_id()),
                        occurred_at: e.occurred_at(),
                    })) as Box<dyn DomainEvent>,
                    "tenant.restored" => Box::new(TenantEvent::Restored(TenantRestored {
                        tenant_id: TenantId::from_str(e.aggregate_id()),
                        occurred_at: e.occurred_at(),
                    })) as Box<dyn DomainEvent>,
                    "tenant.archived" => Box::new(TenantEvent::Archived(TenantArchived {
                        tenant_id: TenantId::from_str(e.aggregate_id()),
                        occurred_at: e.occurred_at(),
                    })) as Box<dyn DomainEvent>,
                    "tenant.deleted" => Box::new(TenantEvent::Deleted(TenantDeleted {
                        tenant_id: TenantId::from_str(e.aggregate_id()),
                        occurred_at: e.occurred_at(),
                    })) as Box<dyn DomainEvent>,
                    "organization.created" => {
                        Box::new(crate::domain::organization::OrganizationEvent::Created(
                            crate::domain::organization::OrganizationCreated {
                                organization_id: crate::domain::organization::OrganizationId::from_str(e.aggregate_id()),
                                tenant_id: crate::domain::tenant::TenantId::from_str(e.aggregate_id()),
                                name: String::new(),
                                slug: String::new(),
                                occurred_at: e.occurred_at(),
                            },
                        )) as Box<dyn DomainEvent>
                    }
                    "organization.updated" => {
                        Box::new(crate::domain::organization::OrganizationEvent::Updated(
                            crate::domain::organization::OrganizationUpdated {
                                organization_id: crate::domain::organization::OrganizationId::from_str(e.aggregate_id()),
                                name: String::new(),
                                occurred_at: e.occurred_at(),
                            },
                        )) as Box<dyn DomainEvent>
                    }
                    "organization.deleted" => {
                        Box::new(crate::domain::organization::OrganizationEvent::Deleted(
                            crate::domain::organization::OrganizationDeleted {
                                organization_id: crate::domain::organization::OrganizationId::from_str(e.aggregate_id()),
                                occurred_at: e.occurred_at(),
                            },
                        )) as Box<dyn DomainEvent>
                    }
                    "team.created" => {
                        Box::new(crate::domain::team::TeamEvent::Created(
                            crate::domain::team::TeamCreated {
                                team_id: crate::domain::team::TeamId::from_str(e.aggregate_id()),
                                tenant_id: crate::domain::tenant::TenantId::from_str(e.aggregate_id()),
                                organization_id: crate::domain::tenant::OrganizationId::from_str(e.aggregate_id()),
                                name: String::new(),
                                code: None,
                                occurred_at: e.occurred_at(),
                            },
                        )) as Box<dyn DomainEvent>
                    }
                    "team.updated" => {
                        Box::new(crate::domain::team::TeamEvent::Updated(
                            crate::domain::team::TeamUpdated {
                                team_id: crate::domain::team::TeamId::from_str(e.aggregate_id()),
                                name: String::new(),
                                occurred_at: e.occurred_at(),
                            },
                        )) as Box<dyn DomainEvent>
                    }
                    "team.deleted" => {
                        Box::new(crate::domain::team::TeamEvent::Deleted(
                            crate::domain::team::TeamDeleted {
                                team_id: crate::domain::team::TeamId::from_str(e.aggregate_id()),
                                occurred_at: e.occurred_at(),
                            },
                        )) as Box<dyn DomainEvent>
                    }
                    "member.invited" => {
                        Box::new(crate::domain::member::MemberEvent::Invited {
                            membership_id: crate::domain::tenant::MembershipId::from_str(e.aggregate_id()),
                            tenant_id: crate::domain::tenant::TenantId::from_str(e.aggregate_id()),
                            organization_id: crate::domain::tenant::OrganizationId::from_str(e.aggregate_id()),
                            team_id: None,
                            user_id: crate::domain::tenant::UserId::from_str(e.aggregate_id()),
                            email: crate::domain::member::value_object::UserEmail::new("test@example.com".to_string()).unwrap(),
                            role: crate::domain::common::MembershipRole::Member,
                            occurred_at: e.occurred_at(),
                        }) as Box<dyn DomainEvent>
                    }
                    "member.invitation_accepted" => {
                        Box::new(crate::domain::member::MemberEvent::InvitationAccepted {
                            membership_id: crate::domain::tenant::MembershipId::from_str(e.aggregate_id()),
                            user_id: crate::domain::tenant::UserId::from_str(e.aggregate_id()),
                            occurred_at: e.occurred_at(),
                        }) as Box<dyn DomainEvent>
                    }
                    "member.suspended" => {
                        Box::new(crate::domain::member::MemberEvent::Suspended {
                            membership_id: crate::domain::tenant::MembershipId::from_str(e.aggregate_id()),
                            reason: "Test".to_string(),
                            occurred_at: e.occurred_at(),
                        }) as Box<dyn DomainEvent>
                    }
                    _ => panic!("Unknown event type: {}", e.event_type()),
                }
            })
            .collect()
    }

    pub async fn clear(&self) {
        self.events.lock().await.clear();
    }
}

#[async_trait::async_trait]
impl DomainEventPublisher for InMemoryEventPublisher {
    async fn publish(&self, event: impl DomainEvent + 'static) {
        self.events.lock().await.push(Box::new(event));
    }

    async fn publish_all(&self, events: Vec<Box<dyn DomainEvent>>) {
        let mut guard = self.events.lock().await;
        for event in events {
            guard.push(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_created_event() {
        let tenant_id = TenantId::generate();
        let event = TenantCreated::new(
            tenant_id.clone(),
            "Test Tenant".to_string(),
            "test-tenant".to_string(),
        );

        assert_eq!(event.tenant_id, tenant_id);
        assert_eq!(event.name, "Test Tenant");
        assert_eq!(event.slug, "test-tenant");

        let tenant_event = TenantEvent::Created(event.clone());
        assert_eq!(tenant_event.aggregate_id(), tenant_id.as_str());
        assert_eq!(tenant_event.aggregate_type(), "tenant");
        assert_eq!(tenant_event.event_type(), "tenant.created");
    }

    #[test]
    fn test_tenant_suspended_event() {
        let tenant_id = TenantId::generate();
        let event = TenantSuspended::new(tenant_id.clone());

        let tenant_event = TenantEvent::Suspended(event);
        assert_eq!(tenant_event.aggregate_id(), tenant_id.as_str());
        assert_eq!(tenant_event.event_type(), "tenant.suspended");
    }

    #[test]
    fn test_tenant_restored_event() {
        let tenant_id = TenantId::generate();
        let event = TenantRestored::new(tenant_id.clone());

        let tenant_event = TenantEvent::Restored(event);
        assert_eq!(tenant_event.event_type(), "tenant.restored");
    }

    #[test]
    fn test_tenant_archived_event() {
        let tenant_id = TenantId::generate();
        let event = TenantArchived::new(tenant_id.clone());

        let tenant_event = TenantEvent::Archived(event);
        assert_eq!(tenant_event.event_type(), "tenant.archived");
    }

    #[test]
    fn test_tenant_deleted_event() {
        let tenant_id = TenantId::generate();
        let event = TenantDeleted::new(tenant_id.clone());

        let tenant_event = TenantEvent::Deleted(event);
        assert_eq!(tenant_event.event_type(), "tenant.deleted");
    }

    #[test]
    fn test_event_to_json() {
        let tenant_id = TenantId::generate();
        let event = TenantCreated::new(
            tenant_id.clone(),
            "Test Tenant".to_string(),
            "test-tenant".to_string(),
        );
        let tenant_event = TenantEvent::Created(event);
        let json = tenant_event.to_json();

        assert!(json.contains("tenant_id"));
        assert!(json.contains("Test Tenant"));
        assert!(json.contains("test-tenant"));
    }

    #[tokio::test]
    async fn test_in_memory_event_publisher() {
        let publisher = InMemoryEventPublisher::new();
        let tenant_id = TenantId::generate();

        let event = TenantEvent::Created(TenantCreated::new(
            tenant_id.clone(),
            "Test Tenant".to_string(),
            "test-tenant".to_string(),
        ));

        publisher.publish(event).await;
        let events = publisher.get_events().await;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].aggregate_id(), tenant_id.as_str());

        publisher.clear().await;
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_event_publisher_batch() {
        let publisher = InMemoryEventPublisher::new();
        let tenant_id1 = TenantId::generate();
        let tenant_id2 = TenantId::generate();

        let event1: Box<dyn DomainEvent> = Box::new(TenantEvent::Created(TenantCreated::new(
            tenant_id1.clone(),
            "Tenant 1".to_string(),
            "tenant-1".to_string(),
        )));
        let event2: Box<dyn DomainEvent> = Box::new(TenantEvent::Created(TenantCreated::new(
            tenant_id2.clone(),
            "Tenant 2".to_string(),
            "tenant-2".to_string(),
        )));

        publisher.publish_all(vec![event1, event2]).await;
        let events = publisher.get_events().await;

        assert_eq!(events.len(), 2);
    }
}
