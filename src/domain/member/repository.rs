//! 成员 Repository trait 定义
//!
//! 定义成员聚合的数据访问接口，由基础设施层实现。

use async_trait::async_trait;

use crate::domain::tenant::value_object::{
    MembershipId, OrganizationId, TeamId, UserId,
};

use super::entity::{MemberDomainError, Membership};

/// 成员查询过滤器
#[derive(Debug, Clone, Default)]
pub struct MembershipFilter {
    /// 按租户 ID 过滤
    pub tenant_id: Option<TenantId>,
    /// 按组织 ID 过滤
    pub organization_id: Option<OrganizationId>,
    /// 按团队 ID 过滤
    pub team_id: Option<TeamId>,
    /// 按用户 ID 过滤
    pub user_id: Option<UserId>,
    /// 按角色过滤
    pub role: Option<crate::domain::common::MembershipRole>,
    /// 按状态过滤
    pub status: Option<crate::domain::common::MembershipStatus>,
}

impl MembershipFilter {
    /// 创建新的过滤器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置租户 ID
    pub fn with_tenant_id(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// 设置组织 ID
    pub fn with_organization_id(mut self, organization_id: OrganizationId) -> Self {
        self.organization_id = Some(organization_id);
        self
    }

    /// 设置团队 ID
    pub fn with_team_id(mut self, team_id: TeamId) -> Self {
        self.team_id = Some(team_id);
        self
    }

    /// 设置用户 ID
    pub fn with_user_id(mut self, user_id: UserId) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// 设置角色
    pub fn with_role(mut self, role: crate::domain::common::MembershipRole) -> Self {
        self.role = Some(role);
        self
    }

    /// 设置状态
    pub fn with_status(mut self, status: crate::domain::common::MembershipStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// 检查过滤器是否为空
    pub fn is_empty(&self) -> bool {
        self.tenant_id.is_none()
            && self.organization_id.is_none()
            && self.team_id.is_none()
            && self.user_id.is_none()
            && self.role.is_none()
            && self.status.is_none()
    }
}

/// 成员 Repository trait
///
/// 定义了成员聚合的持久化接口，由基础设施层（如 SQLite）实现。
#[async_trait]
pub trait MembershipRepository: Send + Sync {
    /// 保存成员（创建或更新）
    ///
    /// 如果成员已存在则更新，否则创建新记录。
    async fn save(&self, membership: &Membership) -> Result<(), MemberDomainError>;

    /// 根据 ID 查找成员
    async fn find_by_id(&self, id: &MembershipId) -> Result<Option<Membership>, MemberDomainError>;

    /// 根据用户 ID 查找成员的所有成员资格
    async fn find_by_user(&self, user_id: &UserId) -> Result<Vec<Membership>, MemberDomainError>;

    /// 根据组织 ID 查找所有成员
    async fn find_by_organization(&self, org_id: &OrganizationId) -> Result<Vec<Membership>, MemberDomainError>;

    /// 根据团队 ID 查找所有成员
    async fn find_by_team(&self, team_id: &TeamId) -> Result<Vec<Membership>, MemberDomainError>;

    /// 根据租户 ID 查找所有成员
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Membership>, MemberDomainError>;

    /// 根据过滤器查找成员
    async fn find_by_filter(&self, filter: &MembershipFilter) -> Result<Vec<Membership>, MemberDomainError>;

    /// 删除成员
    async fn delete(&self, id: &MembershipId) -> Result<(), MemberDomainError>;

    /// 检查成员是否存在
    async fn exists(&self, id: &MembershipId) -> Result<bool, MemberDomainError> {
        match self.find_by_id(id).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// 统计成员数量
    async fn count(&self, filter: &MembershipFilter) -> Result<usize, MemberDomainError> {
        let members = self.find_by_filter(filter).await?;
        Ok(members.len())
    }
}

// 重新导出必要类型
pub use crate::domain::tenant::value_object::TenantId;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::common::{MembershipRole, MembershipStatus};

    #[test]
    fn test_membership_filter_builder() {
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let user_id = UserId::generate();

        let filter = MembershipFilter::new()
            .with_tenant_id(tenant_id.clone())
            .with_organization_id(org_id.clone())
            .with_user_id(user_id.clone())
            .with_role(MembershipRole::OrgAdmin)
            .with_status(MembershipStatus::Active);

        assert_eq!(filter.tenant_id, Some(tenant_id));
        assert_eq!(filter.organization_id, Some(org_id));
        assert_eq!(filter.user_id, Some(user_id));
        assert_eq!(filter.role, Some(MembershipRole::OrgAdmin));
        assert_eq!(filter.status, Some(MembershipStatus::Active));
    }

    #[test]
    fn test_membership_filter_is_empty() {
        let empty_filter = MembershipFilter::new();
        assert!(empty_filter.is_empty());

        let filter = MembershipFilter::new().with_tenant_id(TenantId::generate());
        assert!(!filter.is_empty());
    }
}
