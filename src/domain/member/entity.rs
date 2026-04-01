//! 成员聚合根定义
//!
//! Membership 是成员聚合的聚合根，管理成员的所有业务逻辑。

use chrono::{DateTime, Utc};

use crate::domain::common::{MembershipRole, MembershipStatus, Permission};
use crate::domain::tenant::value_object::{MembershipId, OrganizationId, TeamId, TenantId, UserId};

use super::event::MemberEvent;
use super::value_object::{ToolId, ToolPolicy, ToolRiskLevel, UserEmail};

// 从 event 模块重新导出 MemberDomainError
pub use super::event::MemberDomainError;

/// 成员聚合根
#[derive(Debug, Clone)]
pub struct Membership {
    /// 成员 ID
    id: MembershipId,
    /// 租户 ID
    tenant_id: TenantId,
    /// 组织 ID
    organization_id: OrganizationId,
    /// 团队 ID（可选，为 None 表示组织级成员）
    team_id: Option<TeamId>,
    /// 用户 ID
    user_id: Option<UserId>,
    /// 用户邮箱
    email: UserEmail,
    /// 成员角色
    role: MembershipRole,
    /// 成员状态
    status: MembershipStatus,
    /// 创建时间
    created_at: DateTime<Utc>,
    /// 更新时间
    updated_at: DateTime<Utc>,
    /// 工具策略列表
    tool_policies: Vec<ToolPolicy>,
    /// 待发布的领域事件
    pending_events: Vec<MemberEvent>,
}

impl Membership {
    // ========================================================================
    // 构造函数
    // ========================================================================

    /// 邀请新成员
    ///
    /// 创建一个新的成员记录，状态为 Pending（待处理）
    pub fn invite(
        tenant_id: TenantId,
        organization_id: OrganizationId,
        team_id: Option<TeamId>,
        user_id: Option<UserId>,
        email: UserEmail,
        role: MembershipRole,
    ) -> Result<Self, MemberDomainError> {
        let now = Utc::now();
        let membership = Self {
            id: MembershipId::generate(),
            tenant_id: tenant_id.clone(),
            organization_id: organization_id.clone(),
            team_id: team_id.clone(),
            user_id: user_id.clone(),
            email: email.clone(),
            role: role.clone(),
            status: MembershipStatus::Pending,
            created_at: now,
            updated_at: now,
            tool_policies: Vec::new(),
            pending_events: Vec::new(),
        };

        // 记录领域事件
        let event = MemberEvent::Invited {
            membership_id: membership.id.clone(),
            tenant_id,
            organization_id,
            team_id,
            user_id: user_id.unwrap_or_else(|| UserId::generate()),
            email,
            role,
            occurred_at: now,
        };

        Ok(membership.with_event(event))
    }

