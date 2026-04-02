//! RBAC 领域服务
//!
//! 提供 RBAC 权限检查服务，协调 Membership Repository 和 PermissionCheckService。

use std::sync::Arc;

use crate::domain::common::{MembershipRole, MembershipStatus, Permission};
use crate::domain::member::repository::MembershipRepository;
use crate::domain::service::member_service::PermissionCheckService;
use crate::domain::tenant::value_object::{MembershipId, OrganizationId, TenantId, UserId};

/// RBAC 错误类型
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RbacError {
    #[error("Membership not found: {0}")]
    MembershipNotFound(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Organization not found: {0}")]
    OrganizationNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource mismatch: {0}")]
    ResourceMismatch(String),

    #[error("Invalid role transition: {0}")]
    InvalidRoleTransition(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

/// RBAC 领域服务
///
/// 负责协调跨越多租户、组织、团队的权限检查，
/// 以及角色分配和权限验证等业务操作。
pub struct RbacService<MR> {
    membership_repo: Arc<MR>,
    permission_checker: PermissionCheckService,
}

impl<MR> RbacService<MR>
where
    MR: MembershipRepository + 'static,
{
    /// 创建新的 RBAC 服务实例
    ///
    /// # Arguments
    /// * `membership_repo` - Membership Repository
    pub fn new(membership_repo: Arc<MR>) -> Self {
        Self {
            membership_repo,
            permission_checker: PermissionCheckService::new(),
        }
    }

    /// 检查用户是否有指定权限
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `tenant_id` - 租户 ID
    /// * `permission` - 要检查的权限
    ///
    /// # Returns
    /// * `Result<bool, RbacError>` - 有权限返回 true，否则返回错误
    pub async fn check_permission(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        permission: &Permission,
    ) -> Result<bool, RbacError> {
        // 查找用户在该租户下的所有 membership
        let memberships = self
            .membership_repo
            .find_by_user(user_id)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?;

        // 检查是否有任何 membership 有该权限
        for membership in memberships {
            if membership.tenant_id() != tenant_id {
                continue;
            }

            // 检查成员状态
            if membership.status() != &MembershipStatus::Active {
                continue;
            }

            // 检查是否有该权限
            if membership.has_permission(permission) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 检查用户是否有指定组织的管理权限
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `tenant_id` - 租户 ID
    /// * `organization_id` - 组织 ID
    ///
    /// # Returns
    /// * `Result<bool, RbacError>` - 有管理权限返回 true
    pub async fn has_organization_admin_permission(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        organization_id: &OrganizationId,
    ) -> Result<bool, RbacError> {
        let memberships = self
            .membership_repo
            .find_by_user(user_id)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?;

        for membership in memberships {
            if membership.tenant_id() != tenant_id {
                continue;
            }

            if membership.organization_id() != organization_id {
                continue;
            }

            if membership.status() != &MembershipStatus::Active {
                continue;
            }

            // OrgAdmin 或 PlatformAdmin 有管理权限
            if membership.role() == &MembershipRole::OrgAdmin
                || membership.role() == &MembershipRole::PlatformAdmin
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 分配角色给成员
    ///
    /// # Arguments
    /// * `membership_id` - 成员 ID
    /// * `new_role` - 新角色
    /// * `assigner_id` - 分配者的用户 ID
    ///
    /// # Returns
    /// * `Result<(), RbacError>` - 成功返回 Ok
    pub async fn assign_role(
        &self,
        membership_id: &MembershipId,
        new_role: MembershipRole,
        assigner_id: &UserId,
    ) -> Result<(), RbacError> {
        // 查找要更新的 membership
        let mut membership = self
            .membership_repo
            .find_by_id(membership_id)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?
            .ok_or(RbacError::MembershipNotFound(membership_id.to_string()))?;

        // 验证分配者有权限（必须是 OrgAdmin 或 PlatformAdmin）
        let org_id = membership.organization_id();

        let has_permission = self
            .has_organization_admin_permission(assigner_id, membership.tenant_id(), org_id)
            .await?;

        if !has_permission {
            return Err(RbacError::PermissionDenied(
                "Only OrgAdmin or PlatformAdmin can assign roles".to_string(),
            ));
        }

        // 验证角色转换的合法性
        // PlatformAdmin 只能由其他 PlatformAdmin 分配
        if new_role == MembershipRole::PlatformAdmin {
            let memberships = self
                .membership_repo
                .find_by_user(assigner_id)
                .await
                .map_err(|e| RbacError::RepositoryError(e.to_string()))?;

            let assigner_is_platform_admin = memberships.iter().any(|m| {
                m.role() == &MembershipRole::PlatformAdmin
                    && m.status() == &MembershipStatus::Active
            });

            if !assigner_is_platform_admin {
                return Err(RbacError::InvalidRoleTransition(
                    "Only PlatformAdmin can assign PlatformAdmin role".to_string(),
                ));
            }
        }

        // 更新角色
        membership.change_role(new_role).map_err(|e| {
            RbacError::InvalidRoleTransition(format!("Failed to change role: {}", e))
        })?;

        // 保存
        self.membership_repo
            .save(&membership)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?;

        Ok(())
    }

    /// 检查资源访问权限
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `permission` - 要检查的权限
    /// * `tenant_id` - 租户 ID
    /// * `organization_id` - 组织 ID (可选)
    /// * `team_id` - 团队 ID (可选)
    ///
    /// # Returns
    /// * `Result<bool, RbacError>` - 有访问权限返回 true
    pub async fn check_resource_access(
        &self,
        user_id: &UserId,
        permission: &Permission,
        tenant_id: &TenantId,
        organization_id: Option<&OrganizationId>,
        team_id: Option<&crate::domain::tenant::value_object::TeamId>,
    ) -> Result<bool, RbacError> {
        let memberships = self
            .membership_repo
            .find_by_user(user_id)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?;

        for membership in memberships {
            if membership.tenant_id() != tenant_id {
                continue;
            }

            if membership.status() != &MembershipStatus::Active {
                continue;
            }

            // 检查组织匹配
            if let Some(org_id) = organization_id {
                if membership.organization_id() != org_id {
                    continue;
                }
            }

            // 检查团队匹配
            if let Some(team_id) = team_id {
                if membership.team_id() != Some(team_id) {
                    continue;
                }
            }

            // 检查权限
            if membership.has_permission(permission) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 获取用户在指定组织的有效角色
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    /// * `tenant_id` - 租户 ID
    /// * `organization_id` - 组织 ID
    ///
    /// # Returns
    /// * `Result<Option<MembershipRole>, RbacError>` - 返回用户的角色
    pub async fn get_effective_role(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        organization_id: &OrganizationId,
    ) -> Result<Option<MembershipRole>, RbacError> {
        let memberships = self
            .membership_repo
            .find_by_user(user_id)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?;

        for membership in memberships {
            if membership.tenant_id() != tenant_id {
                continue;
            }

            if membership.organization_id() != organization_id {
                continue;
            }

            if membership.status() != &MembershipStatus::Active {
                continue;
            }

            return Ok(Some(membership.role().clone()));
        }

        Ok(None)
    }

    /// 检查用户是否可以管理另一个成员
    ///
    /// # Arguments
    /// * `manager_user_id` - 管理者的用户 ID
    /// * `target_membership_id` - 目标成员的 ID
    ///
    /// # Returns
    /// * `Result<bool, RbacError>` - 可以管理返回 true
    pub async fn can_manage_member(
        &self,
        manager_user_id: &UserId,
        target_membership_id: &MembershipId,
    ) -> Result<bool, RbacError> {
        // 获取目标 membership
        let target_membership = self
            .membership_repo
            .find_by_id(target_membership_id)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?
            .ok_or(RbacError::MembershipNotFound(
                target_membership_id.to_string(),
            ))?;

        // 获取管理者的 membership
        let manager_memberships = self
            .membership_repo
            .find_by_user(manager_user_id)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?;

        for manager_membership in manager_memberships {
            if manager_membership.tenant_id() != target_membership.tenant_id() {
                continue;
            }

            if manager_membership.status() != &MembershipStatus::Active {
                continue;
            }

            // PlatformAdmin 可以管理所有成员
            if manager_membership.role() == &MembershipRole::PlatformAdmin {
                return Ok(true);
            }

            // OrgAdmin 只能管理同组织的成员
            if manager_membership.role() == &MembershipRole::OrgAdmin {
                if manager_membership.organization_id() == target_membership.organization_id() {
                    // 但是不能管理其他 OrgAdmin 或 PlatformAdmin
                    if target_membership.role() != &MembershipRole::OrgAdmin
                        && target_membership.role() != &MembershipRole::PlatformAdmin
                    {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// 获取用户可以访问的所有组织 ID
    ///
    /// # Arguments
    /// * `user_id` - 用户 ID
    ///
    /// # Returns
    /// * `Result<Vec<OrganizationId>, RbacError>` - 组织 ID 列表
    pub async fn get_accessible_organizations(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<OrganizationId>, RbacError> {
        let memberships = self
            .membership_repo
            .find_by_user(user_id)
            .await
            .map_err(|e| RbacError::RepositoryError(e.to_string()))?;

        let org_ids: Vec<OrganizationId> = memberships
            .iter()
            .filter(|m| m.status() == &MembershipStatus::Active)
            .map(|m| m.organization_id().clone())
            .collect();

        Ok(org_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::member::entity::Membership;
    use crate::domain::member::repository::{InMemoryMembershipRepository, MembershipRepository};
    use crate::domain::member::value_object::UserEmail;

    fn create_test_membership(
        tenant_id: TenantId,
        org_id: OrganizationId,
        role: MembershipRole,
        status: MembershipStatus,
    ) -> Membership {
        let email = UserEmail::new("test@example.com".to_string()).unwrap();
        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, role).unwrap();

        if status == MembershipStatus::Active {
            membership.accept_invite(UserId::generate()).unwrap();
        }

        membership
    }

    fn create_test_membership_for_user(
        tenant_id: TenantId,
        org_id: OrganizationId,
        role: MembershipRole,
        status: MembershipStatus,
        user_id: UserId,
    ) -> Membership {
        let email = UserEmail::new("test@example.com".to_string()).unwrap();
        let mut membership =
            Membership::invite(tenant_id, org_id, None, None, email, role).unwrap();

        if status == MembershipStatus::Active {
            membership.accept_invite(user_id).unwrap();
        }

        membership
    }

    fn create_service() -> (
        RbacService<InMemoryMembershipRepository>,
        Arc<InMemoryMembershipRepository>,
    ) {
        let repo = Arc::new(InMemoryMembershipRepository::new());
        let service = RbacService::new(repo.clone());
        (service, repo)
    }

    #[tokio::test]
    async fn test_check_permission_success() {
        let (service, repo) = create_service();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let user_id = UserId::generate();

        let membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::OrgAdmin,
            MembershipStatus::Active,
        );

        repo.save(&membership).await.unwrap();

        let result = service
            .check_permission(&user_id, &tenant_id, &Permission::OrgWrite)
            .await;

        // OrgAdmin 有 OrgWrite 权限
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_permission_inactive_member() {
        let (service, repo) = create_service();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let user_id = UserId::generate();

        let membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::Member,
            MembershipStatus::Pending,
        );

        repo.save(&membership).await.unwrap();

        let result = service
            .check_permission(&user_id, &tenant_id, &Permission::AgentRead)
            .await
            .unwrap();

        // Pending 状态没有权限
        assert!(!result);
    }

    #[tokio::test]
    async fn test_has_organization_admin_permission_org_admin() {
        let (service, repo) = create_service();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();

        let membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::OrgAdmin,
            MembershipStatus::Active,
        );
        let user_id = membership.user_id().clone().unwrap();

        repo.save(&membership).await.unwrap();

        let result = service
            .has_organization_admin_permission(&user_id, &tenant_id, &org_id)
            .await
            .unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_has_organization_admin_permission_member() {
        let (service, repo) = create_service();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let user_id = UserId::generate();

        let membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::Member,
            MembershipStatus::Active,
        );

        repo.save(&membership).await.unwrap();

        let result = service
            .has_organization_admin_permission(&user_id, &tenant_id, &org_id)
            .await
            .unwrap();

        assert!(!result);
    }

    #[tokio::test]
    async fn test_get_effective_role() {
        let (service, repo) = create_service();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();

        let membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::OrgAdmin,
            MembershipStatus::Active,
        );
        let user_id = membership.user_id().clone().unwrap();

        repo.save(&membership).await.unwrap();

        let result = service
            .get_effective_role(&user_id, &tenant_id, &org_id)
            .await
            .unwrap();

        assert_eq!(result, Some(MembershipRole::OrgAdmin));
    }

    #[tokio::test]
    async fn test_get_accessible_organizations() {
        let (service, repo) = create_service();
        let tenant_id = TenantId::generate();
        let org_id1 = OrganizationId::generate();
        let org_id2 = OrganizationId::generate();
        let user_id = UserId::generate();

        let membership1 = create_test_membership_for_user(
            tenant_id.clone(),
            org_id1.clone(),
            MembershipRole::Member,
            MembershipStatus::Active,
            user_id.clone(),
        );

        let membership2 = create_test_membership_for_user(
            tenant_id.clone(),
            org_id2.clone(),
            MembershipRole::Member,
            MembershipStatus::Active,
            user_id.clone(),
        );

        repo.save(&membership1).await.unwrap();
        repo.save(&membership2).await.unwrap();

        let orgs = service
            .get_accessible_organizations(&user_id)
            .await
            .unwrap();

        assert_eq!(orgs.len(), 2);
        assert!(orgs.contains(&org_id1));
        assert!(orgs.contains(&org_id2));
    }

    #[tokio::test]
    async fn test_can_manage_member_platform_admin() {
        let (service, repo) = create_service();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();

        // 创建 PlatformAdmin
        let platform_admin_membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::PlatformAdmin,
            MembershipStatus::Active,
        );
        let platform_admin_id = platform_admin_membership.user_id().clone().unwrap();
        repo.save(&platform_admin_membership).await.unwrap();

        // 创建普通成员
        let member_membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::Member,
            MembershipStatus::Active,
        );
        let member_id = member_membership.id().clone();
        repo.save(&member_membership).await.unwrap();

        // PlatformAdmin 可以管理普通成员
        let result = service
            .can_manage_member(&platform_admin_id, &member_id)
            .await
            .unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_can_manage_member_org_admin() {
        let (service, repo) = create_service();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();

        // 创建 OrgAdmin
        let org_admin_membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::OrgAdmin,
            MembershipStatus::Active,
        );
        let org_admin_id = org_admin_membership.user_id().clone().unwrap();
        repo.save(&org_admin_membership).await.unwrap();

        // 创建普通成员
        let member_membership = create_test_membership(
            tenant_id.clone(),
            org_id.clone(),
            MembershipRole::Member,
            MembershipStatus::Active,
        );
        let member_id = member_membership.id().clone();
        repo.save(&member_membership).await.unwrap();

        // OrgAdmin 可以管理同组织的普通成员
        let result = service
            .can_manage_member(&org_admin_id, &member_id)
            .await
            .unwrap();

        assert!(result);
    }
}
