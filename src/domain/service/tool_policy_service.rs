//! 工具策略服务
//!
//! 负责检查成员对工具的执行权限，管理工具策略。
//! 基于风险等级和成员角色进行权限控制。

use crate::domain::{
    common::{MembershipRole, MembershipStatus},
    member::{
        entity::Membership,
        value_object::{ToolId, ToolPolicy, ToolRiskLevel},
    },
};

/// 工具策略服务错误类型
#[derive(Debug, thiserror::Error)]
pub enum ToolPolicyError {
    #[error("工具执行被拒绝：{0}")]
    ExecutionDenied(String),

    #[error("工具需要审批：{0}")]
    ApprovalRequired(String),

    #[error("成员不存在：{0}")]
    MemberNotFound(String),

    #[error("工具不存在：{0}")]
    ToolNotFound(String),

    #[error("无效的工具风险等级：{0}")]
    InvalidRiskLevel(String),

    #[error("数据库错误：{0}")]
    DatabaseError(String),
}

/// 工具策略服务
///
/// 负责检查成员对工具的执行权限
pub struct ToolPolicyService {
    /// 平台默认最大风险等级
    platform_max_risk: ToolRiskLevel,
    /// 组织默认最大风险等级（可选，继承平台）
    org_max_risk: Option<ToolRiskLevel>,
    /// 团队默认最大风险等级（可选，继承组织/平台）
    team_max_risk: Option<ToolRiskLevel>,
}

impl ToolPolicyService {
    /// 创建新的工具策略服务
    pub fn new(
        platform_max_risk: ToolRiskLevel,
        org_max_risk: Option<ToolRiskLevel>,
        team_max_risk: Option<ToolRiskLevel>,
    ) -> Self {
        Self {
            platform_max_risk,
            org_max_risk,
            team_max_risk,
        }
    }

    /// 创建默认配置的工具策略服务
    /// - 平台：允许低风险工具
    /// - 组织：允许中风险工具
    /// - 团队：允许高风险工具
    pub fn default_config() -> Self {
        Self::new(
            ToolRiskLevel::Low,
            Some(ToolRiskLevel::Medium),
            Some(ToolRiskLevel::High),
        )
    }

    /// 检查成员是否可以执行指定风险等级的工具
    ///
    /// 权限检查逻辑：
    /// 1. 首先检查成员状态是否 Active
    /// 2. 检查角色是否有足够权限（PlatformAdmin 可以执行所有风险等级）
    /// 3. 检查成员的有效风险等级是否足够（PlatformAdmin 跳过此检查）
    /// 4. 检查成员的工具策略是否允许
    pub fn can_execute_tool(
        &self,
        membership: &Membership,
        tool_id: &ToolId,
        required_risk: ToolRiskLevel,
    ) -> Result<(), ToolPolicyError> {
        // 1. 检查成员状态
        if membership.status() != &MembershipStatus::Active {
            return Err(ToolPolicyError::ExecutionDenied(format!(
                "成员状态不是 Active，无法执行工具：{}",
                membership.id()
            )));
        }

        // 2. 检查角色权限（PlatformAdmin 可以执行所有风险等级）
        if !self.role_can_execute_risk(*membership.role(), required_risk) {
            return Err(ToolPolicyError::ExecutionDenied(format!(
                "角色权限不足：{:?} 无法执行风险等级为 {:?} 的工具",
                membership.role(),
                required_risk
            )));
        }

        // 3. 对于 PlatformAdmin，跳过有效风险等级检查
        if membership.role() != &MembershipRole::PlatformAdmin {
            let effective_risk = self.get_effective_risk_level(membership);
            if effective_risk < required_risk {
                return Err(ToolPolicyError::ExecutionDenied(format!(
                    "风险等级不足：成员有效风险等级为 {:?}，工具需要 {:?}",
                    effective_risk, required_risk
                )));
            }
        }

        // 4. 检查成员的工具策略
        if let Some(policy) = membership
            .tool_policies()
            .iter()
            .find(|p| p.tool_id() == tool_id)
        {
            if !policy.can_execute(required_risk) {
                return Err(ToolPolicyError::ExecutionDenied(format!(
                    "工具策略禁止执行：{}",
                    tool_id.as_str()
                )));
            }
        }

        Ok(())
    }

