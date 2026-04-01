//! 成员领域事件
//!
//! 定义 MemberEvent 枚举并实现 DomainEvent trait。

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::domain::common::MembershipRole;
use crate::domain::event::LegacyDomainEvent as TraitDomainEvent;
use crate::domain::tenant::event::DomainEvent;
use crate::domain::tenant::value_object::{MembershipId, OrganizationId, TeamId, TenantId, UserId};

use super::value_object::{ToolId, ToolRiskLevel, UserEmail};

/// 成员聚合根错误类型
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MemberDomainError {
    #[error("成员不存在：{0}")]
    NotFound(String),

    #[error("成员已存在：{0}")]
    AlreadyExists(String),

    #[error("无效的成员操作：{0}")]
    InvalidOperation(String),

    #[error("成员状态无效：{0}")]
    InvalidStatus(String),

    #[error("成员角色无效：{0}")]
    InvalidRole(String),

    #[error("权限不足：{0}")]
    PermissionDenied(String),

    #[error("工具执行被拒绝：{0}")]
    ToolExecutionDenied(String),

    #[error("数据库错误：{0}")]
    DatabaseError(String),

    #[error("值对象错误：{0}")]
    ValueObject(#[from] crate::domain::member::value_object::ValueObjectError),
}

/// 领域事件 - 成员相关事件
#[derive(Debug, Clone)]
pub enum MemberEvent {
    /// 成员被邀请
    Invited {
        membership_id: MembershipId,
        tenant_id: TenantId,
        organization_id: OrganizationId,
        team_id: Option<TeamId>,
        user_id: UserId,
        email: UserEmail,
        role: MembershipRole,
        occurred_at: DateTime<Utc>,
    },
    /// 成员接受邀请
    InvitationAccepted {
        membership_id: MembershipId,
        user_id: UserId,
        occurred_at: DateTime<Utc>,
    },
    /// 成员被暂停
    Suspended {
        membership_id: MembershipId,
        reason: String,
        occurred_at: DateTime<Utc>,
    },
    /// 成员被移除
    Removed {
        membership_id: MembershipId,
        occurred_at: DateTime<Utc>,
    },
    /// 成员角色变更
    RoleChanged {
        membership_id: MembershipId,
        old_role: MembershipRole,
        new_role: MembershipRole,
        occurred_at: DateTime<Utc>,
    },
    /// 工具策略被添加
    ToolPolicyAdded {
        membership_id: MembershipId,
        tool_id: ToolId,
        risk_level: ToolRiskLevel,
        occurred_at: DateTime<Utc>,
    },
    /// 工具策略被移除
    ToolPolicyRemoved {
        membership_id: MembershipId,
        tool_id: ToolId,
        occurred_at: DateTime<Utc>,
    },
}

/// 成员事件类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemberEventType {
    Invited,
    InvitationAccepted,
    Suspended,
    Removed,
    RoleChanged,
    ToolPolicyAdded,
    ToolPolicyRemoved,
}

impl MemberEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invited => "member.invited",
            Self::InvitationAccepted => "member.invitation_accepted",
            Self::Suspended => "member.suspended",
            Self::Removed => "member.removed",
            Self::RoleChanged => "member.role_changed",
            Self::ToolPolicyAdded => "member.tool_policy_added",
            Self::ToolPolicyRemoved => "member.tool_policy_removed",
        }
    }
}

/// 获取成员事件的类型
impl MemberEvent {
    pub fn event_type(&self) -> MemberEventType {
        match self {
            Self::Invited { .. } => MemberEventType::Invited,
            Self::InvitationAccepted { .. } => MemberEventType::InvitationAccepted,
            Self::Suspended { .. } => MemberEventType::Suspended,
            Self::Removed { .. } => MemberEventType::Removed,
            Self::RoleChanged { .. } => MemberEventType::RoleChanged,
            Self::ToolPolicyAdded { .. } => MemberEventType::ToolPolicyAdded,
            Self::ToolPolicyRemoved { .. } => MemberEventType::ToolPolicyRemoved,
        }
    }

    /// 获取事件的序列号（用于事件溯源）
    #[allow(dead_code)]
    pub fn sequence(&self) -> u64 {
        // 简单实现，实际应用中应该使用事件版本号
        0
    }

    /// 获取事件发生的时间戳
    pub fn occurred_on(&self) -> DateTime<Utc> {
        match self {
            Self::Invited { occurred_at, .. } => *occurred_at,
            Self::InvitationAccepted { occurred_at, .. } => *occurred_at,
            Self::Suspended { occurred_at, .. } => *occurred_at,
            Self::Removed { occurred_at, .. } => *occurred_at,
            Self::RoleChanged { occurred_at, .. } => *occurred_at,
            Self::ToolPolicyAdded { occurred_at, .. } => *occurred_at,
            Self::ToolPolicyRemoved { occurred_at, .. } => *occurred_at,
        }
    }
}

