//! Organization 领域事件定义
//!
//! 领域事件表示组织中发生的重要事情，用于解耦模块间通信。

use crate::domain::common::now;
use chrono::{DateTime, Utc};

use super::value_object::OrganizationId;
use crate::domain::tenant::event::DomainEvent;
use crate::domain::tenant::TenantId;

/// Organization 事件枚举
#[derive(Debug, Clone)]
pub enum OrganizationEvent {
    Created(OrganizationCreated),
    Updated(OrganizationUpdated),
    Deleted(OrganizationDeleted),
}

impl OrganizationEvent {
    /// 获取事件的聚合根 ID
    pub fn aggregate_id(&self) -> &str {
        match self {
            OrganizationEvent::Created(e) => e.organization_id.as_str(),
            OrganizationEvent::Updated(e) => e.organization_id.as_str(),
            OrganizationEvent::Deleted(e) => e.organization_id.as_str(),
        }
    }

    /// 获取事件的聚合根类型
    pub fn aggregate_type(&self) -> &str {
        "organization"
    }

    /// 获取事件类型
    pub fn event_type(&self) -> &str {
        match self {
            OrganizationEvent::Created(_) => "organization.created",
            OrganizationEvent::Updated(_) => "organization.updated",
            OrganizationEvent::Deleted(_) => "organization.deleted",
        }
    }

    /// 获取事件发生时间
    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            OrganizationEvent::Created(e) => e.occurred_at,
            OrganizationEvent::Updated(e) => e.occurred_at,
            OrganizationEvent::Deleted(e) => e.occurred_at,
        }
    }

    /// 序列化为 JSON 字符串（用于持久化）
    pub fn to_json(&self) -> String {
        match self {
            OrganizationEvent::Created(e) => {
                format!(
                    r#"{{"organization_id":"{}","tenant_id":"{}","name":"{}"}}"#,
                    e.organization_id, e.tenant_id, e.name
                )
            }
            OrganizationEvent::Updated(e) => {
                format!(
                    r#"{{"organization_id":"{}","name":"{}"}}"#,
                    e.organization_id, e.name
                )
            }
            OrganizationEvent::Deleted(e) => {
                format!(r#"{{"organization_id":"{}"}}"#, e.organization_id)
            }
        }
    }
}

impl DomainEvent for OrganizationEvent {
    fn aggregate_id(&self) -> &str {
        match self {
            OrganizationEvent::Created(e) => e.organization_id.as_str(),
            OrganizationEvent::Updated(e) => e.organization_id.as_str(),
            OrganizationEvent::Deleted(e) => e.organization_id.as_str(),
        }
    }

    fn aggregate_type(&self) -> &str {
        "organization"
    }

    fn event_type(&self) -> &str {
        match self {
            OrganizationEvent::Created(_) => "organization.created",
            OrganizationEvent::Updated(_) => "organization.updated",
            OrganizationEvent::Deleted(_) => "organization.deleted",
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            OrganizationEvent::Created(e) => e.occurred_at,
            OrganizationEvent::Updated(e) => e.occurred_at,
            OrganizationEvent::Deleted(e) => e.occurred_at,
        }
    }

    fn to_json(&self) -> String {
        match self {
            OrganizationEvent::Created(e) => {
                format!(
                    r#"{{"organization_id":"{}","tenant_id":"{}","name":"{}"}}"#,
                    e.organization_id, e.tenant_id, e.name
                )
            }
            OrganizationEvent::Updated(e) => {
                format!(
                    r#"{{"organization_id":"{}","name":"{}"}}"#,
                    e.organization_id, e.name
                )
            }
            OrganizationEvent::Deleted(e) => {
                format!(r#"{{"organization_id":"{}"}}"#, e.organization_id)
            }
        }
    }
}

/// 组织创建事件
#[derive(Debug, Clone)]
pub struct OrganizationCreated {
    pub organization_id: OrganizationId,
    pub tenant_id: TenantId,
    pub name: String,
    pub slug: String,
    pub occurred_at: DateTime<Utc>,
}

impl OrganizationCreated {
    pub fn new(
        organization_id: OrganizationId,
        tenant_id: TenantId,
        name: String,
        slug: String,
    ) -> Self {
        Self {
            organization_id,
            tenant_id,
            name,
            slug,
            occurred_at: now(),
        }
    }
}

/// 组织更新事件
#[derive(Debug, Clone)]
pub struct OrganizationUpdated {
    pub organization_id: OrganizationId,
    pub name: String,
    pub occurred_at: DateTime<Utc>,
}

impl OrganizationUpdated {
    pub fn new(organization_id: OrganizationId, name: String) -> Self {
        Self {
            organization_id,
            name,
            occurred_at: now(),
        }
    }
}

/// 组织删除事件
#[derive(Debug, Clone)]
pub struct OrganizationDeleted {
    pub organization_id: OrganizationId,
    pub occurred_at: DateTime<Utc>,
}

impl OrganizationDeleted {
    pub fn new(organization_id: OrganizationId) -> Self {
        Self {
            organization_id,
            occurred_at: now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organization_created_event() {
        let org_id = OrganizationId::generate();
        let tenant_id = TenantId::generate();
        let event = OrganizationCreated::new(
            org_id.clone(),
            tenant_id.clone(),
            "Test Organization".to_string(),
            "test-org".to_string(),
        );

        assert_eq!(event.organization_id, org_id);
        assert_eq!(event.tenant_id, tenant_id);
        assert_eq!(event.name, "Test Organization");
        assert_eq!(event.slug, "test-org");

        let org_event = OrganizationEvent::Created(event.clone());
        assert_eq!(org_event.aggregate_id(), org_id.as_str());
        assert_eq!(org_event.aggregate_type(), "organization");
        assert_eq!(org_event.event_type(), "organization.created");
    }

    #[test]
    fn test_organization_updated_event() {
        let org_id = OrganizationId::generate();
        let event = OrganizationUpdated::new(org_id.clone(), "New Name".to_string());

        let org_event = OrganizationEvent::Updated(event);
        assert_eq!(org_event.aggregate_id(), org_id.as_str());
        assert_eq!(org_event.event_type(), "organization.updated");
    }

    #[test]
    fn test_organization_deleted_event() {
        let org_id = OrganizationId::generate();
        let event = OrganizationDeleted::new(org_id.clone());

        let org_event = OrganizationEvent::Deleted(event);
        assert_eq!(org_event.aggregate_id(), org_id.as_str());
        assert_eq!(org_event.event_type(), "organization.deleted");
    }

    #[test]
    fn test_event_to_json() {
        let org_id = OrganizationId::generate();
        let tenant_id = TenantId::generate();
        let event = OrganizationCreated::new(
            org_id.clone(),
            tenant_id.clone(),
            "Test Organization".to_string(),
            "test-org".to_string(),
        );
        let org_event = OrganizationEvent::Created(event);
        let json = org_event.to_json();

        assert!(json.contains("organization_id"));
        assert!(json.contains("tenant_id"));
        assert!(json.contains("Test Organization"));
    }
}