    /// 获取成员的有效风险等级
    ///
    /// 优先级：团队 > 组织 > 平台
    /// 但如果成员有显式工具策略，则以工具策略为准
    pub fn get_effective_risk_level(&self, membership: &Membership) -> ToolRiskLevel {
        // 如果成员有显式的工具策略覆盖，返回最高允许的风险等级
        // 这里简化实现，返回基于层级的风险等级

        // 优先级：团队 > 组织 > 平台
        if membership.team_id().is_some() {
            if let Some(team_risk) = self.team_max_risk {
                return team_risk;
            }
        }

        // organization_id 总是有值的，所以需要检查是否团队级成员
        // 如果没有团队但有组织，返回组织风险等级
        if self.team_max_risk.is_none() {
            if let Some(org_risk) = self.org_max_risk {
                return org_risk;
            }
        }

        self.platform_max_risk
    }

    /// 检查角色是否可以执行指定风险等级的工具
    fn role_can_execute_risk(&self, role: MembershipRole, risk: ToolRiskLevel) -> bool {
        match role {
            // PlatformAdmin 可以执行所有风险等级的工具
            MembershipRole::PlatformAdmin => true,

            // OrgAdmin 可以执行高及以下风险等级的工具
            MembershipRole::OrgAdmin => risk <= ToolRiskLevel::High,

            // TeamAdmin 可以执行中及以下风险等级的工具
            MembershipRole::TeamAdmin => risk <= ToolRiskLevel::Medium,

            // Member 和 Viewer 只能执行低风险工具
            MembershipRole::Member | MembershipRole::Viewer => risk <= ToolRiskLevel::Low,
        }
    }

    /// 为成员创建默认工具策略
    ///
    /// 根据成员角色分配默认的工具权限
    pub fn create_default_tool_policy(&self, role: MembershipRole) -> Vec<ToolPolicy> {
        match role {
            MembershipRole::PlatformAdmin => {
                // 平台管理员可以执行所有工具
                vec![
                    ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::Critical, true),
                    ToolPolicy::new(ToolId::from_str("file_write"), ToolRiskLevel::High, true),
                    ToolPolicy::new(ToolId::from_str("file_read"), ToolRiskLevel::Low, true),
                    ToolPolicy::new(ToolId::from_str("search"), ToolRiskLevel::Low, true),
                ]
            }
            MembershipRole::OrgAdmin => {
                // 组织管理员可以执行高及以下风险工具
                vec![
                    ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::High, true),
                    ToolPolicy::new(ToolId::from_str("file_write"), ToolRiskLevel::High, true),
                    ToolPolicy::new(ToolId::from_str("file_read"), ToolRiskLevel::Low, true),
                    ToolPolicy::new(ToolId::from_str("search"), ToolRiskLevel::Low, true),
                ]
            }
            MembershipRole::TeamAdmin => {
                // 团队管理员可以执行中及以下风险工具
                vec![
                    ToolPolicy::new(ToolId::from_str("file_write"), ToolRiskLevel::Medium, true),
                    ToolPolicy::new(ToolId::from_str("file_read"), ToolRiskLevel::Low, true),
                    ToolPolicy::new(ToolId::from_str("search"), ToolRiskLevel::Low, true),
                ]
            }
            MembershipRole::Member | MembershipRole::Viewer => {
                // 普通成员只能执行低风险工具
                vec![
                    ToolPolicy::new(ToolId::from_str("file_read"), ToolRiskLevel::Low, true),
                    ToolPolicy::new(ToolId::from_str("search"), ToolRiskLevel::Low, true),
                ]
            }
        }
    }

    /// 检查工具是否需要审批
    ///
    /// 高风险工具需要审批
    pub fn requires_approval(&self, _tool_id: &ToolId, risk: ToolRiskLevel) -> bool {
        matches!(risk, ToolRiskLevel::Critical)
    }

    /// 获取工具的风险等级
    ///
    /// 实际应用中应该从工具元数据中获取
    pub fn get_tool_risk_level(&self, tool_id: &ToolId) -> Option<ToolRiskLevel> {
        // 这里提供一个简单的映射表，实际应该从工具注册表中获取
        match tool_id.as_str() {
            "file_read" | "search" | "grep" | "glob" => Some(ToolRiskLevel::Low),
            "file_write" | "file_edit" | "mkdir" => Some(ToolRiskLevel::Medium),
            "shell" | "git" | "npm" | "cargo" => Some(ToolRiskLevel::High),
            "rm_rf" | "drop_table" | "dangerous_command" => Some(ToolRiskLevel::Critical),
            _ => None, // 未知工具
        }
    }

    /// 合并多个工具策略
    ///
    /// 取最严格的策略（最小权限原则）
    pub fn merge_policies(policies: &[ToolPolicy]) -> ToolRiskLevel {
        if policies.is_empty() {
            return ToolRiskLevel::Low;
        }

        // 返回所有策略中最低的风险等级
        policies
            .iter()
            .filter(|p| p.is_allowed())
            .map(|p| p.risk_level())
            .min()
            .unwrap_or(ToolRiskLevel::Low)
    }
}

