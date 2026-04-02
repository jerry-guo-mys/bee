//! Team 聚合根定义
//!
//! Team 是团队领域的聚合根，管理团队的生命周期和状态。

use crate::domain::common::now;
use chrono::{DateTime, Utc};

use super::value_object::{TeamCode, TeamError, TeamId, TeamName};
use crate::domain::tenant::{OrganizationId, TenantId};

/// 团队聚合根
#[derive(Debug, Clone)]
pub struct Team {
    id: TeamId,
    tenant_id: TenantId,
    organization_id: OrganizationId,
    name: TeamName,
    code: Option<TeamCode>,
    description: Option<String>,
    parent_team_id: Option<TeamId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Team {
    /// 创建新团队
    ///
    /// # Arguments
    /// * `tenant_id` - 所属租户 ID
    /// * `organization_id` - 所属组织 ID
    /// * `name` - 团队名称
    /// * `code` - 团队代码（可选）
    /// * `description` - 团队描述（可选）
    /// * `parent_team_id` - 父团队 ID（可选，用于团队层级）
    ///
    /// # Returns
    /// * `Result<Self, TeamError>` - 创建成功返回团队实例，失败返回错误
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        tenant_id: TenantId,
        organization_id: OrganizationId,
        name: String,
        code: Option<String>,
        description: Option<String>,
        parent_team_id: Option<TeamId>,
    ) -> Result<Self, TeamError> {
        let now = now();
        let team = Self {
            id: TeamId::generate(),
            tenant_id,
            organization_id,
            name: TeamName::new(name)?,
            code: code.map(TeamCode::new).transpose()?,
            description,
            parent_team_id,
            created_at: now,
            updated_at: now,
        };

        Ok(team)
    }

