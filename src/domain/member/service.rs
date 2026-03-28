//! 成员领域服务
//!
//! 提供成员相关的领域服务，包括权限检查等。

use crate::domain::common::Permission;

use super::entity::Membership;
use super::value_object::{ToolId, ToolRiskLevel};

/// 权限检查错误详情
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionError {
    /// 成员状态不活跃
    InactiveMember,
    /// 角色权限不足
    InsufficientRole,
    /// 工具策略拒绝
    ToolPolicyDenied,
    /// 资源不匹配（租户/组织/团队）
    ResourceMismatch,
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InactiveMember => write!(f, "成员状态不活跃"),
            Self::InsufficientRole => write!(f, "角色权限不足"),
            Self::ToolPolicyDenied => write!(f, "工具策略拒绝"),
            Self::ResourceMismatch => write!(f, "资源不匹配"),
        }
    }
}

/// 成员领域服务
pub struct MemberDomainService;

impl MemberDomainService {
    /// 创建新的领域服务实例
    pub fn new() -> Self {
        Self
    }

    /// 检查成员是否有指定权限
    ///
    /// 检查逻辑：
    /// 1. 成员状态必须是 Active
    /// 2. 如果是工具执行权限，检查工具策略
    /// 3. 对于其他权限，检查角色权限
    pub fn check_permission(
        &self,
        membership: &Membership,
        permission: &Permission,
    ) -> Result<(), PermissionError> {
        // 检查成员状态
        if membership.status() != &crate::domain::common::MembershipStatus::Active {
            return Err(PermissionError::InactiveMember);
        }

        // 如果是工具执行权限，直接检查工具策略（不检查角色权限）
        if let Permission::ToolExecute(tool_name) = permission {
            let tool_id = ToolId::from_str(tool_name.as_str());
            // 默认工具风险等级为 Medium，具体风险等级应由调用方指定
            // 这里使用保守策略，要求至少 Medium 权限
            if !membership.can_execute_tool(&tool_id, ToolRiskLevel::Medium) {
                return Err(PermissionError::ToolPolicyDenied);
            }
            return Ok(());
        }

        // 检查角色权限
        if !membership.has_permission(permission) {
            return Err(PermissionError::InsufficientRole);
        }

        Ok(())
    }

    /// 检查成员是否可以执行指定工具
    ///
    /// 这是 check_permission 的简化版本，专门用于工具执行检查
    pub fn can_execute_tool(
        &self,
        membership: &Membership,
        tool_id: &ToolId,
        risk_level: ToolRiskLevel,
    ) -> bool {
        membership.can_execute_tool(tool_id, risk_level)
    }

    /// 检查成员是否匹配指定资源
    ///
    /// 验证成员的 tenant_id, organization_id, team_id 是否与给定资源匹配
    pub fn check_resource_match(
        &self,
        membership: &Membership,
        tenant_id: Option<&crate::domain::tenant::value_object::TenantId>,
        organization_id: Option<&crate::domain::tenant::value_object::OrganizationId>,
        team_id: Option<&crate::domain::tenant::value_object::TeamId>,
    ) -> Result<(), PermissionError> {
        // 检查租户匹配
        if let Some(tid) = tenant_id {
            if membership.tenant_id() != tid {
                return Err(PermissionError::ResourceMismatch);
            }
        }

        // 检查组织匹配
        if let Some(oid) = organization_id {
            if membership.organization_id() != oid {
                return Err(PermissionError::ResourceMismatch);
            }
        }

        // 检查团队匹配（仅当成员是团队级且提供了团队 ID 时）
        if let Some(tmid) = team_id {
            if let Some(member_team_id) = membership.team_id() {
                if member_team_id != tmid {
                    return Err(PermissionError::ResourceMismatch);
                }
            }
            // 如果成员是组织级但没有提供团队 ID，也算匹配
        }

        Ok(())
    }

    /// 获取成员对工具的有效风险等级
    ///
    /// 返回成员可以执行的最高风险等级
    pub fn get_effective_risk_level(
        &self,
        membership: &Membership,
        tool_id: &ToolId,
    ) -> ToolRiskLevel {
        // 查找是否有明确的工具策略
        if let Some(policy) = membership.tool_policies().iter().find(|p| p.tool_id() == tool_id) {
            if policy.is_allowed() {
                return policy.risk_level();
            } else {
                return ToolRiskLevel::Low; // 被明确禁止，返回最低
            }
        }

        // 没有明确策略时，根据角色返回默认风险等级
        match membership.role() {
            crate::domain::common::MembershipRole::PlatformAdmin => ToolRiskLevel::Critical,
            crate::domain::common::MembershipRole::OrgAdmin => ToolRiskLevel::High,
            crate::domain::common::MembershipRole::TeamAdmin => ToolRiskLevel::Medium,
            crate::domain::common::MembershipRole::Member => ToolRiskLevel::Low,
            crate::domain::common::MembershipRole::Viewer => ToolRiskLevel::Low, // Viewer 不允许执行
        }
    }