/// 辅助工具：用于构建工具策略
pub struct ToolPolicyBuilder {
    policies: Vec<ToolPolicy>,
}

impl ToolPolicyBuilder {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    pub fn allow_tool(mut self, tool_id: &str, risk: ToolRiskLevel) -> Self {
        self.policies
            .push(ToolPolicy::new(ToolId::from_str(tool_id), risk, true));
        self
    }

    pub fn deny_tool(mut self, tool_id: &str, risk: ToolRiskLevel) -> Self {
        self.policies
            .push(ToolPolicy::new(ToolId::from_str(tool_id), risk, false));
        self
    }

    pub fn build(self) -> Vec<ToolPolicy> {
        self.policies
    }
}

impl Default for ToolPolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        common::{MembershipRole, MembershipStatus},
        member::value_object::UserEmail,
        tenant::value_object::{OrganizationId, TeamId, TenantId, UserId},
    };

    fn create_test_membership(role: MembershipRole, status: MembershipStatus) -> Membership {
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let team_id = TeamId::generate();
        let user_id = UserId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        let mut membership = Membership::invite(
            tenant_id,
            org_id,
            Some(team_id),
            None, // Initially no user_id
            email,
            role,
        )
        .unwrap();

        // 设置状态
        if status == MembershipStatus::Active {
            membership.accept_invite(user_id).unwrap();
        } else if status == MembershipStatus::Suspended {
            membership.accept_invite(user_id).unwrap();
            let _ = membership.suspend("测试暂停".to_string());
        }

        membership
    }

    #[test]
    fn test_can_execute_tool_active_member() {
        let service = ToolPolicyService::default_config();
        let membership = create_test_membership(MembershipRole::OrgAdmin, MembershipStatus::Active);
        let tool_id = ToolId::from_str("file_read");

        // OrgAdmin 应该可以执行低风险工具
        let result = service.can_execute_tool(&membership, &tool_id, ToolRiskLevel::Low);
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_execute_tool_inactive_member() {
        let service = ToolPolicyService::default_config();
        let membership =
            create_test_membership(MembershipRole::OrgAdmin, MembershipStatus::Suspended);
        let tool_id = ToolId::from_str("file_read");

        // Suspended 成员不能执行工具
        let result = service.can_execute_tool(&membership, &tool_id, ToolRiskLevel::Low);
        assert!(result.is_err());
        assert!(matches!(result, Err(ToolPolicyError::ExecutionDenied(_))));
    }

    #[test]
    fn test_can_execute_tool_insufficient_risk() {
        let service = ToolPolicyService::default_config();
        let membership = create_test_membership(MembershipRole::Member, MembershipStatus::Active);
        let tool_id = ToolId::from_str("shell");

        // Member 只能执行低风险工具，不能执行高风险工具
        let result = service.can_execute_tool(&membership, &tool_id, ToolRiskLevel::High);
        assert!(result.is_err());
        assert!(matches!(result, Err(ToolPolicyError::ExecutionDenied(_))));
    }

    #[test]
    fn test_can_execute_tool_platform_admin() {
        let service = ToolPolicyService::default_config();
        let membership =
            create_test_membership(MembershipRole::PlatformAdmin, MembershipStatus::Active);
        let tool_id = ToolId::from_str("rm_rf");

        // PlatformAdmin 应该可以执行所有风险等级的工具
        // 默认配置：平台 Low, 组织 Medium, 团队 High
        // PlatformAdmin 的 role_can_execute_risk 应该返回 true 对于所有风险等级
        let result = service.can_execute_tool(&membership, &tool_id, ToolRiskLevel::Critical);

        // 打印错误信息以便调试
        if let Err(e) = &result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_effective_risk_level_tenant() {
        let service = ToolPolicyService::new(
            ToolRiskLevel::Low,
            Some(ToolRiskLevel::Medium),
            Some(ToolRiskLevel::High),
        );

        // 创建有团队的成员
        let membership = create_test_membership(MembershipRole::Member, MembershipStatus::Active);

        // 团队成员应该获得团队风险等级
        let risk = service.get_effective_risk_level(&membership);
        assert_eq!(risk, ToolRiskLevel::High);
    }

    #[test]
    fn test_get_effective_risk_level_no_team() {
        let service = ToolPolicyService::new(
            ToolRiskLevel::Low,
            Some(ToolRiskLevel::Medium),
            None, // 没有团队风险等级
        );

        // 创建没有团队的成员
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let user_id = UserId::generate();
        let email = UserEmail::new("test@example.com".to_string()).unwrap();

        let mut membership = Membership::invite(
            tenant_id,
            org_id,
            None, // 没有团队
            None, // Initially no user_id
            email,
            MembershipRole::Member,
        )
        .unwrap();
        membership.accept_invite(user_id).unwrap();

        // 组织成员应该获得组织风险等级
        let risk = service.get_effective_risk_level(&membership);
        assert_eq!(risk, ToolRiskLevel::Medium);
    }

    #[test]
    fn test_get_effective_risk_level_platform_only() {
        let service = ToolPolicyService::new(
            ToolRiskLevel::Medium, // 平台默认中风险
            None,
            None,
        );

        let membership = create_test_membership(MembershipRole::Member, MembershipStatus::Active);

        // 没有其他配置时，使用平台风险等级
        let risk = service.get_effective_risk_level(&membership);
        assert_eq!(risk, ToolRiskLevel::Medium);
    }

    #[test]
    fn test_role_can_execute_risk() {
        let service = ToolPolicyService::default_config();

        // PlatformAdmin 可以执行所有风险
        assert!(service.role_can_execute_risk(MembershipRole::PlatformAdmin, ToolRiskLevel::Low));
        assert!(
            service.role_can_execute_risk(MembershipRole::PlatformAdmin, ToolRiskLevel::Critical)
        );

        // OrgAdmin 可以执行 High 及以下
        assert!(service.role_can_execute_risk(MembershipRole::OrgAdmin, ToolRiskLevel::High));
        assert!(!service.role_can_execute_risk(MembershipRole::OrgAdmin, ToolRiskLevel::Critical));

        // TeamAdmin 可以执行 Medium 及以下
        assert!(service.role_can_execute_risk(MembershipRole::TeamAdmin, ToolRiskLevel::Medium));
        assert!(!service.role_can_execute_risk(MembershipRole::TeamAdmin, ToolRiskLevel::High));

        // Member 只能执行 Low
        assert!(service.role_can_execute_risk(MembershipRole::Member, ToolRiskLevel::Low));
        assert!(!service.role_can_execute_risk(MembershipRole::Member, ToolRiskLevel::Medium));
    }

    #[test]
    fn test_create_default_tool_policy() {
        let service = ToolPolicyService::default_config();

        // OrgAdmin 的策略
        let policies = service.create_default_tool_policy(MembershipRole::OrgAdmin);
        assert!(!policies.is_empty());

        // 应该包含 shell 工具
        let shell_policy = policies.iter().find(|p| p.tool_id().as_str() == "shell");
        assert!(shell_policy.is_some());
        assert_eq!(shell_policy.unwrap().risk_level(), ToolRiskLevel::High);
    }

    #[test]
    fn test_requires_approval() {
        let service = ToolPolicyService::default_config();

        // Critical 风险需要审批
        assert!(service.requires_approval(&ToolId::from_str("rm_rf"), ToolRiskLevel::Critical));

        // High 及以下不需要审批
        assert!(!service.requires_approval(&ToolId::from_str("shell"), ToolRiskLevel::High));
    }

    #[test]
    fn test_get_tool_risk_level() {
        let service = ToolPolicyService::default_config();

        // 低风险工具
        assert_eq!(
            service.get_tool_risk_level(&ToolId::from_str("file_read")),
            Some(ToolRiskLevel::Low)
        );

        // 中风险工具
        assert_eq!(
            service.get_tool_risk_level(&ToolId::from_str("file_write")),
            Some(ToolRiskLevel::Medium)
        );

        // 高风险工具
        assert_eq!(
            service.get_tool_risk_level(&ToolId::from_str("shell")),
            Some(ToolRiskLevel::High)
        );

        // 严重风险工具
        assert_eq!(
            service.get_tool_risk_level(&ToolId::from_str("rm_rf")),
            Some(ToolRiskLevel::Critical)
        );

        // 未知工具
        assert_eq!(
            service.get_tool_risk_level(&ToolId::from_str("unknown_tool")),
            None
        );
    }

    #[test]
    fn test_merge_policies() {
        let policies = vec![
            ToolPolicy::new(ToolId::from_str("tool1"), ToolRiskLevel::High, true),
            ToolPolicy::new(ToolId::from_str("tool2"), ToolRiskLevel::Medium, true),
            ToolPolicy::new(ToolId::from_str("tool3"), ToolRiskLevel::Low, true),
        ];

        // 合并后应该返回最低风险等级
        let merged = ToolPolicyService::merge_policies(&policies);
        assert_eq!(merged, ToolRiskLevel::Low);
    }

    #[test]
    fn test_merge_policies_empty() {
        let policies: Vec<ToolPolicy> = vec![];
        let merged = ToolPolicyService::merge_policies(&policies);
        assert_eq!(merged, ToolRiskLevel::Low);
    }

    #[test]
    fn test_tool_policy_builder() {
        let policies = ToolPolicyBuilder::new()
            .allow_tool("file_read", ToolRiskLevel::Low)
            .allow_tool("shell", ToolRiskLevel::High)
            .deny_tool("rm_rf", ToolRiskLevel::Critical)
            .build();

        assert_eq!(policies.len(), 3);

        // 验证第一个策略
        assert_eq!(policies[0].tool_id().as_str(), "file_read");
        assert!(policies[0].is_allowed());
        assert_eq!(policies[0].risk_level(), ToolRiskLevel::Low);

        // 验证第三个策略（deny）
        assert_eq!(policies[2].tool_id().as_str(), "rm_rf");
        assert!(!policies[2].is_allowed());
    }
}
