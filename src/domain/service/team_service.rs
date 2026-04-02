//! Team 领域服务
//!
//! 提供团队相关的领域服务，协调 Repository 和 Event Publisher。

use std::sync::Arc;

use crate::domain::event::{DomainEventPublisher, InMemoryEventPublisher};
use crate::domain::team::{Team, TeamCode, TeamError, TeamEvent, TeamId, TeamName, TeamRepository};
use crate::domain::tenant::{OrganizationId, TenantId};

/// Team 领域服务
///
/// 负责协调团队的创建、更新、删除等业务操作，
/// 并确保领域事件的正确发布。
pub struct TeamDomainService<TR, EP> {
    team_repo: Arc<TR>,
    event_publisher: Arc<EP>,
}

impl<TR, EP> TeamDomainService<TR, EP>
where
    TR: TeamRepository<Error = TeamError> + 'static,
    EP: DomainEventPublisher + 'static,
{
    /// 创建新的团队领域服务实例
    ///
    /// # Arguments
    /// * `team_repo` - 团队 Repository
    /// * `event_publisher` - 领域事件发布器
    pub fn new(team_repo: Arc<TR>, event_publisher: Arc<EP>) -> Self {
        Self {
            team_repo,
            event_publisher,
        }
    }

    /// 创建团队
    ///
    /// # Arguments
    /// * `tenant_id` - 所属租户 ID
    /// * `organization_id` - 所属组织 ID
    /// * `name` - 团队名称
    /// * `code` - 团队编码 (可选)
    /// * `description` - 团队描述 (可选)
    /// * `parent_team_id` - 父团队 ID (可选)
    ///
    /// # Returns
    /// * `Result<Team, TeamError>` - 创建成功返回团队实例，失败返回错误
    pub async fn create_team(
        &self,
        tenant_id: TenantId,
        organization_id: OrganizationId,
        name: String,
        code: Option<String>,
        description: Option<String>,
        parent_team_id: Option<TeamId>,
    ) -> Result<Team, TeamError> {
        // 验证名称 (会返回错误如果无效)
        let _name = TeamName::new(name.clone())?;

        // 验证 code (如果提供)
        if let Some(ref code_str) = code {
            let _ = TeamCode::new(code_str.clone())?;
        }

        // 检查 code 是否已存在 (如果提供)
        if let Some(ref code_str) = code {
            let existing = self
                .team_repo
                .find_by_code(&organization_id, code_str)
                .await?;
            if existing.is_some() {
                return Err(TeamError::AlreadyExists(format!(
                    "Team with code '{}' already exists in this organization",
                    code_str
                )));
            }
        }

        // 创建团队
        let team = Team::create(
            tenant_id.clone(),
            organization_id.clone(),
            name.clone(),
            code.clone(),
            description.clone(),
            parent_team_id.clone(),
        )?;

        // 保存团队
        self.team_repo.save(&team).await?;

        // 发布 TeamCreated 事件
        let created_event = TeamEvent::Created(crate::domain::team::TeamCreated::new(
            team.id().clone(),
            tenant_id,
            organization_id,
            name,
            code,
        ));
        self.event_publisher.publish(created_event).await;

        Ok(team)
    }

    /// 更新团队名称
    ///
    /// # Arguments
    /// * `id` - 团队 ID
    /// * `name` - 新的团队名称
    ///
    /// # Returns
    /// * `Result<(), TeamError>` - 成功返回 Ok，失败返回错误
    pub async fn update_team_name(&self, id: &TeamId, name: String) -> Result<(), TeamError> {
        let mut team = self
            .team_repo
            .find_by_id(id)
            .await?
            .ok_or(TeamError::NotFound("Team not found".into()))?;

        team.update_name(name.clone())?;
        self.team_repo.save(&team).await?;

        // 发布 TeamUpdated 事件
        let updated_event =
            TeamEvent::Updated(crate::domain::team::TeamUpdated::new(id.clone(), name));
        self.event_publisher.publish(updated_event).await;

        Ok(())
    }

    /// 更新团队编码
    ///
    /// # Arguments
    /// * `id` - 团队 ID
    /// * `code` - 新的团队编码
    ///
    /// # Returns
    /// * `Result<(), TeamError>` - 成功返回 Ok，失败返回错误
    pub async fn update_team_code(&self, id: &TeamId, code: String) -> Result<(), TeamError> {
        let mut team = self
            .team_repo
            .find_by_id(id)
            .await?
            .ok_or(TeamError::NotFound("Team not found".into()))?;

        team.update_code(Some(code.clone()))?;
        self.team_repo.save(&team).await?;

        // 发布 TeamUpdated 事件
        let updated_event = TeamEvent::Updated(crate::domain::team::TeamUpdated::new(
            id.clone(),
            team.name().as_str().to_string(),
        ));
        self.event_publisher.publish(updated_event).await;

        Ok(())
    }

    /// 更新团队描述
    ///
    /// # Arguments
    /// * `id` - 团队 ID
    /// * `description` - 新的团队描述
    ///
    /// # Returns
    /// * `Result<(), TeamError>` - 成功返回 Ok，失败返回错误
    pub async fn update_team_description(
        &self,
        id: &TeamId,
        description: String,
    ) -> Result<(), TeamError> {
        let mut team = self
            .team_repo
            .find_by_id(id)
            .await?
            .ok_or(TeamError::NotFound("Team not found".into()))?;

        team.update_description(Some(description));
        self.team_repo.save(&team).await?;

        // 发布 TeamUpdated 事件
        let updated_event = TeamEvent::Updated(crate::domain::team::TeamUpdated::new(
            id.clone(),
            team.name().as_str().to_string(),
        ));
        self.event_publisher.publish(updated_event).await;

        Ok(())
    }

    /// 设置父团队
    ///
    /// # Arguments
    /// * `id` - 团队 ID
    /// * `parent_team_id` - 父团队 ID (可选)
    ///
    /// # Returns
    /// * `Result<(), TeamError>` - 成功返回 Ok，失败返回错误
    pub async fn set_parent_team(
        &self,
        id: &TeamId,
        parent_team_id: Option<TeamId>,
    ) -> Result<(), TeamError> {
        let mut team = self
            .team_repo
            .find_by_id(id)
            .await?
            .ok_or(TeamError::NotFound("Team not found".into()))?;

        team.set_parent_team(parent_team_id.clone());
        self.team_repo.save(&team).await?;

        // 发布 TeamUpdated 事件
        let updated_event = TeamEvent::Updated(crate::domain::team::TeamUpdated::new(
            id.clone(),
            team.name().as_str().to_string(),
        ));
        self.event_publisher.publish(updated_event).await;

        Ok(())
    }

    /// 删除团队
    ///
    /// # Arguments
    /// * `id` - 团队 ID
    ///
    /// # Returns
    /// * `Result<(), TeamError>` - 成功返回 Ok，失败返回错误
    pub async fn delete_team(&self, id: &TeamId) -> Result<(), TeamError> {
        // 先检查是否存在
        let _team = self
            .team_repo
            .find_by_id(id)
            .await?
            .ok_or(TeamError::NotFound("Team not found".into()))?;

        // 发布 TeamDeleted 事件
        let deleted_event = TeamEvent::Deleted(crate::domain::team::TeamDeleted::new(id.clone()));
        self.event_publisher.publish(deleted_event).await;

        // 物理删除
        self.team_repo.delete(id).await?;

        Ok(())
    }

    /// 根据 ID 获取团队
    ///
    /// # Arguments
    /// * `id` - 团队 ID
    ///
    /// # Returns
    /// * `Result<Option<Team>, TeamError>` - 找到返回 Some(Team)，否则返回 None
    pub async fn get_team_by_id(&self, id: &TeamId) -> Result<Option<Team>, TeamError> {
        self.team_repo.find_by_id(id).await
    }

    /// 根据组织 ID 获取所有团队
    ///
    /// # Arguments
    /// * `organization_id` - 组织 ID
    ///
    /// # Returns
    /// * `Result<Vec<Team>, TeamError>` - 团队列表
    pub async fn get_teams_by_organization(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Vec<Team>, TeamError> {
        self.team_repo.find_by_organization(organization_id).await
    }

    /// 根据租户 ID 获取所有团队
    ///
    /// # Arguments
    /// * `tenant_id` - 租户 ID
    ///
    /// # Returns
    /// * `Result<Vec<Team>, TeamError>` - 团队列表
    pub async fn get_teams_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Team>, TeamError> {
        self.team_repo.find_by_tenant(tenant_id).await
    }

    /// 根据父团队 ID 获取子团队
    ///
    /// # Arguments
    /// * `parent_team_id` - 父团队 ID
    ///
    /// # Returns
    /// * `Result<Vec<Team>, TeamError>` - 子团队列表
    pub async fn get_child_teams(&self, parent_team_id: &TeamId) -> Result<Vec<Team>, TeamError> {
        self.team_repo.find_by_parent(parent_team_id).await
    }

    /// 根据 code 获取团队
    ///
    /// # Arguments
    /// * `organization_id` - 组织 ID
    /// * `code` - 团队 code
    ///
    /// # Returns
    /// * `Result<Option<Team>, TeamError>` - 找到返回 Some(Team)，否则返回 None
    pub async fn get_team_by_code(
        &self,
        organization_id: &OrganizationId,
        code: &str,
    ) -> Result<Option<Team>, TeamError> {
        self.team_repo.find_by_code(organization_id, code).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use crate::domain::team::InMemoryTeamRepository;

    fn create_service() -> (
        TeamDomainService<InMemoryTeamRepository, InMemoryEventPublisher>,
        Arc<InMemoryTeamRepository>,
        Arc<InMemoryEventPublisher>,
    ) {
        let repo = Arc::new(InMemoryTeamRepository::new());
        let publisher = Arc::new(InMemoryEventPublisher::new());
        let service = TeamDomainService::new(repo.clone(), publisher.clone());
        (service, repo, publisher)
    }

    #[tokio::test]
    async fn test_create_team_success() {
        let (service, _repo, publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let team = service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Test Team".to_string(),
                Some("TEST-001".to_string()),
                Some("A test team".to_string()),
                None,
            )
            .await
            .unwrap();

        assert_eq!(team.name().as_str(), "Test Team");
        assert_eq!(team.code().unwrap().as_str(), "TEST-001");
        assert_eq!(team.organization_id(), &org_id);
        assert_eq!(team.tenant_id(), &tenant_id);

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "team.created");
    }

    #[tokio::test]
    async fn test_create_team_duplicate_code() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();

        // 创建第一个团队
        service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Team 1".to_string(),
                Some("TEST-001".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // 尝试创建重复 code 的团队
        let result = service
            .create_team(
                tenant_id,
                org_id,
                "Team 2".to_string(),
                Some("TEST-001".to_string()),
                None,
                None,
            )
            .await;

        assert!(matches!(result, Err(TeamError::AlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_create_team_invalid_name() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let result = service
            .create_team(tenant_id, org_id, "".to_string(), None, None, None)
            .await;

        assert!(matches!(result, Err(TeamError::InvalidName(_))));
    }

    #[tokio::test]
    async fn test_create_team_with_parent() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let parent_team = service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Parent Team".to_string(),
                Some("PARENT-001".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        let child_team = service
            .create_team(
                tenant_id,
                org_id,
                "Child Team".to_string(),
                Some("CHILD-001".to_string()),
                None,
                Some(parent_team.id().clone()),
            )
            .await
            .unwrap();

        assert_eq!(child_team.parent_team_id(), Some(parent_team.id()));
    }

    #[tokio::test]
    async fn test_update_team_name() {
        let (service, _repo, publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let team = service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Test Team".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 更新名称
        service
            .update_team_name(team.id(), "Updated Team".to_string())
            .await
            .unwrap();

        // 验证更新
        let updated = service.get_team_by_id(team.id()).await.unwrap().unwrap();
        assert_eq!(updated.name().as_str(), "Updated Team");

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 2); // Created + Updated
        assert_eq!(events[1].event_type(), "team.updated");
    }

    #[tokio::test]
    async fn test_update_team_code() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let team = service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Test Team".to_string(),
                Some("OLD-001".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        // 更新 code
        service
            .update_team_code(team.id(), "NEW-001".to_string())
            .await
            .unwrap();

        // 验证更新
        let updated = service.get_team_by_id(team.id()).await.unwrap().unwrap();
        assert_eq!(updated.code().unwrap().as_str(), "NEW-001");
    }

    #[tokio::test]
    async fn test_set_parent_team() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let team1 = service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Team 1".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let team2 = service
            .create_team(tenant_id, org_id, "Team 2".to_string(), None, None, None)
            .await
            .unwrap();

        // 设置 team2 的父团队为 team1
        service
            .set_parent_team(team2.id(), Some(team1.id().clone()))
            .await
            .unwrap();

        // 验证更新
        let updated = service.get_team_by_id(team2.id()).await.unwrap().unwrap();
        assert_eq!(updated.parent_team_id(), Some(team1.id()));
    }

    #[tokio::test]
    async fn test_delete_team() {
        let (service, _repo, publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let team = service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Test Team".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let team_id = team.id().clone();

        // 删除团队
        service.delete_team(&team_id).await.unwrap();

        // 验证团队已删除
        let found = service.get_team_by_id(&team_id).await.unwrap();
        assert!(found.is_none());

        // 验证事件已发布
        let events = publisher.get_events().await;
        assert_eq!(events.len(), 2); // Created + Deleted
        assert_eq!(events[1].event_type(), "team.deleted");
    }

    #[tokio::test]
    async fn test_get_team_not_found() {
        let (service, _repo, _publisher) = create_service();

        let non_existent_id = TeamId::generate();
        let result = service.get_team_by_id(&non_existent_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_teams_by_organization() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id1 = OrganizationId::generate();
        let org_id2 = OrganizationId::generate();

        service
            .create_team(
                tenant_id.clone(),
                org_id1.clone(),
                "Team 1".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        service
            .create_team(
                tenant_id.clone(),
                org_id1.clone(),
                "Team 2".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        service
            .create_team(
                tenant_id,
                org_id2.clone(),
                "Team 3".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let teams = service.get_teams_by_organization(&org_id1).await.unwrap();
        assert_eq!(teams.len(), 2);

        let teams = service.get_teams_by_organization(&org_id2).await.unwrap();
        assert_eq!(teams.len(), 1);
    }

    #[tokio::test]
    async fn test_get_child_teams() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let parent_team = service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Parent Team".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Child 1".to_string(),
                None,
                None,
                Some(parent_team.id().clone()),
            )
            .await
            .unwrap();
        service
            .create_team(
                tenant_id,
                org_id,
                "Child 2".to_string(),
                None,
                None,
                Some(parent_team.id().clone()),
            )
            .await
            .unwrap();

        let children = service.get_child_teams(parent_team.id()).await.unwrap();
        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_get_team_by_code() {
        let (service, _repo, _publisher) = create_service();

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        service
            .create_team(
                tenant_id.clone(),
                org_id.clone(),
                "Test Team".to_string(),
                Some("TEST-001".to_string()),
                None,
                None,
            )
            .await
            .unwrap();

        let team = service.get_team_by_code(&org_id, "TEST-001").await.unwrap();
        assert!(team.is_some());
        assert_eq!(team.unwrap().code().unwrap().as_str(), "TEST-001");
    }
}
