//! Organization 聚合根定义
//!
//! Organization 是组织领域的聚合根，管理组织的生命周期和状态。

use crate::domain::common::now;
use chrono::{DateTime, Utc};

use super::value_object::{
    OrganizationError, OrganizationId, OrganizationName, OrganizationSlug,
};
use crate::domain::tenant::TenantId;

/// 组织聚合根
#[derive(Debug, Clone)]
pub struct Organization {
    id: OrganizationId,
    tenant_id: TenantId,
    name: OrganizationName,
    slug: OrganizationSlug,
    industry: Option<String>,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Organization {
    /// 创建新组织
    ///
    /// # Arguments
    /// * `tenant_id` - 所属租户 ID
    /// * `name` - 组织名称
    /// * `slug` - 组织 slug (URL 友好的标识符)
    ///
    /// # Returns
    /// * `Result<Self, OrganizationError>` - 创建成功返回组织实例，失败返回错误
    pub fn create(
        tenant_id: TenantId,
        name: String,
        slug: String,
    ) -> Result<Self, OrganizationError> {
        let now = now();
        let organization = Self {
            id: OrganizationId::generate(),
            tenant_id,
            name: OrganizationName::new(name)?,
            slug: OrganizationSlug::new(slug)?,
            industry: None,
            description: None,
            created_at: now,
            updated_at: now,
        };

        Ok(organization)
    }

    /// 从已有数据加载组织（用于从数据库加载）
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        id: OrganizationId,
        tenant_id: TenantId,
        name: OrganizationName,
        slug: OrganizationSlug,
        industry: Option<String>,
        description: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            name,
            slug,
            industry,
            description,
            created_at,
            updated_at,
        }
    }

    // ==================== Getter 方法 ====================

    /// 获取组织 ID
    pub fn id(&self) -> &OrganizationId {
        &self.id
    }

    /// 获取所属租户 ID
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    /// 获取组织名称
    pub fn name(&self) -> &OrganizationName {
        &self.name
    }

    /// 获取组织 slug
    pub fn slug(&self) -> &OrganizationSlug {
        &self.slug
    }

    /// 获取所属行业
    pub fn industry(&self) -> Option<&String> {
        self.industry.as_ref()
    }

    /// 获取描述
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    /// 获取创建时间
    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    /// 获取更新时间
    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }

    // ==================== 业务方法 ====================

    /// 更新组织名称
    ///
    /// # Arguments
    /// * `name` - 新的组织名称
    pub fn update_name(&mut self, name: String) -> Result<(), OrganizationError> {
        self.name = OrganizationName::new(name)?;
        self.updated_at = now();
        Ok(())
    }

    /// 更新组织 slug
    ///
    /// # Arguments
    /// * `slug` - 新的组织 slug
    pub fn update_slug(&mut self, slug: String) -> Result<(), OrganizationError> {
        self.slug = OrganizationSlug::new(slug)?;
        self.updated_at = now();
        Ok(())
    }

    /// 设置所属行业
    ///
    /// # Arguments
    /// * `industry` - 所属行业
    pub fn set_industry(&mut self, industry: Option<String>) {
        self.industry = industry;
        self.updated_at = now();
    }

    /// 设置描述
    ///
    /// # Arguments
    /// * `description` - 描述
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_organization() -> Organization {
        Organization::create(
            TenantId::generate(),
            "Test Organization".to_string(),
            "test-org".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn test_organization_create() {
        let tenant_id = TenantId::generate();
        let org = Organization::create(
            tenant_id.clone(),
            "Test Organization".to_string(),
            "test-org".to_string(),
        )
        .unwrap();

        assert_eq!(org.name().as_str(), "Test Organization");
        assert_eq!(org.slug().as_str(), "test-org");
        assert_eq!(org.tenant_id(), &tenant_id);
        assert!(org.industry().is_none());
        assert!(org.description().is_none());
    }

    #[test]
    fn test_organization_create_invalid_name() {
        let result = Organization::create(
            TenantId::generate(),
            "".to_string(),
            "test".to_string(),
        );
        assert!(matches!(result, Err(OrganizationError::InvalidName(_))));
    }

    #[test]
    fn test_organization_create_invalid_slug() {
        let result = Organization::create(
            TenantId::generate(),
            "Test".to_string(),
            "INVALID_SLUG".to_string(),
        );
        assert!(matches!(result, Err(OrganizationError::InvalidSlug(_))));
    }

    #[test]
    fn test_update_name() {
        let mut org = create_test_organization();
        let old_updated_at = org.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        org.update_name("New Organization Name".to_string())
            .unwrap();

        assert_eq!(org.name.as_str(), "New Organization Name");
        assert!(org.updated_at > old_updated_at);
    }

    #[test]
    fn test_update_name_invalid() {
        let mut org = create_test_organization();
        let result = org.update_name("".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidName(_))));
        // 名称不应改变
        assert_eq!(org.name().as_str(), "Test Organization");
    }

    #[test]
    fn test_update_slug() {
        let mut org = create_test_organization();
        let old_updated_at = org.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        org.update_slug("new-slug".to_string()).unwrap();

        assert_eq!(org.slug.as_str(), "new-slug");
        assert!(org.updated_at > old_updated_at);
    }

    #[test]
    fn test_update_slug_invalid() {
        let mut org = create_test_organization();
        let result = org.update_slug("INVALID_SLUG".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidSlug(_))));
        // slug 不应改变
        assert_eq!(org.slug().as_str(), "test-org");
    }

    #[test]
    fn test_set_industry() {
        let mut org = create_test_organization();
        assert!(org.industry().is_none());

        org.set_industry(Some("Technology".to_string()));
        assert_eq!(org.industry(), Some(&"Technology".to_string()));

        org.set_industry(None);
        assert!(org.industry().is_none());
    }

    #[test]
    fn test_set_description() {
        let mut org = create_test_organization();
        assert!(org.description().is_none());

        org.set_description(Some("A test organization".to_string()));
        assert_eq!(org.description(), Some(&"A test organization".to_string()));

        org.set_description(None);
        assert!(org.description().is_none());
    }

    #[test]
    fn test_load() {
        let id = OrganizationId::generate();
        let tenant_id = TenantId::generate();
        let name = OrganizationName::new("Loaded Org".to_string()).unwrap();
        let slug = OrganizationSlug::new("loaded-org".to_string()).unwrap();

        let org = Organization::load(
            id.clone(),
            tenant_id.clone(),
            name.clone(),
            slug.clone(),
            Some("Tech".to_string()),
            Some("Description".to_string()),
            now(),
            now(),
        );

        assert_eq!(org.id(), &id);
        assert_eq!(org.tenant_id(), &tenant_id);
        assert_eq!(org.name(), &name);
        assert_eq!(org.slug(), &slug);
        assert_eq!(org.industry(), Some(&"Tech".to_string()));
        assert_eq!(org.description(), Some(&"Description".to_string()));
    }

    #[test]
    fn test_update_timestamp_changes() {
        let mut org = create_test_organization();
        let created_at = org.created_at;

        // 初始时 created_at 和 updated_at 应该相同（或非常接近）
        assert!(org.updated_at >= created_at);

        // 更新名称后 updated_at 应该更新
        std::thread::sleep(std::time::Duration::from_millis(1));
        org.update_name("Updated Name".to_string()).unwrap();
        assert!(org.updated_at > created_at);

        // 更新 slug 后 updated_at 应该再次更新
        std::thread::sleep(std::time::Duration::from_millis(1));
        org.update_slug("updated-slug".to_string()).unwrap();
        assert!(org.updated_at > created_at);
    }
}