    /// 从存储加载成员（不发布事件）
    pub fn load(
        id: MembershipId,
        tenant_id: TenantId,
        organization_id: OrganizationId,
        team_id: Option<TeamId>,
        user_id: Option<UserId>,
        email: UserEmail,
        role: MembershipRole,
        status: MembershipStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        tool_policies: Vec<ToolPolicy>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            organization_id,
            team_id,
            user_id,
            email,
            role,
            status,
            created_at,
            updated_at,
            tool_policies,
            pending_events: Vec::new(),
        }
    }

    // ========================================================================
    // 辅助方法
    // ========================================================================

    /// 添加领域事件
    fn with_event(mut self, event: MemberEvent) -> Self {
        self.pending_events.push(event);
        self
    }

    /// 更新 updated_at 时间戳
    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    // ========================================================================
    // 获取方法
    // ========================================================================

    /// 获取成员 ID
    pub fn id(&self) -> &MembershipId {
        &self.id
    }

    /// 获取租户 ID
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    /// 获取组织 ID
    pub fn organization_id(&self) -> &OrganizationId {
        &self.organization_id
    }

    /// 获取团队 ID
    pub fn team_id(&self) -> Option<&TeamId> {
        self.team_id.as_ref()
    }

    /// 获取用户 ID
    pub fn user_id(&self) -> Option<&UserId> {
        self.user_id.as_ref()
    }

    /// 获取邮箱
    pub fn email(&self) -> &UserEmail {
        &self.email
    }

    /// 获取角色
    pub fn role(&self) -> &MembershipRole {
        &self.role
    }

    /// 获取状态
    pub fn status(&self) -> &MembershipStatus {
        &self.status
    }

    /// 获取创建时间
    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    /// 获取更新时间
    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }

    /// 获取工具策略列表
    pub fn tool_policies(&self) -> &[ToolPolicy] {
        &self.tool_policies
    }

    /// 获取待发布的领域事件
    pub fn pending_events(&self) -> &[MemberEvent] {
        &self.pending_events
    }

    /// 清除已处理的领域事件
    pub fn clear_events(&mut self) {
        self.pending_events.clear();
    }

    // ========================================================================
    // 行为方法
    // ========================================================================

    /// 接受邀请
    ///
    /// 将成员状态从 Pending 改为 Active
    pub fn accept_invite(&mut self, user_id: UserId) -> Result<(), MemberDomainError> {
        if self.status != MembershipStatus::Pending {
            return Err(MemberDomainError::InvalidOperation(
                "只有待处理的邀请才能被接受".to_string(),
            ));
        }

        self.user_id = Some(user_id.clone());
        self.status = MembershipStatus::Active;
        self.touch();

        let event = MemberEvent::InvitationAccepted {
            membership_id: self.id.clone(),
            user_id,
            occurred_at: Utc::now(),
        };
        self.pending_events.push(event);

        Ok(())
    }

    /// 暂停成员
    ///
    /// 将成员状态从 Active 改为 Suspended
    pub fn suspend(&mut self, reason: String) -> Result<(), MemberDomainError> {
        if self.status != MembershipStatus::Active {
            return Err(MemberDomainError::InvalidOperation(
                "只有活跃成员才能被暂停".to_string(),
            ));
        }

        self.status = MembershipStatus::Suspended;
        self.touch();

        let event = MemberEvent::Suspended {
            membership_id: self.id.clone(),
            reason,
            occurred_at: Utc::now(),
        };
        self.pending_events.push(event);

        Ok(())
    }

    /// 移除成员
    ///
    /// 将成员状态改为 Removed
    pub fn remove(&mut self) -> Result<(), MemberDomainError> {
        if self.status == MembershipStatus::Removed {
            return Err(MemberDomainError::InvalidOperation(
                "成员已被移除".to_string(),
            ));
        }

        self.status = MembershipStatus::Removed;
        self.touch();

        let event = MemberEvent::Removed {
            membership_id: self.id.clone(),
            occurred_at: Utc::now(),
        };
        self.pending_events.push(event);

        Ok(())
    }

    /// 变更成员角色
    pub fn change_role(&mut self, new_role: MembershipRole) -> Result<(), MemberDomainError> {
        if self.status == MembershipStatus::Removed {
            return Err(MemberDomainError::InvalidOperation(
                "已移除的成员不能变更角色".to_string(),
            ));
        }

        let old_role = self.role.clone();
        self.role = new_role.clone();
        self.touch();

        let event = MemberEvent::RoleChanged {
            membership_id: self.id.clone(),
            old_role,
            new_role,
            occurred_at: Utc::now(),
        };
        self.pending_events.push(event);

        Ok(())
    }

    /// 添加工具策略
    pub fn add_tool_policy(&mut self, policy: ToolPolicy) -> Result<(), MemberDomainError> {
        if self.status == MembershipStatus::Removed {
            return Err(MemberDomainError::InvalidOperation(
                "已移除的成员不能添加工具策略".to_string(),
            ));
        }

        // 移除已存在的相同工具策略
        self.tool_policies
            .retain(|p| p.tool_id() != policy.tool_id());

        let tool_id = policy.tool_id().clone();
        let risk_level = policy.risk_level();

        self.tool_policies.push(policy);
        self.touch();

        let event = MemberEvent::ToolPolicyAdded {
            membership_id: self.id.clone(),
            tool_id,
            risk_level,
            occurred_at: Utc::now(),
        };
        self.pending_events.push(event);

        Ok(())
    }

    /// 移除工具策略
    pub fn remove_tool_policy(&mut self, tool_id: &ToolId) -> Result<(), MemberDomainError> {
        if self.status == MembershipStatus::Removed {
            return Err(MemberDomainError::InvalidOperation(
                "已移除的成员不能移除工具策略".to_string(),
            ));
        }

        let original_len = self.tool_policies.len();
        self.tool_policies.retain(|p| p.tool_id() != tool_id);
        let removed = original_len - self.tool_policies.len();

        if removed == 0 {
            return Err(MemberDomainError::InvalidOperation(
                "工具策略不存在".to_string(),
            ));
        }

        self.touch();

        let event = MemberEvent::ToolPolicyRemoved {
            membership_id: self.id.clone(),
            tool_id: tool_id.clone(),
            occurred_at: Utc::now(),
        };
        self.pending_events.push(event);

        Ok(())
    }

    /// 检查是否可以执行指定工具
    ///
    /// 检查逻辑：
    /// 1. 成员状态必须是 Active
    /// 2. 查找是否有匹配的工具策略
    /// 3. 如果有策略，检查是否允许执行且风险等级足够
    /// 4. 如果没有策略，根据角色默认权限判断
    pub fn can_execute_tool(&self, tool_id: &ToolId, required_level: ToolRiskLevel) -> bool {
        // 只有活跃成员才能执行工具
        if self.status != MembershipStatus::Active {
            return false;
        }

        // 查找是否有匹配的工具策略
        if let Some(policy) = self.tool_policies.iter().find(|p| p.tool_id() == tool_id) {
            return policy.can_execute(required_level);
        }

        // 没有明确策略时，根据角色判断默认权限
        match self.role {
            MembershipRole::PlatformAdmin => true,
            MembershipRole::OrgAdmin => required_level <= ToolRiskLevel::High,
            MembershipRole::TeamAdmin => required_level <= ToolRiskLevel::Medium,
            MembershipRole::Member => required_level <= ToolRiskLevel::Low,
            MembershipRole::Viewer => false,
        }
    }

    /// 检查是否有指定权限
    pub fn has_permission(&self, permission: &Permission) -> bool {
        // 只有活跃成员才有权限
        if self.status != MembershipStatus::Active {
            return false;
        }

        self.role.has_permission(permission)
    }

    /// 检查是否是组织级成员（不属于特定团队）
    pub fn is_organization_level(&self) -> bool {
        self.team_id.is_none()
    }

    /// 检查是否是团队级成员
    pub fn is_team_level(&self) -> bool {
        self.team_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_email() -> UserEmail {
        UserEmail::new("test@example.com".to_string()).unwrap()
    }

    fn create_test_tenant_id() -> TenantId {
        TenantId::generate()
    }

    fn create_test_org_id() -> OrganizationId {
        OrganizationId::generate()
    }

    fn create_test_team_id() -> TeamId {
        TeamId::generate()
    }

    fn create_test_user_id() -> UserId {
        UserId::generate()
    }

    #[test]
    fn test_invite_creates_pending_membership() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();

        let membership = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::Member,
        )
        .unwrap();

        assert_eq!(membership.status(), &MembershipStatus::Pending);
        assert_eq!(membership.role(), &MembershipRole::Member);
        assert_eq!(membership.email(), &email);
        assert!(membership
            .pending_events()
            .iter()
            .any(|e| matches!(e, MemberEvent::Invited { .. })));
    }

    #[test]
    fn test_accept_invite() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();

        assert_eq!(membership.status(), &MembershipStatus::Pending);

        membership.accept_invite(user_id.clone()).unwrap();

        assert_eq!(membership.status(), &MembershipStatus::Active);
        assert_eq!(membership.user_id(), Some(&user_id));
        assert!(membership
            .pending_events()
            .iter()
            .any(|e| matches!(e, MemberEvent::InvitationAccepted { .. })));
    }

    #[test]
    fn test_accept_invite_only_pending() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();

        // 先接受
        membership.accept_invite(user_id.clone()).unwrap();

        // 再次接受应该失败
        let result = membership.accept_invite(user_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_suspend_member() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();

        assert_eq!(membership.status(), &MembershipStatus::Active);

        membership.suspend("违反规定".to_string()).unwrap();

        assert_eq!(membership.status(), &MembershipStatus::Suspended);
        assert!(membership
            .pending_events()
            .iter()
            .any(|e| matches!(e, MemberEvent::Suspended { .. })));
    }

    #[test]
    fn test_remove_member() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();

        membership.remove().unwrap();

        assert_eq!(membership.status(), &MembershipStatus::Removed);
        assert!(membership
            .pending_events()
            .iter()
            .any(|e| matches!(e, MemberEvent::Removed { .. })));
    }

    #[test]
    fn test_change_role() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();

        assert_eq!(membership.role(), &MembershipRole::Member);

        membership.change_role(MembershipRole::TeamAdmin).unwrap();

        assert_eq!(membership.role(), &MembershipRole::TeamAdmin);
        assert!(membership
            .pending_events()
            .iter()
            .any(|e| matches!(e, MemberEvent::RoleChanged { .. })));
    }

    #[test]
    fn test_change_role_removed_member() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();
        membership.remove().unwrap();

        let result = membership.change_role(MembershipRole::TeamAdmin);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_tool_policy() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();

        let policy = ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::Medium, true);

        membership.add_tool_policy(policy).unwrap();

        assert_eq!(membership.tool_policies().len(), 1);
        assert_eq!(membership.tool_policies()[0].tool_id().as_str(), "shell");
    }

    #[test]
    fn test_add_tool_policy_replaces_existing() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();

        let policy1 = ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::Low, true);
        let policy2 = ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::High, false);

        membership.add_tool_policy(policy1).unwrap();
        membership.add_tool_policy(policy2).unwrap();

        assert_eq!(membership.tool_policies().len(), 1);
        assert_eq!(
            membership.tool_policies()[0].risk_level(),
            ToolRiskLevel::High
        );
        assert!(!membership.tool_policies()[0].is_allowed());
    }

    #[test]
    fn test_remove_tool_policy() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();

        let policy = ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::Medium, true);
        membership.add_tool_policy(policy).unwrap();

        membership
            .remove_tool_policy(&ToolId::from_str("shell"))
            .unwrap();

        assert_eq!(membership.tool_policies().len(), 0);
    }

    #[test]
    fn test_can_execute_tool_active_member() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();

        // 添加策略允许 shell 工具（Medium 风险）
        let policy = ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::Medium, true);
        membership.add_tool_policy(policy).unwrap();

        // 可以执行低风险工具
        assert!(membership.can_execute_tool(&ToolId::from_str("shell"), ToolRiskLevel::Low));
        // 可以执行同等风险工具
        assert!(membership.can_execute_tool(&ToolId::from_str("shell"), ToolRiskLevel::Medium));
        // 不能执行高风险工具
        assert!(!membership.can_execute_tool(&ToolId::from_str("shell"), ToolRiskLevel::High));
    }

    #[test]
    fn test_can_execute_tool_inactive_member() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        // 不接受邀请，保持 Pending 状态

        assert!(!membership.can_execute_tool(&ToolId::from_str("shell"), ToolRiskLevel::Low));
    }

    #[test]
    fn test_can_execute_tool_by_role() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        // PlatformAdmin 可以执行所有工具
        let mut admin = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::PlatformAdmin,
        )
        .unwrap();
        admin.accept_invite(user_id.clone()).unwrap();
        assert!(admin.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Critical));

        // OrgAdmin 可以执行 High 及以下
        let mut org_admin = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::OrgAdmin,
        )
        .unwrap();
        org_admin.accept_invite(user_id.clone()).unwrap();
        assert!(org_admin.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::High));
        assert!(!org_admin.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Critical));

        // Viewer 不能执行任何工具
        let mut viewer = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::Viewer,
        )
        .unwrap();
        viewer.accept_invite(user_id).unwrap();
        assert!(!viewer.can_execute_tool(&ToolId::from_str("any"), ToolRiskLevel::Low));
    }

    #[test]
    fn test_has_permission() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let user_id = create_test_user_id();

        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, MembershipRole::Member)
                .unwrap();
        membership.accept_invite(user_id).unwrap();

        // Member 有 AgentRead 权限
        assert!(membership.has_permission(&Permission::AgentRead));
        // Member 没有 OrgWrite 权限
        assert!(!membership.has_permission(&Permission::OrgWrite));
    }

    #[test]
    fn test_is_organization_vs_team_level() {
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let team_id = create_test_team_id();
        let email = create_test_email();

        // 组织级成员
        let org_member = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email.clone(),
            MembershipRole::Member,
        )
        .unwrap();
        assert!(org_member.is_organization_level());
        assert!(!org_member.is_team_level());

        // 团队级成员
        let team_member = Membership::invite(
            tenant_id,
            org_id,
            Some(team_id),
            None,
            email,
            MembershipRole::Member,
        )
        .unwrap();
        assert!(!team_member.is_organization_level());
        assert!(team_member.is_team_level());
    }

    #[test]
    fn test_load_from_repository() {
        let id = MembershipId::generate();
        let tenant_id = create_test_tenant_id();
        let org_id = create_test_org_id();
        let email = create_test_email();
        let now = Utc::now();

        let membership = Membership::load(
            id.clone(),
            tenant_id,
            org_id,
            None,
            None,
            email.clone(),
            MembershipRole::OrgAdmin,
            MembershipStatus::Active,
            now,
            now,
            Vec::new(),
        );

        assert_eq!(membership.id(), &id);
        assert_eq!(membership.status(), &MembershipStatus::Active);
        assert_eq!(membership.pending_events().len(), 0);
    }
}
