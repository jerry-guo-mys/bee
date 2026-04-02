//! Organization Repository trait 定义
//!
//! Repository 模式用于抽象持久化层，使领域层不依赖于具体的数据库实现。

use async_trait::async_trait;

use super::entity::Organization;
use super::value_object::{OrganizationError, OrganizationId};
use crate::domain::tenant::TenantId;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
pub use postgres::PostgresOrganizationRepository;

/// Organization Repository trait
#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    type Error;

    /// 保存组织（新增或更新）
    ///
    /// # Arguments
    /// * `org` - 要保存的组织实例
    ///
    /// # Returns
    /// * `Result<(), Self::Error>` - 成功返回 Ok，失败返回错误
    async fn save(&self, org: &Organization) -> Result<(), Self::Error>;

    /// 根据 ID 查找组织
    ///
    /// # Arguments
    /// * `id` - 组织 ID
    ///
    /// # Returns
    /// * `Result<Option<Organization>, Self::Error>` - 找到返回 Some(Organization)，否则返回 None 或错误
    async fn find_by_id(&self, id: &OrganizationId) -> Result<Option<Organization>, Self::Error>;

    /// 根据租户 ID 查找所有组织
    ///
    /// # Arguments
    /// * `tenant_id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<Vec<Organization>, Self::Error>` - 组织列表
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Organization>, Self::Error>;

    /// 根据 slug 查找组织
    ///
    /// # Arguments
    /// * `tenant_id` - 租户 ID
    /// * `slug` - 组织 slug
    ///
    /// # Returns
    /// * `Result<Option<Organization>, Self::Error>` - 找到返回 Some(Organization)，否则返回 None 或错误
    async fn find_by_slug(
        &self,
        tenant_id: &TenantId,
        slug: &str,
    ) -> Result<Option<Organization>, Self::Error>;

    /// 删除组织
    ///
    /// # Arguments
    /// * `id` - 组织 ID
    ///
    /// # Returns
    /// * `Result<(), Self::Error>` - 成功返回 Ok，失败返回错误
    async fn delete(&self, id: &OrganizationId) -> Result<(), Self::Error>;
}

/// 内存实现（用于测试）
pub struct InMemoryOrganizationRepository {
    data: tokio::sync::RwLock<std::collections::HashMap<String, Organization>>,
}

impl InMemoryOrganizationRepository {
    pub fn new() -> Self {
        Self {
            data: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryOrganizationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrganizationRepository for InMemoryOrganizationRepository {
    type Error = OrganizationError;

    async fn save(&self, org: &Organization) -> Result<(), Self::Error> {
        let mut data = self.data.write().await;
        let id = org.id().as_str().to_string();
        data.insert(id, org.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &OrganizationId) -> Result<Option<Organization>, Self::Error> {
        let data = self.data.read().await;
        Ok(data.get(id.as_str()).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Organization>, Self::Error> {
        let data = self.data.read().await;
        Ok(data
            .values()
            .filter(|org| org.tenant_id() == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_slug(
        &self,
        tenant_id: &TenantId,
        slug: &str,
    ) -> Result<Option<Organization>, Self::Error> {
        let data = self.data.read().await;
        Ok(data
            .values()
            .find(|org| org.tenant_id() == tenant_id && org.slug().as_str() == slug)
            .cloned())
    }

    async fn delete(&self, id: &OrganizationId) -> Result<(), Self::Error> {
        let mut data = self.data.write().await;
        data.remove(id.as_str());
        Ok(())
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

    #[tokio::test]
    async fn test_in_memory_repository_save_and_find() {
        let repo = InMemoryOrganizationRepository::new();
        let org = create_test_organization();
        let org_id = org.id().clone();

        // 保存组织
        repo.save(&org).await.unwrap();

        // 根据 ID 查找
        let found = repo.find_by_id(&org_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name().as_str(), "Test Organization");
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_by_tenant() {
        let repo = InMemoryOrganizationRepository::new();
        let tenant_id = TenantId::generate();

        let org1 =
            Organization::create(tenant_id.clone(), "Org 1".to_string(), "org-1".to_string())
                .unwrap();
        let org2 =
            Organization::create(tenant_id.clone(), "Org 2".to_string(), "org-2".to_string())
                .unwrap();

        repo.save(&org1).await.unwrap();
        repo.save(&org2).await.unwrap();

        let orgs = repo.find_by_tenant(&tenant_id).await.unwrap();
        assert_eq!(orgs.len(), 2);

        // 不同租户应该返回空
        let other_tenant = TenantId::generate();
        let orgs = repo.find_by_tenant(&other_tenant).await.unwrap();
        assert_eq!(orgs.len(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_by_slug() {
        let repo = InMemoryOrganizationRepository::new();
        let tenant_id = TenantId::generate();
        let org = Organization::create(
            tenant_id.clone(),
            "Test Org".to_string(),
            "test-org".to_string(),
        )
        .unwrap();

        repo.save(&org).await.unwrap();

        // 找到组织的 slug
        let found = repo.find_by_slug(&tenant_id, "test-org").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().slug().as_str(), "test-org");

        // 不存在的 slug
        let found = repo.find_by_slug(&tenant_id, "non-existent").await.unwrap();
        assert!(found.is_none());

        // 不同租户的相同 slug 应该找不到
        let other_tenant = TenantId::generate();
        let found = repo.find_by_slug(&other_tenant, "test-org").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_delete() {
        let repo = InMemoryOrganizationRepository::new();
        let org = create_test_organization();
        let org_id = org.id().clone();

        repo.save(&org).await.unwrap();
        repo.delete(&org_id).await.unwrap();

        let found = repo.find_by_id(&org_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_not_found() {
        let repo = InMemoryOrganizationRepository::new();
        let non_existent_id = OrganizationId::generate();

        let found = repo.find_by_id(&non_existent_id).await.unwrap();
        assert!(found.is_none());

        let found = repo
            .find_by_slug(&TenantId::generate(), "non-existent")
            .await
            .unwrap();
        assert!(found.is_none());
    }
}
