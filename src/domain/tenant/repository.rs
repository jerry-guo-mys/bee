//! 租户 Repository trait 定义
//!
//! Repository 模式用于抽象持久化层，使领域层不依赖于具体的数据库实现。

use async_trait::async_trait;

use super::entity::Tenant;
use super::value_object::{TenantError, TenantId};

/// 租户 Repository trait
#[async_trait]
pub trait TenantRepository: Send + Sync {
    /// 保存租户（新增或更新）
    ///
    /// # Arguments
    /// * `tenant` - 要保存的租户实例
    ///
    /// # Returns
    /// * `Result<(), TenantError>` - 成功返回 Ok，失败返回错误
    async fn save(&self, tenant: &Tenant) -> Result<(), TenantError>;

    /// 根据 ID 查找租户
    ///
    /// # Arguments
    /// * `id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<Option<Tenant>, TenantError>` - 找到返回 Some(Tenant)，否则返回 None 或错误
    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError>;

    /// 根据 slug 查找租户
    ///
    /// # Arguments
    /// * `slug` - 租户 slug
    ///
    /// # Returns
    /// * `Result<Option<Tenant>, TenantError>` - 找到返回 Some(Tenant)，否则返回 None 或错误
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, TenantError>;

    /// 删除租户
    ///
    /// # Arguments
    /// * `id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<(), TenantError>` - 成功返回 Ok，失败返回错误
    async fn delete(&self, id: &TenantId) -> Result<(), TenantError>;

    /// 检查 slug 是否存在
    ///
    /// # Arguments
    /// * `slug` - 租户 slug
    ///
    /// # Returns
    /// * `Result<bool, TenantError>` - 存在返回 true，否则返回 false
    async fn exists_by_slug(&self, slug: &str) -> Result<bool, TenantError> {
        let tenant = self.find_by_slug(slug).await?;
        Ok(tenant.is_some())
    }
}

/// 内存实现（用于测试）
pub struct InMemoryTenantRepository {
    data: tokio::sync::RwLock<std::collections::HashMap<String, Tenant>>,
}

impl InMemoryTenantRepository {
    pub fn new() -> Self {
        Self {
            data: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryTenantRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TenantRepository for InMemoryTenantRepository {
    async fn save(&self, tenant: &Tenant) -> Result<(), TenantError> {
        let mut data = self.data.write().await;
        let id = tenant.id().as_str().to_string();
        data.insert(id, tenant.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, TenantError> {
        let data = self.data.read().await;
        Ok(data.get(id.as_str()).cloned())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, TenantError> {
        let data = self.data.read().await;
        Ok(data.values().find(|t| t.slug().as_str() == slug).cloned())
    }

    async fn delete(&self, id: &TenantId) -> Result<(), TenantError> {
        let mut data = self.data.write().await;
        data.remove(id.as_str());
        Ok(())
    }

    async fn exists_by_slug(&self, slug: &str) -> Result<bool, TenantError> {
        let data = self.data.read().await;
        Ok(data.values().any(|t| t.slug().as_str() == slug))
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

    #[tokio::test]
    async fn test_in_memory_repository_save_and_find() {
        let repo = InMemoryTenantRepository::new();
        let tenant = create_test_tenant();
        let tenant_id = tenant.id().clone();

        // 保存租户
        repo.save(&tenant).await.unwrap();

        // 根据 ID 查找
        let found = repo.find_by_id(&tenant_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name().as_str(), "Test Tenant");

        // 根据 slug 查找
        let found = repo.find_by_slug("test-tenant").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().slug().as_str(), "test-tenant");
    }

    #[tokio::test]
    async fn test_in_memory_repository_delete() {
        let repo = InMemoryTenantRepository::new();
        let tenant = create_test_tenant();
        let tenant_id = tenant.id().clone();

        repo.save(&tenant).await.unwrap();
        repo.delete(&tenant_id).await.unwrap();

        let found = repo.find_by_id(&tenant_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_exists_by_slug() {
        let repo = InMemoryTenantRepository::new();
        let tenant = create_test_tenant();

        // 保存前不存在
        let exists = repo.exists_by_slug("test-tenant").await.unwrap();
        assert!(!exists);

        // 保存后存在
        repo.save(&tenant).await.unwrap();
        let exists = repo.exists_by_slug("test-tenant").await.unwrap();
        assert!(exists);

        // 不存在的 slug
        let exists = repo.exists_by_slug("non-existent").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_not_found() {
        let repo = InMemoryTenantRepository::new();
        let non_existent_id = TenantId::generate();

        let found = repo.find_by_id(&non_existent_id).await.unwrap();
        assert!(found.is_none());

        let found = repo.find_by_slug("non-existent").await.unwrap();
        assert!(found.is_none());
    }
}
