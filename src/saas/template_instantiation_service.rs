//! Agent 模板实例化服务
//!
//! 提供团队级模板实例化能力，把平台/租户模板转成具体的 Agent 实例。

use anyhow::{anyhow, Context};
use rusqlite::{params, OptionalExtension};

use crate::saas::models::{AgentInstance, AgentInstanceStatus, AgentTemplate, Team};
use crate::saas::sqlite::SaasSqliteStore;
use crate::saas::sqlite_seed_repository::SaasSeedRepository;
use crate::saas::sqlite_template_repository::SaasTemplateRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamTemplateInstantiationRequest {
    pub tenant_id: String,
    pub organization_id: String,
    pub team_id: String,
    pub template_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamTemplateInstantiationResult {
    pub tenant_id: String,
    pub organization_id: String,
    pub team_id: String,
    pub created_count: usize,
    pub existing_count: usize,
    pub instances: Vec<AgentInstance>,
}

pub fn list_team_templates(
    store: &SaasSqliteStore,
    tenant_id: &str,
) -> anyhow::Result<Vec<AgentTemplate>> {
    let repo = SaasTemplateRepository::new(store);
    repo.list_agent_templates(tenant_id)
}

pub fn instantiate_team_templates(
    store: &SaasSqliteStore,
    req: &TeamTemplateInstantiationRequest,
) -> anyhow::Result<TeamTemplateInstantiationResult> {
    let team = load_team(store, &req.tenant_id, &req.organization_id, &req.team_id)?
        .ok_or_else(|| anyhow!("team not found for organization scope"))?;
    let selected = load_selected_templates(store, &req.tenant_id, &req.template_ids)?;
    if selected.is_empty() {
        return Err(anyhow!("no agent templates available for instantiation"));
    }

    let repo = SaasSeedRepository::new(store);
    let mut created_count = 0;
    let mut existing_count = 0;
    let mut instances = Vec::new();
    for template in selected {
        if let Some(existing) =
            load_existing_instance(store, &req.tenant_id, &req.organization_id, &team.id, &template.id)?
        {
            existing_count += 1;
            instances.push(existing);
            continue;
        }

        let now = chrono::Utc::now().to_rfc3339();
        let instance = AgentInstance {
            id: format!("agentinst_{}", uuid::Uuid::new_v4().simple()),
            tenant_id: req.tenant_id.clone(),
            organization_id: req.organization_id.clone(),
            team_id: Some(team.id.clone()),
            template_id: template.id.clone(),
            name: format!("{} {}", team.name, template.name),
            status: AgentInstanceStatus::Active,
            prompt_override: template.prompt.clone(),
            tool_ids_override: template.tool_ids.clone(),
            model_id_override: template.model_id.clone(),
            knowledge_base_ids_override: template.knowledge_base_ids.clone(),
            created_at: now.clone(),
            updated_at: now,
        };
        repo.create_agent_instance(&instance)?;
        created_count += 1;
        instances.push(instance);
    }

    Ok(TeamTemplateInstantiationResult {
        tenant_id: req.tenant_id.clone(),
        organization_id: req.organization_id.clone(),
        team_id: req.team_id.clone(),
        created_count,
        existing_count,
        instances,
    })
}

fn load_team(
    store: &SaasSqliteStore,
    tenant_id: &str,
    organization_id: &str,
    team_id: &str,
) -> anyhow::Result<Option<Team>> {
    let mut stmt = store.connection().prepare(
        "SELECT id, tenant_id, organization_id, name, code, description, parent_team_id, created_at, updated_at
         FROM saas_teams
         WHERE id = ?1 AND tenant_id = ?2 AND organization_id = ?3
         LIMIT 1",
    )?;
    stmt.query_row(params![team_id, tenant_id, organization_id], |row| {
        Ok(Team {
            id: row.get(0)?,
            tenant_id: row.get(1)?,
            organization_id: row.get(2)?,
            name: row.get(3)?,
            code: row.get(4)?,
            description: row.get(5)?,
            parent_team_id: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })
    .optional()
    .context("load team for template instantiation")
}

fn load_selected_templates(
    store: &SaasSqliteStore,
    tenant_id: &str,
    template_ids: &[String],
) -> anyhow::Result<Vec<AgentTemplate>> {
    let repo = SaasTemplateRepository::new(store);
    let mut templates = repo.list_agent_templates(tenant_id)?;
    if template_ids.is_empty() {
        return Ok(templates);
    }

    templates.retain(|template| template_ids.iter().any(|id| id == &template.id));
    Ok(templates)
}

fn load_existing_instance(
    store: &SaasSqliteStore,
    tenant_id: &str,
    organization_id: &str,
    team_id: &str,
    template_id: &str,
) -> anyhow::Result<Option<AgentInstance>> {
    let mut stmt = store.connection().prepare(
        "SELECT id, tenant_id, organization_id, team_id, template_id, name, status, prompt_override, tool_ids_override_json, model_id_override, knowledge_base_ids_override_json, created_at, updated_at
         FROM saas_agent_instances
         WHERE tenant_id = ?1 AND organization_id = ?2 AND team_id = ?3 AND template_id = ?4
         LIMIT 1",
    )?;
    stmt.query_row(params![tenant_id, organization_id, team_id, template_id], |row| {
        let tool_ids_override_json: String = row.get(8)?;
        let knowledge_base_ids_override_json: Option<String> = row.get(10)?;
        Ok(AgentInstance {
            id: row.get(0)?,
            tenant_id: row.get(1)?,
            organization_id: row.get(2)?,
            team_id: row.get(3)?,
            template_id: row.get(4)?,
            name: row.get(5)?,
            status: deserialize_agent_status(&row.get::<_, String>(6)?),
            prompt_override: row.get(7)?,
            tool_ids_override: serde_json::from_str(&tool_ids_override_json).unwrap_or_default(),
            model_id_override: row.get(9)?,
            knowledge_base_ids_override: knowledge_base_ids_override_json
                .as_deref()
                .and_then(|value| serde_json::from_str(value).ok())
                .unwrap_or_default(),
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    })
    .optional()
    .context("load existing team agent instance")
}

fn deserialize_agent_status(status: &str) -> AgentInstanceStatus {
    match status {
        "disabled" => AgentInstanceStatus::Disabled,
        "archived" => AgentInstanceStatus::Archived,
        _ => AgentInstanceStatus::Active,
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::saas::models::{AgentTemplate, Organization, Team, Tenant, TenantStatus};

    #[test]
    fn test_instantiate_team_templates_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        let store = SaasSqliteStore::from_connection(conn).unwrap();
        let repo = SaasSeedRepository::new(&store);
        let now = chrono::Utc::now().to_rfc3339();

        repo.create_tenant(&Tenant {
            id: "tenant-1".to_string(),
            name: "Tenant 1".to_string(),
            status: TenantStatus::Active,
            created_at: now.clone(),
            updated_at: now.clone(),
        })
        .unwrap();
        repo.create_organization(&Organization {
            id: "org-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            name: "Org 1".to_string(),
            slug: Some("org-1".to_string()),
            industry: None,
            description: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        })
        .unwrap();
        repo.create_team(&Team {
            id: "team-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            organization_id: "org-1".to_string(),
            name: "Support".to_string(),
            code: Some("support".to_string()),
            description: None,
            parent_team_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        })
        .unwrap();
        repo.upsert_agent_template(&AgentTemplate {
            id: "template-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            name: "客服助手".to_string(),
            description: Some("desc".to_string()),
            prompt: Some("prompt".to_string()),
            tool_ids: vec!["search".to_string(), "cat".to_string()],
            model_id: Some("gpt-5.4-mini".to_string()),
            knowledge_base_ids: vec!["kb-support".to_string()],
            created_at: now.clone(),
            updated_at: now.clone(),
        })
        .unwrap();

        let req = TeamTemplateInstantiationRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: "org-1".to_string(),
            team_id: "team-1".to_string(),
            template_ids: vec!["template-1".to_string()],
        };
        let first = instantiate_team_templates(&store, &req).unwrap();
        let second = instantiate_team_templates(&store, &req).unwrap();

        assert_eq!(first.created_count, 1);
        assert_eq!(first.existing_count, 0);
        assert_eq!(first.instances.len(), 1);
        assert_eq!(first.instances[0].prompt_override.as_deref(), Some("prompt"));
        assert_eq!(
            first.instances[0].tool_ids_override,
            vec!["search".to_string(), "cat".to_string()]
        );
        assert_eq!(
            first.instances[0].model_id_override.as_deref(),
            Some("gpt-5.4-mini")
        );
        assert_eq!(
            first.instances[0].knowledge_base_ids_override,
            vec!["kb-support".to_string()]
        );
        assert_eq!(second.created_count, 0);
        assert_eq!(second.existing_count, 1);
        assert_eq!(second.instances.len(), 1);
        assert_eq!(first.instances[0].id, second.instances[0].id);
    }
}
