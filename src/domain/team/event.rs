//! Team 领域事件定义
//!
//! 领域事件表示团队中发生的重要事情，用于解耦模块间通信。

use crate::domain::common::now;
use chrono::{DateTime, Utc};

use super::value_object::TeamId;
use crate::domain::tenant::{OrganizationId, TenantId};

/// Team 事件枚举
#[derive(Debug, Clone)]
pub enum TeamEvent {
    Created(TeamCreated),
    Updated(TeamUpdated),
    Deleted(TeamDeleted),
}

impl TeamEvent {
    /// 获取事件的聚合根 ID
    pub fn aggregate_id(&self) -> &str {
        match self {
            TeamEvent::Created(e) => e.team_id.as_str(),
            TeamEvent::Updated(e) => e.team_id.as_str(),
            TeamEvent::Deleted(e) => e.team_id.as_str(),
        }
    }

    /// 获取事件的聚合根类型
    pub fn aggregate_type(&self) -> &str {
        "team"
    }

    /// 获取事件类型
    pub fn event_type(&self) -> &str {
        match self {
            TeamEvent::Created(_) => "team.created",
            TeamEvent::Updated(_) => "team.updated",
            TeamEvent::Deleted(_) => "team.deleted",
        }
    }

    /// 获取事件发生时间
    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            TeamEvent::Created(e) => e.occurred_at,
            TeamEvent::Updated(e) => e.occurred_at,
            TeamEvent::Deleted(e) => e.occurred_at,
        }
    }

    /// 序列化为 JSON 字符串（用于持久化）
    pub fn to_json(&self) -> String {
        match self {
            TeamEvent::Created(e) => {
                format!(
                    r#"{{"team_id":"{}","tenant_id":"{}","organization_id":"{}","name":"{}"}}"#,
                    e.team_id, e.tenant_id, e.organization_id, e.name
                )
            }
            TeamEvent::Updated(e) => {
                format!(
                    r#"{{"team_id":"{}","name":"{}"}}"#,
                    e.team_id, e.name
                )
            }
            TeamEvent::Deleted(e) => {
                format!(r#"{{"team_id":"{}"}}"#, e.team_id)
            }
        }
    }
}

/// 团队创建事件
#[derive(Debug, Clone)]
pub struct TeamCreated {
    pub team_id: TeamId,
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub name: String,
    pub code: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl TeamCreated {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        team_id: TeamId,
        tenant_id: TenantId,
        organization_id: OrganizationId,
        name: String,
        code: Option<String>,
    ) -> Self {
        Self {
            team_id,
            tenant_id,
            organization_id,
            name,
            code,
            occurred_at: now(),
        }
    }
}

/// 团队更新事件
#[derive(Debug, Clone)]
pub struct TeamUpdated {
    pub team_id: TeamId,
    pub name: String,
    pub occurred_at: DateTime<Utc>,
}

impl TeamUpdated {
    pub fn new(team_id: TeamId, name: String) -> Self {
        Self {
            team_id,
            name,
            occurred_at: now(),
        }
    }
}

/// 团队删除事件
#[derive(Debug, Clone)]
pub struct TeamDeleted {
    pub team_id: TeamId,
    pub occurred_at: DateTime<Utc>,
}

impl TeamDeleted {
    pub fn new(team_id: TeamId) -> Self {
        Self {
            team_id,
            occurred_at: now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_created_event() {
        let team_id = TeamId::generate();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let event = TeamCreated::new(
            team_id.clone(),
            tenant_id.clone(),
            org_id.clone(),
            "Test Team".to_string(),
            Some("TEST-001".to_string()),
        );

        assert_eq!(event.team_id, team_id);
        assert_eq!(event.tenant_id, tenant_id);
        assert_eq!(event.organization_id, org_id);
        assert_eq!(event.name, "Test Team");
        assert_eq!(event.code, Some("TEST-001".to_string()));

        let team_event = TeamEvent::Created(event.clone());
        assert_eq!(team_event.aggregate_id(), team_id.as_str());
        assert_eq!(team_event.aggregate_type(), "team");
        assert_eq!(team_event.event_type(), "team.created");
    }

    #[test]
    fn test_team_updated_event() {
        let team_id = TeamId::generate();
        let event = TeamUpdated::new(team_id.clone(), "New Name".to_string());

        let team_event = TeamEvent::Updated(event);
        assert_eq!(team_event.aggregate_id(), team_id.as_str());
        assert_eq!(team_event.event_type(), "team.updated");
    }

    #[test]
    fn test_team_deleted_event() {
        let team_id = TeamId::generate();
        let event = TeamDeleted::new(team_id.clone());

        let team_event = TeamEvent::Deleted(event);
        assert_eq!(team_event.aggregate_id(), team_id.as_str());
        assert_eq!(team_event.event_type(), "team.deleted");
    }

    #[test]
    fn test_event_to_json() {
        let team_id = TeamId::generate();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let event = TeamCreated::new(
            team_id.clone(),
            tenant_id.clone(),
            org_id.clone(),
            "Test Team".to_string(),
            Some("TEST-001".to_string()),
        );
        let team_event = TeamEvent::Created(event);
        let json = team_event.to_json();

        assert!(json.contains("team_id"));
        assert!(json.contains("tenant_id"));
        assert!(json.contains("organization_id"));
        assert!(json.contains("Test Team"));
    }
}
