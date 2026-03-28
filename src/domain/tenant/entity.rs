//! 租户聚合根定义
//!
//! Tenant 是租户领域的聚合根，管理组织的生命周期和状态。

use crate::domain::common::{now, TenantStatus};
use chrono::{DateTime, Utc};

use super::value_object::{OrganizationId, TenantError, TenantId, TenantName, TenantSlug};

/// 租户聚合根
#[derive(Debug, Clone)]
pub struct Tenant {
    id: TenantId,
    name: TenantName,
    slug: TenantSlug,
    status: TenantStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    organizations: Vec<OrganizationId>,
}

impl Tenant {
    /// 创建新租户
    ///
    /// # Arguments
    /// * `name` - 租户名称
    /// * `slug` - 租户 slug (URL 友好的标识符)
    ///
    /// # Returns
    /// * `Result<Self, TenantError>` - 创建成功返回租户实例，失败返回错误
    pub fn create(name: String, slug: String) -> Result<Self, TenantError> {
        let now = now();
        let tenant = Self {
            id: TenantId::generate(),
            name: TenantName::new(name)?,
            slug: TenantSlug::new(slug)?,
            status: TenantStatus::Active,
            created_at: now,
            updated_at: now,
            organizations: Vec::new(),
        };

        Ok(tenant)
    }

    /// 从已有 ID 加载租户（用于从数据库加载）
    pub fn load(
        id: TenantId,
        name: TenantName,
        slug: TenantSlug,
        status: TenantStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        organizations: Vec<OrganizationId>,
    ) -> Self {
        Self {
            id,
            name,
            slug,
            status,
            created_at,
            updated_at,
            organizations,
        }
    }

    // ==================== Getter 方法 ====================

    /// 获取租户 ID
    pub fn id(&self) -> &TenantId {
        &self.id
    }

    /// 获取租户名称
    pub fn name(&self) -> &TenantName {
        &self.name
    }

    /// 获取租户 slug
    pub fn slug(&self) -> &TenantSlug {
        &self.slug
    }

