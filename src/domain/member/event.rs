//! 成员领域事件
//!
//! 导出 MemberEvent 并实现 DomainEvent trait。

use crate::domain::event::DomainEvent as TraitDomainEvent;

use super::entity::MemberEvent;

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
    pub fn sequence(&self) -> u64 {
        // 简单实现，实际应用中应该使用事件版本号
        0
    }

    /// 获取事件发生的时间戳
    pub fn occurred_on(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

/// 将 MemberEvent 转换为通用 DomainEvent
impl From<MemberEvent> for TraitDomainEvent {
    fn from(event: MemberEvent) -> Self {
        match event {
            MemberEvent::Invited { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.invited:{}", membership_id.as_str()))
            }
            MemberEvent::InvitationAccepted { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.invitation_accepted:{}", membership_id.as_str()))
            }
            MemberEvent::Suspended { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.suspended:{}", membership_id.as_str()))
            }
            MemberEvent::Removed { membership_id } => {
                TraitDomainEvent::Custom(format!("member.removed:{}", membership_id.as_str()))
            }
            MemberEvent::RoleChanged { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.role_changed:{}", membership_id.as_str()))
            }
            MemberEvent::ToolPolicyAdded { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.tool_policy_added:{}", membership_id.as_str()))
            }
            MemberEvent::ToolPolicyRemoved { membership_id, .. } => {
                TraitDomainEvent::Custom(format!("member.tool_policy_removed:{}", membership_id.as_str()))
            }
        }
    }
}