    /// 检查成员是否是管理员级别
    pub fn is_admin(&self, membership: &Membership) -> bool {
        matches!(
            membership.role(),
            crate::domain::common::MembershipRole::PlatformAdmin
                | crate::domain::common::MembershipRole::OrgAdmin
                | crate::domain::common::MembershipRole::TeamAdmin,
        )
    }

    /// 检查成员是否是平台管理员
    pub fn is_platform_admin(&self, membership: &Membership) -> bool {
        matches!(
            membership.role(),
            crate::domain::common::MembershipRole::PlatformAdmin
        )
    }

    /// 检查成员是否可以管理其他成员
    ///
    /// 规则：
    /// - PlatformAdmin 可以管理所有成员
    /// - OrgAdmin 可以管理组织级成员和团队级成员
    /// - TeamAdmin 只能管理同团队的成员
    pub fn can_manage_member(
        &self,
        manager: &Membership,
        target: &Membership,
    ) -> bool {
        // 检查资源匹配（同一租户/组织）
        if manager.tenant_id() != target.tenant_id()
            || manager.organization_id() != target.organization_id()
        {
            return false;
        }

        match manager.role() {
            crate::domain::common::MembershipRole::PlatformAdmin => true,
            crate::domain::common::MembershipRole::OrgAdmin => true,
            crate::domain::common::MembershipRole::TeamAdmin => {
                // TeamAdmin 只能管理同团队的成员
                manager.team_id() == target.team_id()
            }
            _ => false,
        }
    }
}

