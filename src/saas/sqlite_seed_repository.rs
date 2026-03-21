//! SaaS sqlite 最小写入实现
//!
//! 当前仅覆盖 Phase 2 组织初始化所需的写路径，完整 repository 后续再补齐。

use rusqlite::params;

use crate::saas::models::{AgentInstance, AgentTemplate, Organization, Team, Tenant, Workspace};
use crate::saas::sqlite::SaasSqliteStore;

pub struct SaasSeedRepository<'a> {
    store: &'a SaasSqliteStore,
}

impl<'a> SaasSeedRepository<'a> {
    pub fn new(store: &'a SaasSqliteStore) -> Self {
        Self { store }
    }

    pub fn create_tenant(&self, tenant: &Tenant) -> anyhow::Result<()> {
        self.store.connection().execute(
            "INSERT OR REPLACE INTO saas_tenants (id, name, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                tenant.id,
                tenant.name,
                serialize_tenant_status(&tenant.status),
                tenant.created_at,
                tenant.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn create_organization(&self, organization: &Organization) -> anyhow::Result<()> {
        self.store.connection().execute(
            "INSERT OR REPLACE INTO saas_organizations
             (id, tenant_id, name, slug, industry, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                organization.id,
                organization.tenant_id,
                organization.name,
                organization.slug,
                organization.industry,
                organization.description,
                organization.created_at,
                organization.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn create_team(&self, team: &Team) -> anyhow::Result<()> {
        self.store.connection().execute(
            "INSERT OR REPLACE INTO saas_teams
             (id, tenant_id, organization_id, name, code, description, parent_team_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                team.id,
                team.tenant_id,
                team.organization_id,
                team.name,
                team.code,
                team.description,
                team.parent_team_id,
                team.created_at,
                team.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn upsert_agent_template(&self, template: &AgentTemplate) -> anyhow::Result<()> {
        self.store.connection().execute(
            "INSERT OR REPLACE INTO saas_agent_templates
             (id, tenant_id, name, description, prompt, tool_ids_json, model_id, knowledge_base_ids_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                template.id,
                template.tenant_id,
                template.name,
                template.description,
                template.prompt,
                serde_json::to_string(&template.tool_ids)?,
                template.model_id,
                serde_json::to_string(&template.knowledge_base_ids)?,
                template.created_at,
                template.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn create_agent_instance(&self, instance: &AgentInstance) -> anyhow::Result<()> {
        self.store.connection().execute(
            "INSERT OR REPLACE INTO saas_agent_instances
             (id, tenant_id, organization_id, team_id, template_id, name, status, prompt_override, tool_ids_override_json, model_id_override, knowledge_base_ids_override_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                instance.id,
                instance.tenant_id,
                instance.organization_id,
                instance.team_id,
                instance.template_id,
                instance.name,
                serialize_agent_status(&instance.status),
                instance.prompt_override,
                serde_json::to_string(&instance.tool_ids_override)?,
                instance.model_id_override,
                serde_json::to_string(&instance.knowledge_base_ids_override)?,
                instance.created_at,
                instance.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn create_workspace(&self, workspace: &Workspace) -> anyhow::Result<()> {
        self.store.connection().execute(
            "INSERT OR REPLACE INTO saas_workspaces
             (id, tenant_id, organization_id, team_id, name, root_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                workspace.id,
                workspace.tenant_id,
                workspace.organization_id,
                workspace.team_id,
                workspace.name,
                workspace.root_path,
                workspace.created_at,
                workspace.updated_at
            ],
        )?;
        Ok(())
    }
}

fn serialize_tenant_status(status: &crate::saas::models::TenantStatus) -> &'static str {
    match status {
        crate::saas::models::TenantStatus::Active => "active",
        crate::saas::models::TenantStatus::Suspended => "suspended",
        crate::saas::models::TenantStatus::Archived => "archived",
    }
}

fn serialize_agent_status(status: &crate::saas::models::AgentInstanceStatus) -> &'static str {
    match status {
        crate::saas::models::AgentInstanceStatus::Active => "active",
        crate::saas::models::AgentInstanceStatus::Disabled => "disabled",
        crate::saas::models::AgentInstanceStatus::Archived => "archived",
    }
}
