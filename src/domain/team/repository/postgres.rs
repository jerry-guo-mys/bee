//! PostgreSQL 实现的 TeamRepository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::team::entity::Team;
use crate::domain::team::value_object::{TeamCode, TeamError, TeamId, TeamName};
use crate::domain::tenant::{OrganizationId, TenantId};
use crate::infrastructure::persistence::postgres::PostgresConnection;

/// PostgreSQL 团队仓库实现
pub struct PostgresTeamRepository {
    pool: PgPool,
}

impl PostgresTeamRepository {
    /// 创建新的 PostgreSQL 团队仓库
    pub fn new(conn: &PostgresConnection) -> Self {
        Self {
            pool: conn.pool().clone(),
        }
    }
}

/// 数据库行结构
#[derive(FromRow)]
struct TeamRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    organization_id: uuid::Uuid,
    name: String,
    code: Option<String>,
    parent_team_id: Option<uuid::Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TeamRow {
    /// 将数据库行转换为 Team 实体
    fn into_team(self) -> Result<Team, TeamError> {
        let id = TeamId::new(self.id.to_string());
        let tenant_id = TenantId::new(self.tenant_id.to_string());
        let organization_id = OrganizationId::new(self.organization_id.to_string());
        let name =
            TeamName::new(self.name).map_err(|e| TeamError::InvalidName(e.to_string()))?;
        let code = self
            .code
            .map(|c| TeamCode::new(c).map_err(|e| TeamError::InvalidCode(e.to_string())))
            .transpose()?;
        let parent_team_id = self.parent_team_id.map(|id| TeamId::new(id.to_string()));

        Ok(Team::load(
            id,
            tenant_id,
            organization_id,
            name,
            code,
            None, // description 从单独字段加载（如果需要）
            parent_team_id,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[async_trait]
impl super::TeamRepository for PostgresTeamRepository {
    type Error = TeamError;

    async fn save(&self, team: &Team) -> Result<(), Self::Error> {
        sqlx::query(
            r#"
            INSERT INTO teams (id, tenant_id, organization_id, name, code, parent_team_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                name = $4, code = $5, parent_team_id = $6, updated_at = $8
            "#,
        )
        .bind(
            team.id()
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .bind(
            team.tenant_id()
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .bind(
            team.organization_id()
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .bind(team.name().as_str())
        .bind(team.code().map(|c| c.as_str()))
        .bind(team.parent_team_id().map(|id| id.as_str()))
        .bind(team.created_at())
        .bind(team.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                TeamError::AlreadyExists(team.id().to_string())
            }
            _ => TeamError::DatabaseError(e.to_string()),
        })?;

        Ok(())
    }

    async fn find_by_id(&self, id: &TeamId) -> Result<Option<Team>, Self::Error> {
        let team = sqlx::query_as::<_, TeamRow>("SELECT * FROM teams WHERE id = $1")
            .bind(
                id.as_str()
                    .parse::<uuid::Uuid>()
                    .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| TeamError::DatabaseError(e.to_string()))?;

        match team {
            Some(row) => row.into_team().map(Some),
            None => Ok(None),
        }
    }

    async fn find_by_organization(
        &self,
        organization_id: &OrganizationId,
    ) -> Result<Vec<Team>, Self::Error> {
        let teams = sqlx::query_as::<_, TeamRow>(
            "SELECT * FROM teams WHERE organization_id = $1 ORDER BY name",
        )
        .bind(
            organization_id
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TeamError::DatabaseError(e.to_string()))?;

        teams.into_iter().map(|row| row.into_team()).collect()
    }

    async fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Team>, Self::Error> {
        let teams = sqlx::query_as::<_, TeamRow>(
            "SELECT * FROM teams WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(
            tenant_id
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TeamError::DatabaseError(e.to_string()))?;

        teams.into_iter().map(|row| row.into_team()).collect()
    }

    async fn find_by_parent(&self, parent_team_id: &TeamId) -> Result<Vec<Team>, Self::Error> {
        let teams = sqlx::query_as::<_, TeamRow>(
            "SELECT * FROM teams WHERE parent_team_id = $1 ORDER BY name",
        )
        .bind(
            parent_team_id
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TeamError::DatabaseError(e.to_string()))?;

        teams.into_iter().map(|row| row.into_team()).collect()
    }

    async fn find_by_code(
        &self,
        organization_id: &OrganizationId,
        code: &str,
    ) -> Result<Option<Team>, Self::Error> {
        let team = sqlx::query_as::<_, TeamRow>(
            "SELECT * FROM teams WHERE organization_id = $1 AND code = $2",
        )
        .bind(
            organization_id
                .as_str()
                .parse::<uuid::Uuid>()
                .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TeamError::DatabaseError(e.to_string()))?;

        match team {
            Some(row) => row.into_team().map(Some),
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &TeamId) -> Result<(), Self::Error> {
        let result = sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(
                id.as_str()
                    .parse::<uuid::Uuid>()
                    .map_err(|e| TeamError::DatabaseError(format!("Invalid UUID: {}", e)))?,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| TeamError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(TeamError::NotFound(id.to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::team::repository::TeamRepository;

    /// 获取测试数据库连接
    async fn get_test_connection() -> Option<PostgresConnection> {
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                eprintln!("DATABASE_URL not set, skipping test");
                return None;
            }
        };

        PostgresConnection::new(&database_url).await.ok()
    }

    #[tokio::test]
    async fn test_save_and_find_team() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTeamRepository::new(&conn);

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

        // 保存
        repo.save(&team).await.unwrap();

        // 根据 ID 查找
        let found = repo.find_by_id(team.id()).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id(), team.id());
        assert_eq!(found.name().as_str(), "Test Team");
        assert_eq!(found.code().unwrap().as_str(), "TEST-001");

        // 清理
        repo.delete(team.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_find_by_organization() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTeamRepository::new(&conn);

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

        // 清理
        repo.delete(team1.id()).await.unwrap();
        repo.delete(team2.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_find_by_parent() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTeamRepository::new(&conn);

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

        // 清理
        repo.delete(child_team.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_find_nonexistent_team() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTeamRepository::new(&conn);

        let nonexistent_id = TeamId::generate();
        let found = repo.find_by_id(&nonexistent_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_update_team() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTeamRepository::new(&conn);

        let tenant_id = TenantId::generate();
        let org_id = OrganizationId::generate();
        let mut team = Team::create(
            tenant_id.clone(),
            org_id.clone(),
            "Test Team".to_string(),
            Some("TEST-001".to_string()),
            None,
            None,
        )
        .unwrap();

        repo.save(&team).await.unwrap();

        // 修改并保存
        team.update_name("Updated Team".to_string())
            .unwrap();
        repo.save(&team).await.unwrap();

        // 验证更新
        let found = repo.find_by_id(team.id()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name().as_str(), "Updated Team");

        // 清理
        repo.delete(team.id()).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_team() {
        let conn = match get_test_connection().await {
            Some(c) => c,
            None => return,
        };
        let repo = PostgresTeamRepository::new(&conn);

        let team = Team::create(
            TenantId::generate(),
            OrganizationId::generate(),
            "Test Team".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

        repo.save(&team).await.unwrap();
        repo.delete(team.id()).await.unwrap();

        let found = repo.find_by_id(team.id()).await.unwrap();
        assert!(found.is_none());
    }
}