impl Default for MemberDomainService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::common::{MembershipRole, MembershipStatus};
    use crate::domain::member::entity::Membership;
    use crate::domain::member::value_object::{ToolId, ToolPolicy, ToolRiskLevel, UserEmail};
    use crate::domain::tenant::value_object::{
        MembershipId, OrganizationId, TeamId, TenantId, UserId,
    };

    fn create_test_membership(
        role: MembershipRole,
        status: MembershipStatus,
    ) -> Membership {
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        let mut membership = Membership::invite(
            tenant_id,
            org_id,
            None,
            None,
            email,
            role,
        ).unwrap();

        if status == crate::domain::common::MembershipStatus::Active {
            membership.accept_invite(UserId::generate()).unwrap();
        }

        membership
    }

    #[test]
    fn test_check_permission_active_member() {
        let service = MemberDomainService::new();
        let membership = create_test_membership(MembershipRole::Member, MembershipStatus::Active);

        // Member 有 AgentRead 权限
        let result = service.check_permission(&membership, &Permission::AgentRead);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_permission_inactive_member() {
        let service = MemberDomainService::new();
        let membership = create_test_membership(MembershipRole::Member, MembershipStatus::Pending);

        // Pending 状态的成员没有权限
        let result = service.check_permission(&membership, &Permission::AgentRead);
        assert_eq!(result, Err(PermissionError::InactiveMember));
    }

    #[test]
    fn test_check_permission_insufficient_role() {
        let service = MemberDomainService::new();
        let membership = create_test_membership(MembershipRole::Member, MembershipStatus::Active);

        // Member 没有 OrgWrite 权限
        let result = service.check_permission(&membership, &Permission::OrgWrite);
        assert_eq!(result, Err(PermissionError::InsufficientRole));
    }

    #[test]
    fn test_check_permission_tool_execute() {
        let service = MemberDomainService::new();

        // 使用 OrgAdmin 角色，因为有更高的默认权限
        let membership = create_test_membership(MembershipRole::OrgAdmin, MembershipStatus::Active);

        // 添加 shell 工具策略（Medium 风险）
        let policy = ToolPolicy::new(
            ToolId::from_str("shell"),
            ToolRiskLevel::Medium,
            true,
        );

        let mut membership = membership;
        membership.add_tool_policy(policy).unwrap();

        // 可以执行 shell 工具
        let result = service.check_permission(&membership, &Permission::ToolExecute("shell".to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_execute_tool() {
        let service = MemberDomainService::new();
        let membership = create_test_membership(MembershipRole::Member, MembershipStatus::Active);

        let tool_id = ToolId::from_str("shell");

        // 没有策略时，Member 只能执行 Low 风险
        assert!(!service.can_execute_tool(&membership, &tool_id, ToolRiskLevel::Medium));
        assert!(service.can_execute_tool(&membership, &tool_id, ToolRiskLevel::Low));
    }

    #[test]
    fn test_check_resource_match() {
        let service = MemberDomainService::new();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        let mut membership = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email,
            MembershipRole::Member,
        ).unwrap();
        membership.accept_invite(UserId::generate()).unwrap();

        // 匹配同一租户和组织
        let result = service.check_resource_match(
            &membership,
            Some(&tenant_id),
            Some(&org_id),
            None,
        );
        assert!(result.is_ok());

        // 不匹配的租户
        let wrong_tenant = TenantId::generate();
        let result = service.check_resource_match(
            &membership,
            Some(&wrong_tenant),
            Some(&org_id),
            None,
        );
        assert_eq!(result, Err(PermissionError::ResourceMismatch));
    }

    #[test]
    fn test_get_effective_risk_level() {
        let service = MemberDomainService::new();

        // OrgAdmin 默认 High 风险
        let membership = create_test_membership(MembershipRole::OrgAdmin, MembershipStatus::Active);
        let tool_id = ToolId::from_str("unknown_tool");
        assert_eq!(service.get_effective_risk_level(&membership, &tool_id), ToolRiskLevel::High);

        // 添加策略覆盖
        let policy = ToolPolicy::new(
            ToolId::from_str("unknown_tool"),
            ToolRiskLevel::Medium,
            true,
        );
        let mut membership = create_test_membership(MembershipRole::OrgAdmin, MembershipStatus::Active);
        membership.add_tool_policy(policy).unwrap();

        assert_eq!(service.get_effective_risk_level(&membership, &tool_id), ToolRiskLevel::Medium);
    }

    #[test]
    fn test_is_admin() {
        let service = MemberDomainService::new();

        let admin = create_test_membership(MembershipRole::OrgAdmin, MembershipStatus::Active);
        assert!(service.is_admin(&admin));

        let member = create_test_membership(MembershipRole::Member, MembershipStatus::Active);
        assert!(!service.is_admin(&member));
    }

    #[test]
    fn test_is_platform_admin() {
        let service = MemberDomainService::new();

        let platform_admin = create_test_membership(MembershipRole::PlatformAdmin, MembershipStatus::Active);
        assert!(service.is_platform_admin(&platform_admin));

        let org_admin = create_test_membership(MembershipRole::OrgAdmin, MembershipStatus::Active);
        assert!(!service.is_platform_admin(&org_admin));
    }

    #[test]
    fn test_can_manage_member() {
        let service = MemberDomainService::new();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let email1 = UserEmail::new("admin@example.com".to_string()).unwrap();
        let email2 = UserEmail::new("member@example.com".to_string()).unwrap();

        // 创建组织管理员
        let mut admin = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email1,
            MembershipRole::OrgAdmin,
        ).unwrap();
        admin.accept_invite(UserId::generate()).unwrap();

        // 创建同组织成员
        let mut member = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            None,
            None,
            email2,
            MembershipRole::Member,
        ).unwrap();
        member.accept_invite(UserId::generate()).unwrap();

        // 组织管理员可以管理成员
        assert!(service.can_manage_member(&admin, &member));

        // 创建不同组织的成员
        let other_org_id = OrganizationId::generate();
        let email3 = UserEmail::new("other@example.com".to_string()).unwrap();
        let mut other_member = Membership::invite(
            tenant_id.clone(),
            other_org_id,
            None,
            None,
            email3,
            MembershipRole::Member,
        ).unwrap();
        other_member.accept_invite(UserId::generate()).unwrap();

        // 不能管理其他组织的成员
        assert!(!service.can_manage_member(&admin, &other_member));
    }

    #[test]
    fn test_team_admin_can_manage_member() {
        let service = MemberDomainService::new();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let team_id = TeamId::generate();
        let other_team_id = TeamId::generate();

        let email1 = UserEmail::new("team_admin@example.com".to_string()).unwrap();
        let email2 = UserEmail::new("team_member@example.com".to_string()).unwrap();
        let email3 = UserEmail::new("other_team@example.com".to_string()).unwrap();

        // 创建团队管理员
        let mut team_admin = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            Some(team_id.clone()),
            None,
            email1,
            MembershipRole::TeamAdmin,
        ).unwrap();
        team_admin.accept_invite(UserId::generate()).unwrap();

        // 创建同团队成员
        let mut team_member = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            Some(team_id.clone()),
            None,
            email2,
            MembershipRole::Member,
        ).unwrap();
        team_member.accept_invite(UserId::generate()).unwrap();

        // 创建不同团队成员
        let mut other_team_member = Membership::invite(
            tenant_id.clone(),
            org_id.clone(),
            Some(other_team_id),
            None,
            email3,
            MembershipRole::Member,
        ).unwrap();
        other_team_member.accept_invite(UserId::generate()).unwrap();

        // TeamAdmin 可以管理同团队成员
        assert!(service.can_manage_member(&team_admin, &team_member));

        // TeamAdmin 不能管理不同团队成员
        assert!(!service.can_manage_member(&team_admin, &other_team_member));
    }
}