    /// 从已有数据加载团队（用于从数据库加载）
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        id: TeamId,
        tenant_id: TenantId,
        organization_id: OrganizationId,
        name: TeamName,
        code: Option<TeamCode>,
        description: Option<String>,
        parent_team_id: Option<TeamId>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            organization_id,
            name,
            code,
            description,
            parent_team_id,
            created_at,
            updated_at,
        }
    }

    // ==================== Getter 方法 ====================

    /// 获取团队 ID
    pub fn id(&self) -> &TeamId {
        &self.id
    }

    /// 获取所属租户 ID
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    /// 获取所属组织 ID
    pub fn organization_id(&self) -> &OrganizationId {
        &self.organization_id
    }

    /// 获取团队名称
    pub fn name(&self) -> &TeamName {
        &self.name
    }

    /// 获取团队代码
    pub fn code(&self) -> Option<&TeamCode> {
        self.code.as_ref()
    }

    /// 获取团队描述
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    /// 获取父团队 ID
    pub fn parent_team_id(&self) -> Option<&TeamId> {
        self.parent_team_id.as_ref()
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

    /// 更新团队名称
    ///
    /// # Arguments
    /// * `name` - 新的团队名称
    pub fn update_name(&mut self, name: String) -> Result<(), TeamError> {
        self.name = TeamName::new(name)?;
        self.updated_at = now();
        Ok(())
    }

    /// 更新团队代码
    ///
    /// # Arguments
    /// * `code` - 新的团队代码
    pub fn update_code(&mut self, code: Option<String>) -> Result<(), TeamError> {
        self.code = code.map(TeamCode::new).transpose()?;
        self.updated_at = now();
        Ok(())
    }

    /// 更新团队描述
    ///
    /// # Arguments
    /// * `description` - 新的团队描述
    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = now();
    }

    /// 设置父团队
    ///
    /// # Arguments
    /// * `parent_team_id` - 父团队 ID
    pub fn set_parent_team(&mut self, parent_team_id: Option<TeamId>) {
        self.parent_team_id = parent_team_id;
        self.updated_at = now();
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

    #[test]
    fn test_team_create() {
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let team = Team::create(
            tenant_id.clone(),
            org_id.clone(),
            "Test Team".to_string(),
            Some("TEST-001".to_string()),
            Some("A test team".to_string()),
            None,
        )
        .unwrap();

        assert_eq!(team.name().as_str(), "Test Team");
        assert_eq!(team.code().unwrap().as_str(), "TEST-001");
        assert_eq!(team.tenant_id(), &tenant_id);
        assert_eq!(team.organization_id(), &org_id);
        assert!(team.parent_team_id().is_none());
    }

    #[test]
    fn test_team_create_without_code() {
        let team = Team::create(
            TenantId::generate(),
            OrganizationId::generate(),
            "Test Team".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

        assert!(team.code().is_none());
        assert!(team.description().is_none());
    }

    #[test]
    fn test_team_create_invalid_name() {
        let result = Team::create(
            TenantId::generate(),
            OrganizationId::generate(),
            "".to_string(),
            None,
            None,
            None,
        );
        assert!(matches!(result, Err(TeamError::InvalidName(_))));
    }

    #[test]
    fn test_team_create_invalid_code() {
        let result = Team::create(
            TenantId::generate(),
            OrganizationId::generate(),
            "Test Team".to_string(),
            Some("invalid.code".to_string()),
            None,
            None,
        );
        assert!(matches!(result, Err(TeamError::InvalidCode(_))));
    }

    #[test]
    fn test_update_name() {
        let mut team = create_test_team();
        let old_updated_at = team.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        team.update_name("New Team Name".to_string()).unwrap();

        assert_eq!(team.name.as_str(), "New Team Name");
        assert!(team.updated_at > old_updated_at);
    }

    #[test]
    fn test_update_name_invalid() {
        let mut team = create_test_team();
        let result = team.update_name("".to_string());
        assert!(matches!(result, Err(TeamError::InvalidName(_))));
        assert_eq!(team.name.as_str(), "Test Team");
    }

    #[test]
    fn test_update_code() {
        let mut team = create_test_team();
        let old_updated_at = team.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        team.update_code(Some("NEW-002".to_string())).unwrap();

        assert_eq!(team.code.as_ref().unwrap().as_str(), "NEW-002");
        assert!(team.updated_at > old_updated_at);
    }

    #[test]
    fn test_update_code_remove() {
        let mut team = create_test_team();
        assert!(team.code().is_some());

        team.update_code(None).unwrap();
        assert!(team.code().is_none());
    }

    #[test]
    fn test_update_code_invalid() {
        let mut team = create_test_team();
        let result = team.update_code(Some("invalid.code".to_string()));
        assert!(matches!(result, Err(TeamError::InvalidCode(_))));
        assert!(team.code().is_some());
    }

    #[test]
    fn test_update_description() {
        let mut team = create_test_team();
        let old_updated_at = team.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        team.update_description(Some("New description".to_string()));

        assert_eq!(team.description(), Some(&"New description".to_string()));
        assert!(team.updated_at > old_updated_at);
    }

    #[test]
    fn test_update_description_remove() {
        let mut team = create_test_team();
        assert!(team.description().is_some());

        team.update_description(None);
        assert!(team.description().is_none());
    }

    #[test]
    fn test_set_parent_team() {
        let mut team = create_test_team();
        assert!(team.parent_team_id().is_none());

        let parent_id = TeamId::generate();
        team.set_parent_team(Some(parent_id.clone()));
        assert_eq!(team.parent_team_id(), Some(&parent_id));

        team.set_parent_team(None);
        assert!(team.parent_team_id().is_none());
    }

    #[test]
    fn test_load() {
        let id = TeamId::generate();
        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let name = TeamName::new("Loaded Team".to_string()).unwrap();
        let code = Some(TeamCode::new("LOAD-001".to_string()).unwrap());
        let parent_id = TeamId::generate();

        let team = Team::load(
            id.clone(),
            tenant_id.clone(),
            org_id.clone(),
            name.clone(),
            code.clone(),
            Some("Description".to_string()),
            Some(parent_id.clone()),
            now(),
            now(),
        );

        assert_eq!(team.id(), &id);
        assert_eq!(team.tenant_id(), &tenant_id);
        assert_eq!(team.organization_id(), &org_id);
        assert_eq!(team.name(), &name);
        assert_eq!(team.code(), code.as_ref());
        assert_eq!(team.parent_team_id(), Some(&parent_id));
    }

    #[test]
    fn test_update_timestamp_changes() {
        let mut team = create_test_team();
        let created_at = team.created_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        team.update_name("Updated".to_string()).unwrap();
        assert!(team.updated_at > created_at);

        std::thread::sleep(std::time::Duration::from_millis(1));
        team.update_code(Some("NEW-CODE".to_string())).unwrap();
        assert!(team.updated_at > created_at);

        std::thread::sleep(std::time::Duration::from_millis(1));
        team.update_description(Some("Desc".to_string()));
        assert!(team.updated_at > created_at);
    }
}