    /// 获取租户状态
    pub fn status(&self) -> &TenantStatus {
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

    /// 获取组织列表
    pub fn organizations(&self) -> &[OrganizationId] {
        &self.organizations
    }

    // ==================== 业务方法 ====================

    /// 添加组织到租户
    ///
    /// # Arguments
    /// * `org_id` - 组织 ID
    pub fn add_organization(&mut self, org_id: OrganizationId) {
        if !self.organizations.contains(&org_id) {
            self.organizations.push(org_id);
            self.updated_at = now();
        }
    }

    /// 从租户移除组织
    ///
    /// # Arguments
    /// * `org_id` - 要移除的组织 ID
    pub fn remove_organization(&mut self, org_id: &OrganizationId) {
        if let Some(pos) = self.organizations.iter().position(|id| id == org_id) {
            self.organizations.remove(pos);
            self.updated_at = now();
        }
    }

    /// 暂停租户
    ///
    /// 暂停后，租户下的所有资源将被冻结，用户无法访问。
    pub fn suspend(&mut self) {
        if self.status == TenantStatus::Active {
            self.status = TenantStatus::Suspended;
            self.updated_at = now();
        }
    }

    /// 恢复租户
    ///
    /// 从暂停状态恢复到活跃状态。
    pub fn restore(&mut self) {
        if self.status == TenantStatus::Suspended {
            self.status = TenantStatus::Active;
            self.updated_at = now();
        }
    }

    /// 归档租户
    ///
    /// 归档后，租户数据将被保留但不可用，通常用于删除前的软删除状态。
    pub fn archive(&mut self) {
        if self.status != TenantStatus::Archived {
            self.status = TenantStatus::Archived;
            self.updated_at = now();
        }
    }

    /// 检查租户是否活跃
    pub fn is_active(&self) -> bool {
        self.status == TenantStatus::Active
    }

    /// 检查租户是否被暂停
    pub fn is_suspended(&self) -> bool {
        self.status == TenantStatus::Suspended
    }

    /// 检查租户是否已归档
    pub fn is_archived(&self) -> bool {
        self.status == TenantStatus::Archived
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tenant() -> Tenant {
        Tenant::create(
            "Test Tenant".to_string(),
            "test-tenant".to_string(),
        ).unwrap()
    }

    #[test]
    fn test_tenant_create() {
        let tenant = create_test_tenant();

        assert_eq!(tenant.name().as_str(), "Test Tenant");
        assert_eq!(tenant.slug().as_str(), "test-tenant");
        assert_eq!(tenant.status(), &TenantStatus::Active);
        assert!(tenant.organizations().is_empty());
    }

    #[test]
    fn test_tenant_create_invalid_name() {
        let result = Tenant::create("".to_string(), "test".to_string());
        assert!(matches!(result, Err(TenantError::InvalidName(_))));
    }

    #[test]
    fn test_tenant_create_invalid_slug() {
        let result = Tenant::create("Test".to_string(), "INVALID_SLUG".to_string());
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
    }

    #[test]
    fn test_add_organization() {
        let mut tenant = create_test_tenant();
        assert_eq!(tenant.organizations().len(), 0);

        let org_id = OrganizationId::generate();
        tenant.add_organization(org_id.clone());
        assert_eq!(tenant.organizations().len(), 1);
        assert!(tenant.organizations().contains(&org_id));

        // 添加重复的组织应该被忽略
        tenant.add_organization(org_id.clone());
        assert_eq!(tenant.organizations().len(), 1);
    }

    #[test]
    fn test_remove_organization() {
        let mut tenant = create_test_tenant();
        let org_id = OrganizationId::generate();
        tenant.add_organization(org_id.clone());
        assert_eq!(tenant.organizations().len(), 1);

        tenant.remove_organization(&org_id);
        assert_eq!(tenant.organizations().len(), 0);

        // 移除不存在的组织应该无效果
        tenant.remove_organization(&org_id);
        assert_eq!(tenant.organizations().len(), 0);
    }

    #[test]
    fn test_suspend() {
        let mut tenant = create_test_tenant();
        assert!(tenant.is_active());

        tenant.suspend();
        assert!(tenant.is_suspended());

        // 重复暂停应该无效果
        let _old_updated_at = tenant.updated_at;
        tenant.suspend();
        assert!(tenant.is_suspended());
    }

    #[test]
    fn test_restore() {
        let mut tenant = create_test_tenant();
        tenant.suspend();
        assert!(tenant.is_suspended());

        tenant.restore();
        assert!(tenant.is_active());

        // 对活跃租户恢复应该无效果
        let _old_updated_at = tenant.updated_at;
        tenant.restore();
        assert!(tenant.is_active());
    }

    #[test]
    fn test_archive() {
        let mut tenant = create_test_tenant();
        assert!(tenant.is_active());

        tenant.archive();
        assert!(tenant.is_archived());

        // 重复归档应该无效果
        tenant.archive();
        assert!(tenant.is_archived());

        // 已归档的租户不能恢复
        tenant.restore();
        assert!(tenant.is_archived());
    }

    #[test]
    fn test_tenant_state_transitions() {
        let mut tenant = create_test_tenant();

        // Active -> Suspended
        tenant.suspend();
        assert!(tenant.is_suspended());

        // Suspended -> Active
        tenant.restore();
        assert!(tenant.is_active());

        // Active -> Archived
        tenant.archive();
        assert!(tenant.is_archived());

        // Archived 状态不能转换到其他状态
        tenant.suspend();
        assert!(tenant.is_archived());

        tenant.restore();
        assert!(tenant.is_archived());
    }

    #[test]
    fn test_update_timestamp() {
        let mut tenant = create_test_tenant();
        let created_at = tenant.created_at;

        // 初始时 created_at 和 updated_at 应该相同（或非常接近）
        assert!(tenant.updated_at >= created_at);

        // 添加组织后 updated_at 应该更新
        std::thread::sleep(std::time::Duration::from_millis(1));
        tenant.add_organization(OrganizationId::generate());
        assert!(tenant.updated_at > created_at);
    }

    #[test]
    fn test_load() {
        let id = TenantId::generate();
        let name = TenantName::new("Loaded Tenant".to_string()).unwrap();
        let slug = TenantSlug::new("loaded-tenant".to_string()).unwrap();
        let org_id = OrganizationId::generate();

        let tenant = Tenant::load(
            id.clone(),
            name.clone(),
            slug.clone(),
            TenantStatus::Active,
            now(),
            now(),
            vec![org_id.clone()],
        );

        assert_eq!(tenant.id(), &id);
        assert_eq!(tenant.name(), &name);
        assert_eq!(tenant.slug(), &slug);
        assert_eq!(tenant.organizations().len(), 1);
    }
}
