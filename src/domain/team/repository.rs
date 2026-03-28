//! Team Repository trait 定义
//!
//! Repository 模式用于抽象持久化层，使领域层不依赖于具体的数据库实现。

use async_trait::async_trait;

use super::entity::Team;
use super::value_object::{TeamError, TeamId};
use crate::domain::tenant::{OrganizationId, TenantId};

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
pub use postgres::PostgresTeamRepository;

/// Team Repository trait
#[async_trait]
pub trait TeamRepository: Send + Sync {
    type Error;

    /// 保存团队（新增或更新）
    async fn save(&self, team: &Team) -> Result<(), Self::Error>;

    /// 根据 ID 查找团队
    async fn find_by_id(&self, id: &TeamId) -> Result<Option<Team>, Self::Error>;

    /// 根据组织 ID 查找所有团队
    async fn find_by_organization(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Vec<Team>, Self::Error>;

    /// 根据租户 ID 查找所有团队
    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Team>, Self::Error>;

    /// 根据父团队 ID 查找子团队
    async fn find_by_parent(
        &self,
        parent_team_id: &TeamId,
    ) -> Result<Vec<Team>, Self::Error>;

    /// 根据代码查找团队
    async fn find_by_code(
        &self,
        organization_id: &OrganizationId,
        code: &str,
    ) -> Result<Option<Team>, Self::Error>;

    /// 删除团队
    async fn delete(&self, id: &TeamId) -> Result<(), Self::Error>;
}

/// 内存实现（用于测试）
pub struct InMemoryTeamRepository {
    data: tokio::sync::RwLock<std::collections::HashMap<String, Team>>,
}

impl InMemoryTeamRepository {
    pub fn new() -> Self {
        Self {
            data: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryTeamRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TeamRepository for InMemoryTeamRepository {
    type Error = TeamError;

    async fn save(&self, team: &Team) -> Result<(), Self::Error> {
        let mut data = self.data.write().await;
        let id = team.id().as_str().to_string();
        data.insert(id, team.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &TeamId) -> Result<Option<Team>, Self::Error> {
        let data = self.data.read().await;
        Ok(data.get(id.as_str()).cloned())
    }

    async fn find_by_organization(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Vec<Team>, Self::Error> {
        let data = self.data.read().await;
        Ok(data
            .values()
            .filter(|team| team.organization_id() == organization_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Team>, Self::Error> {
        let data = self.data.read().await;
        Ok(data
            .values()
            .filter(|team| team.tenant_id() == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_parent(
        &self,
        parent_team_id: &TeamId,
    ) -> Result<Vec<Team>, Self::Error> {
        let data = self.data.read().await;
        Ok(data
            .values()
            .filter(|team| team.parent_team_id() == Some(parent_team_id))
            .cloned()
            .collect())
    }

    async fn find_by_code(
        &self,
        organization_id: &OrganizationId,
        code: &str,
    ) -> Result<Option<Team>, Self::Error> {
        let data = self.data.read().await;
        Ok(data.values().find(|team| {
            team.organization_id() == organization_id
                && team.code().map(|c| c.as_str()) == Some(code)
        }).cloned())
    }

    async fn delete(&self, id: &TeamId) -> Result<(), Self::Error> {
        let mut data = self.data.write().await;
        data.remove(id.as_str());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_team() -> Team {
        Team::create(
            TenantId::generate(),
            OrganizationId::generate(),
            "Test Team".to_string(),
            Some("TEST-001".to_string()),
            Some("A test team".to_string()),
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_in_memory_repository_save_and_find() {
        let repo = InMemoryTeamRepository::new();
        let team = create_test_team();
        let team_id = team.id().clone();

        repo.save(&team).await.unwrap();

        let found = repo.find_by_id(&team_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name().as_str(), "Test Team");
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_by_organization() {
        let repo = InMemoryTeamRepository::new();
        let org_id = OrganizationId::generate();

        let team1 = Team::create(
            TenantId::generate(),
            org_id.clone(),
            "Team 1".to_string(),
            Some("TEAM-001".to_string()),
            None,
            None,
        )
        .unwrap();
        let team2 = Team::create(
            TenantId::generate(),
            org_id.clone(),
            "Team 2".to_string(),
            Some("TEAM-002".to_string()),
            None,
            None,
        )
        .unwrap();

        repo.save(&team1).await.unwrap();
        repo.save(&team2).await.unwrap();

        let teams = repo.find_by_organization(&org_id).await.unwrap();
        assert_eq!(teams.len(), 2);

        // 不同组织应该返回空
        let other_org = OrganizationId::generate();
        let teams = repo.find_by_organization(&other_org).await.unwrap();
        assert_eq!(teams.len(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_by_tenant() {
        let repo = InMemoryTeamRepository::new();
        let tenant_id = TenantId::generate();

        let team1 = Team::create(
            tenant_id.clone(),
            OrganizationId::generate(),
            "Team 1".to_string(),
            None,
            None,
            None,
        )
        .unwrap();
        let team2 = Team::create(
            tenant_id.clone(),
            OrganizationId::generate(),
            "Team 2".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

        repo.save(&team1).await.unwrap();
        repo.save(&team2).await.unwrap();

        let teams = repo.find_by_tenant(&tenant_id).await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_by_parent() {
        let repo = InMemoryTeamRepository::new();
        let parent_id = TeamId::generate();

        let child_team = Team::create(
            TenantId::generate(),
            OrganizationId::generate(),
            "Child Team".to_string(),
            None,
            None,
            Some(parent_id.clone()),
        )
        .unwrap();

        repo.save(&child_team).await.unwrap();

        let children = repo.find_by_parent(&parent_id).await.unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name().as_str(), "Child Team");
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_by_code() {
        let repo = InMemoryTeamRepository::new();
        let org_id = OrganizationId::generate();

        let team = Team::create(
            TenantId::generate(),
            org_id.clone(),
            "Test Team".to_string(),
            Some("UNIQUE-CODE".to_string()),
            None,
            None,
        )
        .unwrap();

        repo.save(&team).await.unwrap();

        let found = repo.find_by_code(&org_id, "UNIQUE-CODE").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().code().unwrap().as_str(), "UNIQUE-CODE");

        // 不存在的代码
        let found = repo.find_by_code(&org_id, "NON-EXISTENT").await.unwrap();
        assert!(found.is_none());

        // 不同组织的相同代码应该找不到
        let other_org = OrganizationId::generate();
        let found = repo.find_by_code(&other_org, "UNIQUE-CODE").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_delete() {
        let repo = InMemoryTeamRepository::new();
        let team = create_test_team();
        let team_id = team.id().clone();

        repo.save(&team).await.unwrap();
        repo.delete(&team_id).await.unwrap();

        let found = repo.find_by_id(&team_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_repository_find_not_found() {
        let repo = InMemoryTeamRepository::new();
        let non_existent_id = TeamId::generate();

        let found = repo.find_by_id(&non_existent_id).await.unwrap();
        assert!(found.is_none());
    }
}