/// 为 MemberEvent 实现 DomainEvent trait
impl DomainEvent for MemberEvent {
    fn aggregate_id(&self) -> &str {
        match self {
            Self::Invited { membership_id, .. } => membership_id.as_str(),
            Self::InvitationAccepted { membership_id, .. } => membership_id.as_str(),
            Self::Suspended { membership_id, .. } => membership_id.as_str(),
            Self::Removed { membership_id, .. } => membership_id.as_str(),
            Self::RoleChanged { membership_id, .. } => membership_id.as_str(),
            Self::ToolPolicyAdded { membership_id, .. } => membership_id.as_str(),
            Self::ToolPolicyRemoved { membership_id, .. } => membership_id.as_str(),
        }
    }

    fn aggregate_type(&self) -> &str {
        "membership"
    }

    fn event_type(&self) -> &str {
        match self {
            Self::Invited { .. } => "member.invited",
            Self::InvitationAccepted { .. } => "member.invitation_accepted",
            Self::Suspended { .. } => "member.suspended",
            Self::Removed { .. } => "member.removed",
            Self::RoleChanged { .. } => "member.role_changed",
            Self::ToolPolicyAdded { .. } => "member.tool_policy_added",
            Self::ToolPolicyRemoved { .. } => "member.tool_policy_removed",
        }
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_on()
    }

    fn to_json(&self) -> String {
        match self {
            Self::Invited {
                membership_id,
                email,
                role,
                ..
            } => {
                format!(
                    r#"{{"membership_id":"{}","email":"{}","role":"{:?}"}}"#,
                    membership_id.as_str(),
                    email.as_str(),
                    role
                )
            }
            Self::InvitationAccepted {
                membership_id,
                user_id,
                ..
            } => {
                format!(
                    r#"{{"membership_id":"{}","user_id":"{}"}}"#,
                    membership_id.as_str(),
                    user_id.as_str()
                )
            }
            Self::Suspended {
                membership_id,
                reason,
                ..
            } => {
                format!(
                    r#"{{"membership_id":"{}","reason":"{}"}}"#,
                    membership_id.as_str(),
                    reason
                )
            }
            Self::Removed { membership_id, .. } => {
                format!(r#"{{"membership_id":"{}"}}"#, membership_id.as_str())
            }
            Self::RoleChanged {
                membership_id,
                old_role,
                new_role,
                ..
            } => {
                format!(
                    r#"{{"membership_id":"{}","old_role":"{:?}","new_role":"{:?}"}}"#,
                    membership_id.as_str(),
                    old_role,
                    new_role
                )
            }
            Self::ToolPolicyAdded {
                membership_id,
                tool_id,
                risk_level,
                ..
            } => {
                format!(
                    r#"{{"membership_id":"{}","tool_id":"{}","risk_level":"{:?}"}}"#,
                    membership_id.as_str(),
                    tool_id.as_str(),
                    risk_level
                )
            }
            Self::ToolPolicyRemoved {
                membership_id,
                tool_id,
                ..
            } => {
                format!(
                    r#"{{"membership_id":"{}","tool_id":"{}"}}"#,
                    membership_id.as_str(),
                    tool_id.as_str()
                )
            }
        }
    }
}

/// 将 MemberEvent 转换为通用 DomainEvent
impl From<MemberEvent> for TraitDomainEvent {
    fn from(event: MemberEvent) -> Self {
        match event {
            MemberEvent::Invited { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.invited:{}", membership_id.as_str()))
            }
            MemberEvent::InvitationAccepted { membership_id, .. } => TraitDomainEvent::Custom(
                format!("member.invitation_accepted:{}", membership_id.as_str()),
            ),
            MemberEvent::Suspended { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.suspended:{}", membership_id.as_str()))
            }
            MemberEvent::Removed { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.removed:{}", membership_id.as_str()))
            }
            MemberEvent::RoleChanged { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.role_changed:{}", membership_id.as_str()))
            }
            MemberEvent::ToolPolicyAdded { membership_id, .. } => TraitDomainEvent::Custom(
                format!("member.tool_policy_added:{}", membership_id.as_str()),
            ),
            MemberEvent::ToolPolicyRemoved { membership_id, .. } => TraitDomainEvent::Custom(
                format!("member.tool_policy_removed:{}", membership_id.as_str()),
            ),
        }
    }
}
