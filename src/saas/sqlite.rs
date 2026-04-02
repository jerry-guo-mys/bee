//! SaaS SQLite 存储引导
//!
//! 当前阶段先提供第一版 schema 初始化能力，后续再逐步补齐 repository 实现与旧数据迁移。

use std::path::Path;

use anyhow::Context;
use rusqlite::Connection;

const SAAS_SCHEMA_STATEMENTS: &[&str] = &[
    "PRAGMA foreign_keys = ON",
    "CREATE TABLE IF NOT EXISTS saas_tenants (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS saas_organizations (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        name TEXT NOT NULL,
        slug TEXT,
        industry TEXT,
        description TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS saas_teams (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        name TEXT NOT NULL,
        code TEXT,
        description TEXT,
        parent_team_id TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (parent_team_id) REFERENCES saas_teams(id) ON DELETE SET NULL
    )",
    "CREATE TABLE IF NOT EXISTS saas_users (
        id TEXT PRIMARY KEY,
        external_user_id TEXT,
        display_name TEXT NOT NULL,
        email TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS saas_memberships (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        user_id TEXT NOT NULL,
        team_id TEXT,
        role TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (user_id) REFERENCES saas_users(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL
    )",
    "CREATE TABLE IF NOT EXISTS saas_audit_logs (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT,
        team_id TEXT,
        user_id TEXT,
        action TEXT NOT NULL,
        resource_type TEXT NOT NULL,
        resource_id TEXT NOT NULL,
        detail_json TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE SET NULL,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL,
        FOREIGN KEY (user_id) REFERENCES saas_users(id) ON DELETE SET NULL
    )",
    "CREATE TABLE IF NOT EXISTS saas_tool_access_policies (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT,
        team_id TEXT,
        allowed_tool_ids_json TEXT NOT NULL,
        denied_tool_ids_json TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS saas_agent_templates (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        prompt TEXT,
        tool_ids_json TEXT,
        model_id TEXT,
        knowledge_base_ids_json TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS saas_agent_instances (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        team_id TEXT,
        template_id TEXT NOT NULL,
        name TEXT NOT NULL,
        status TEXT NOT NULL,
        prompt_override TEXT,
        tool_ids_override_json TEXT,
        model_id_override TEXT,
        knowledge_base_ids_override_json TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL,
        FOREIGN KEY (template_id) REFERENCES saas_agent_templates(id) ON DELETE RESTRICT
    )",
    "CREATE TABLE IF NOT EXISTS saas_workspaces (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        team_id TEXT,
        name TEXT NOT NULL,
        root_path TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL
    )",
    "CREATE TABLE IF NOT EXISTS saas_collaboration_groups (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        workspace_id TEXT,
        name TEXT,
        member_ids_json TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (workspace_id) REFERENCES saas_workspaces(id) ON DELETE SET NULL
    )",
    "CREATE TABLE IF NOT EXISTS saas_conversations (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        team_id TEXT,
        user_id TEXT NOT NULL,
        agent_instance_id TEXT,
        collaboration_group_id TEXT,
        title TEXT,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL,
        FOREIGN KEY (user_id) REFERENCES saas_users(id) ON DELETE RESTRICT,
        FOREIGN KEY (agent_instance_id) REFERENCES saas_agent_instances(id) ON DELETE SET NULL,
        FOREIGN KEY (collaboration_group_id) REFERENCES saas_collaboration_groups(id) ON DELETE SET NULL
    )",
    "CREATE TABLE IF NOT EXISTS saas_conversation_messages (
        id TEXT PRIMARY KEY,
        conversation_id TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        tool_name TEXT,
        metadata_json TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (conversation_id) REFERENCES saas_conversations(id) ON DELETE CASCADE
    )",
    "CREATE TABLE IF NOT EXISTS saas_tasks (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        team_id TEXT,
        project_id TEXT,
        parent_task_id TEXT,
        workspace_id TEXT,
        title TEXT NOT NULL,
        description TEXT,
        task_kind TEXT,
        artifacts_json TEXT,
        execution_json TEXT,
        review_report_json TEXT,
        assignee_agent_id TEXT,
        creator_user_id TEXT,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL,
        FOREIGN KEY (project_id) REFERENCES saas_projects(id) ON DELETE SET NULL,
        FOREIGN KEY (parent_task_id) REFERENCES saas_tasks(id) ON DELETE SET NULL,
        FOREIGN KEY (workspace_id) REFERENCES saas_workspaces(id) ON DELETE SET NULL,
        FOREIGN KEY (assignee_agent_id) REFERENCES saas_agent_instances(id) ON DELETE SET NULL,
        FOREIGN KEY (creator_user_id) REFERENCES saas_users(id) ON DELETE SET NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_saas_orgs_tenant ON saas_organizations(tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_teams_org ON saas_teams(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_memberships_org ON saas_memberships(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_audit_logs_tenant ON saas_audit_logs(tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_tool_policies_tenant ON saas_tool_access_policies(tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_agent_templates_tenant ON saas_agent_templates(tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_agent_instances_org ON saas_agent_instances(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_workspaces_org ON saas_workspaces(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_groups_org ON saas_collaboration_groups(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_conversations_org ON saas_conversations(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_messages_conversation ON saas_conversation_messages(conversation_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_tasks_org ON saas_tasks(organization_id)",
    // idx_saas_tasks_project / idx_saas_tasks_parent：旧库 saas_tasks 可能尚无 project_id、parent_task_id，
    // 须在 init_schema 末尾 ensure_column 之后再建索引（见 init_schema 尾部）。
    "CREATE TABLE IF NOT EXISTS saas_workflow_templates (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        slug TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        UNIQUE (tenant_id, slug)
    )",
    "CREATE TABLE IF NOT EXISTS saas_workflow_template_versions (
        id TEXT PRIMARY KEY,
        template_id TEXT NOT NULL,
        version INTEGER NOT NULL,
        definition_json TEXT NOT NULL,
        published_at TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (template_id) REFERENCES saas_workflow_templates(id) ON DELETE CASCADE,
        UNIQUE (template_id, version)
    )",
    "CREATE INDEX IF NOT EXISTS idx_saas_wf_templates_tenant ON saas_workflow_templates(tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_wf_versions_template ON saas_workflow_template_versions(template_id)",
    "CREATE TABLE IF NOT EXISTS saas_projects (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        team_id TEXT,
        name TEXT NOT NULL,
        description TEXT,
        workflow_run_id TEXT,
        pinned_workflow_template_id TEXT,
        pinned_template_version INTEGER,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_saas_projects_org ON saas_projects(organization_id)",
    "CREATE TABLE IF NOT EXISTS saas_task_spawn_idempotency (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        organization_id TEXT NOT NULL,
        team_id TEXT,
        parent_task_id TEXT NOT NULL,
        idempotency_key TEXT NOT NULL,
        child_task_ids_json TEXT NOT NULL,
        created_at TEXT NOT NULL,
        UNIQUE(parent_task_id, idempotency_key),
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL,
        FOREIGN KEY (parent_task_id) REFERENCES saas_tasks(id) ON DELETE CASCADE
    )",
    "CREATE INDEX IF NOT EXISTS idx_saas_spawn_parent ON saas_task_spawn_idempotency(parent_task_id)",
];

pub struct SaasSqliteStore {
    conn: Connection,
}

impl SaasSqliteStore {
    pub fn new(db_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let db_path = db_path.as_ref();
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("create SaaS database parent directory {}", parent.display())
                })?;
            }
        }
        let conn = Connection::open(db_path)
            .with_context(|| format!("open SQLite database {}", db_path.display()))?;
        let store = Self { conn };
        store
            .init_schema()
            .with_context(|| format!("init SaaS schema {}", db_path.display()))?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> anyhow::Result<Self> {
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn init_schema(&self) -> anyhow::Result<()> {
        for statement in SAAS_SCHEMA_STATEMENTS {
            self.conn.execute(statement, [])?;
        }
        ensure_column(
            &self.conn,
            "saas_agent_instances",
            "model_id_override",
            "TEXT",
        )?;
        ensure_column(
            &self.conn,
            "saas_agent_templates",
            "knowledge_base_ids_json",
            "TEXT",
        )?;
        ensure_column(
            &self.conn,
            "saas_agent_instances",
            "knowledge_base_ids_override_json",
            "TEXT",
        )?;
        // Workbench / TaskRepository（M3）：与 API Task 对齐的扩展列
        ensure_column(&self.conn, "saas_tasks", "workflow_run_id", "TEXT")?;
        ensure_column(&self.conn, "saas_tasks", "workflow_template_id", "TEXT")?;
        ensure_column(
            &self.conn,
            "saas_tasks",
            "workflow_template_version",
            "INTEGER",
        )?;
        ensure_column(&self.conn, "saas_tasks", "assignee_ids_json", "TEXT")?;
        ensure_column(&self.conn, "saas_tasks", "group_id", "TEXT")?;
        ensure_column(&self.conn, "saas_tasks", "coordinator_id", "TEXT")?;
        ensure_column(
            &self.conn,
            "saas_tasks",
            "internal_group",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(&self.conn, "saas_tasks", "project_id", "TEXT")?;
        ensure_column(&self.conn, "saas_tasks", "parent_task_id", "TEXT")?;
        ensure_column(&self.conn, "saas_tasks", "task_kind", "TEXT")?;
        ensure_column(&self.conn, "saas_tasks", "artifacts_json", "TEXT")?;
        ensure_column(&self.conn, "saas_tasks", "execution_json", "TEXT")?;
        ensure_column(&self.conn, "saas_tasks", "review_report_json", "TEXT")?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_saas_tasks_project ON saas_tasks(project_id)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_saas_tasks_parent ON saas_tasks(parent_task_id)",
            [],
        )?;
        Ok(())
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    // ==================== Tenant 相关方法 ====================

    pub fn list_tenants(&self) -> anyhow::Result<Vec<crate::saas::Tenant>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, created_at, updated_at FROM saas_tenants ORDER BY created_at DESC"
        )?;
        let tenants = stmt.query_map([], |row| {
            Ok(crate::saas::Tenant {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        tenants
            .collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_tenant(&self, id: &str) -> anyhow::Result<Option<crate::saas::Tenant>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, created_at, updated_at FROM saas_tenants WHERE id = ?",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(crate::saas::Tenant {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn create_tenant(&self, tenant: &crate::saas::Tenant) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO saas_tenants (id, name, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            [
                &tenant.id,
                &tenant.name,
                &tenant.status.to_string(),
                &tenant.created_at,
                &tenant.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_tenant_status(
        &self,
        id: &str,
        status: &crate::saas::TenantStatus,
    ) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE saas_tenants SET status = ?, updated_at = datetime('now') WHERE id = ?",
            [&status.to_string(), id],
        )?;
        Ok(())
    }

    // ==================== Organization 相关方法 ====================

    pub fn list_organizations(
        &self,
        tenant_id: Option<&str>,
    ) -> anyhow::Result<Vec<crate::saas::Organization>> {
        let sql = match tenant_id {
            Some(tid) => format!(
                "SELECT id, tenant_id, name, slug, industry, description, created_at, updated_at FROM saas_organizations WHERE tenant_id = '{}' ORDER BY created_at DESC",
                tid.replace('\'', "''")
            ),
            None => "SELECT id, tenant_id, name, slug, industry, description, created_at, updated_at FROM saas_organizations ORDER BY created_at DESC".to_string(),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let orgs = stmt.query_map([], |row| {
            Ok(crate::saas::Organization {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                name: row.get(2)?,
                slug: row.get(3)?,
                industry: row.get(4)?,
                description: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        orgs.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_organization(&self, id: &str) -> anyhow::Result<Option<crate::saas::Organization>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tenant_id, name, slug, industry, description, created_at, updated_at FROM saas_organizations WHERE id = ?"
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(crate::saas::Organization {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                name: row.get(2)?,
                slug: row.get(3)?,
                industry: row.get(4)?,
                description: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn create_organization(&self, org: &crate::saas::Organization) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO saas_organizations (id, tenant_id, name, slug, industry, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [
                &org.id,
                &org.tenant_id,
                &org.name,
                org.slug.as_deref().unwrap_or(""),
                org.industry.as_deref().unwrap_or(""),
                org.description.as_deref().unwrap_or(""),
                &org.created_at,
                &org.updated_at,
            ],
        )?;
        Ok(())
    }

    // ==================== Team 相关方法 ====================

    pub fn list_teams(
        &self,
        organization_id: Option<&str>,
    ) -> anyhow::Result<Vec<crate::saas::Team>> {
        let sql = match organization_id {
            Some(org_id) => format!(
                "SELECT id, tenant_id, organization_id, name, code, description, parent_team_id, created_at, updated_at FROM saas_teams WHERE organization_id = '{}' ORDER BY created_at DESC",
                org_id.replace('\'', "''")
            ),
            None => "SELECT id, tenant_id, organization_id, name, code, description, parent_team_id, created_at, updated_at FROM saas_teams ORDER BY created_at DESC".to_string(),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let teams = stmt.query_map([], |row| {
            Ok(crate::saas::Team {
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
        })?;
        teams
            .collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_team(&self, id: &str) -> anyhow::Result<Option<crate::saas::Team>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tenant_id, organization_id, name, code, description, parent_team_id, created_at, updated_at FROM saas_teams WHERE id = ?"
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(crate::saas::Team {
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
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn create_team(&self, team: &crate::saas::Team) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO saas_teams (id, tenant_id, organization_id, name, code, description, parent_team_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                &team.id,
                &team.tenant_id,
                &team.organization_id,
                &team.name,
                team.code.as_deref().unwrap_or(""),
                team.description.as_deref().unwrap_or(""),
                team.parent_team_id.as_deref().unwrap_or(""),
                &team.created_at,
                &team.updated_at,
            ],
        )?;
        Ok(())
    }

    // ==================== Membership 相关方法 ====================

    pub fn list_memberships(
        &self,
        tenant_id: Option<&str>,
        organization_id: Option<&str>,
    ) -> anyhow::Result<Vec<crate::saas::Membership>> {
        let mut conditions = Vec::new();
        let mut params: Vec<&str> = Vec::new();

        if let Some(tid) = tenant_id {
            conditions.push("tenant_id = ?");
            params.push(tid);
        }
        if let Some(org_id) = organization_id {
            conditions.push("organization_id = ?");
            params.push(org_id);
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, tenant_id, organization_id, user_id, team_id, role, created_at, updated_at FROM saas_memberships{} ORDER BY created_at DESC",
            where_clause
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let memberships = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(crate::saas::Membership {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                organization_id: row.get(2)?,
                user_id: row.get(3)?,
                team_id: row.get(4)?,
                role: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        memberships
            .collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_membership(&self, id: &str) -> anyhow::Result<Option<crate::saas::Membership>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tenant_id, organization_id, user_id, team_id, role, created_at, updated_at FROM saas_memberships WHERE id = ?"
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(crate::saas::Membership {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                organization_id: row.get(2)?,
                user_id: row.get(3)?,
                team_id: row.get(4)?,
                role: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn create_membership(&self, membership: &crate::saas::Membership) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO saas_memberships (id, tenant_id, organization_id, user_id, team_id, role, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [
                &membership.id,
                &membership.tenant_id,
                &membership.organization_id,
                &membership.user_id,
                membership.team_id.as_deref().unwrap_or(""),
                &membership.role.to_string(),
                &membership.created_at,
                &membership.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_membership_role(
        &self,
        id: &str,
        role: &crate::saas::MembershipRole,
    ) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE saas_memberships SET role = ?, updated_at = datetime('now') WHERE id = ?",
            [&role.to_string(), id],
        )?;
        Ok(())
    }

    // ==================== Audit Log 相关方法 ====================

    pub fn list_audit_logs(
        &self,
        tenant_id: Option<&str>,
        organization_id: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<crate::saas::AuditLogRecord>> {
        let mut conditions = Vec::new();
        let mut params: Vec<&str> = Vec::new();

        if let Some(tid) = tenant_id {
            conditions.push("tenant_id = ?");
            params.push(tid);
        }
        if let Some(org_id) = organization_id {
            conditions.push("organization_id = ?");
            params.push(org_id);
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, tenant_id, organization_id, team_id, user_id, action, resource_type, resource_id, detail_json, created_at FROM saas_audit_logs{} ORDER BY created_at DESC LIMIT ?",
            where_clause
        );
        let limit_str = limit.to_string();
        params.push(&limit_str);

        let mut stmt = self.conn.prepare(&sql)?;
        let logs = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(crate::saas::AuditLogRecord {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                organization_id: row.get(2)?,
                team_id: row.get(3)?,
                user_id: row.get(4)?,
                action: row.get(5)?,
                resource_type: row.get(6)?,
                resource_id: row.get(7)?,
                detail_json: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        logs.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    // ==================== 工作流模板（工作台 M2） ====================

    /// 按团队 + Agent 模板解析一个活跃实例 ID（用于任务 assignee）
    pub fn find_agent_instance_for_team_template(
        &self,
        team_id: &str,
        agent_template_id: &str,
    ) -> anyhow::Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM saas_agent_instances WHERE team_id = ? AND template_id = ? AND status = 'active' LIMIT 1",
        )?;
        let mut rows =
            stmt.query_map([team_id, agent_template_id], |row| row.get::<_, String>(0))?;
        Ok(rows.next().transpose()?)
    }

    /// 某租户下所有已发布版本（每模板取最高已发布 version）
    pub fn list_published_workflow_templates(
        &self,
        tenant_id: &str,
    ) -> anyhow::Result<Vec<(crate::saas::WorkflowTemplateRecord, i32, String)>> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT t.id, t.tenant_id, t.slug, t.name, t.description, t.status, t.created_at, t.updated_at,
                   v.version, v.definition_json
            FROM saas_workflow_templates t
            INNER JOIN saas_workflow_template_versions v ON v.template_id = t.id
            WHERE t.tenant_id = ?1
              AND t.status != 'archived'
              AND v.published_at IS NOT NULL
              AND v.version = (
                SELECT MAX(v2.version) FROM saas_workflow_template_versions v2
                WHERE v2.template_id = t.id AND v2.published_at IS NOT NULL
              )
            ORDER BY t.slug
            ",
        )?;
        let rows = stmt.query_map([tenant_id], |row| {
            Ok((
                crate::saas::WorkflowTemplateRecord {
                    id: row.get(0)?,
                    tenant_id: row.get(1)?,
                    slug: row.get(2)?,
                    name: row.get(3)?,
                    description: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                },
                row.get::<_, i64>(8)? as i32,
                row.get::<_, String>(9)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// 管理端：含 draft、未发布版本
    pub fn list_workflow_templates_for_tenant(
        &self,
        tenant_id: &str,
    ) -> anyhow::Result<Vec<crate::saas::WorkflowTemplateRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tenant_id, slug, name, description, status, created_at, updated_at
             FROM saas_workflow_templates WHERE tenant_id = ?1 ORDER BY slug",
        )?;
        let rows = stmt.query_map([tenant_id], |row| {
            Ok(crate::saas::WorkflowTemplateRecord {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                slug: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn list_workflow_template_versions(
        &self,
        template_id: &str,
    ) -> anyhow::Result<Vec<crate::saas::WorkflowTemplateVersionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, template_id, version, definition_json, published_at, created_at
             FROM saas_workflow_template_versions WHERE template_id = ?1 ORDER BY version",
        )?;
        let rows = stmt.query_map([template_id], |row| {
            Ok(crate::saas::WorkflowTemplateVersionRecord {
                id: row.get(0)?,
                template_id: row.get(1)?,
                version: row.get::<_, i64>(2)? as i32,
                definition_json: row.get(3)?,
                published_at: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// 解析用于启动 run：指定 slug + 可选版本；无版本则取最新已发布
    pub fn resolve_published_workflow_for_start(
        &self,
        tenant_id: &str,
        slug: &str,
        version: Option<i32>,
    ) -> anyhow::Result<Option<(String, i32, crate::saas::WorkflowDefinitionJson)>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id FROM saas_workflow_templates t WHERE t.tenant_id = ?1 AND t.slug = ?2 LIMIT 1",
        )?;
        let template_uuid: String = match stmt.query_row([tenant_id, slug], |row| row.get(0)) {
            Ok(id) => id,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let (ver, def_str): (i32, String) = if let Some(v) = version {
            match self.conn.query_row(
                "SELECT version, definition_json FROM saas_workflow_template_versions
                 WHERE template_id = ?1 AND version = ?2 AND published_at IS NOT NULL",
                rusqlite::params![&template_uuid, v],
                |row| Ok((row.get::<_, i64>(0)? as i32, row.get::<_, String>(1)?)),
            ) {
                Ok(x) => x,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    anyhow::bail!("workflow version {v} not found or not published")
                }
                Err(e) => return Err(e.into()),
            }
        } else {
            match self.conn.query_row(
                "SELECT version, definition_json FROM saas_workflow_template_versions
                 WHERE template_id = ?1 AND published_at IS NOT NULL
                 ORDER BY version DESC LIMIT 1",
                [&template_uuid],
                |row| Ok((row.get::<_, i64>(0)? as i32, row.get::<_, String>(1)?)),
            ) {
                Ok(x) => x,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    anyhow::bail!("no published workflow version for this template")
                }
                Err(e) => return Err(e.into()),
            }
        };

        let def = crate::saas::WorkflowDefinitionJson::parse(&def_str)?;
        Ok(Some((template_uuid, ver, def)))
    }

    pub fn create_workflow_template(
        &self,
        record: &crate::saas::WorkflowTemplateRecord,
        definition: &crate::saas::WorkflowDefinitionJson,
    ) -> anyhow::Result<()> {
        let def_str = serde_json::to_string(definition)?;
        let now = chrono::Utc::now().to_rfc3339();
        let ver_id = uuid::Uuid::new_v4().to_string();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO saas_workflow_templates (id, tenant_id, slug, name, description, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &record.id,
                &record.tenant_id,
                &record.slug,
                &record.name,
                record.description.as_deref().unwrap_or(""),
                &record.status,
                &record.created_at,
                &record.updated_at,
            ],
        )?;
        tx.execute(
            "INSERT INTO saas_workflow_template_versions (id, template_id, version, definition_json, published_at, created_at)
             VALUES (?1, ?2, 1, ?3, NULL, ?4)",
            rusqlite::params![&ver_id, &record.id, &def_str, &now],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_workflow_template_version(
        &self,
        template_id: &str,
        definition: &crate::saas::WorkflowDefinitionJson,
    ) -> anyhow::Result<i32> {
        let next_v: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM saas_workflow_template_versions WHERE template_id = ?1",
            [template_id],
            |row| row.get::<_, i64>(0),
        )? as i32;
        let def_str = serde_json::to_string(definition)?;
        let now = chrono::Utc::now().to_rfc3339();
        let ver_id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO saas_workflow_template_versions (id, template_id, version, definition_json, published_at, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
            rusqlite::params![&ver_id, template_id, next_v, &def_str, &now],
        )?;
        self.conn.execute(
            "UPDATE saas_workflow_templates SET updated_at = ?1 WHERE id = ?2",
            [&now, template_id],
        )?;
        Ok(next_v)
    }

    pub fn publish_workflow_template_version(
        &self,
        template_id: &str,
        version: i32,
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let n = self.conn.execute(
            "UPDATE saas_workflow_template_versions SET published_at = ?1
             WHERE template_id = ?2 AND version = ?3 AND published_at IS NULL",
            rusqlite::params![&now, template_id, version],
        )?;
        if n == 0 {
            anyhow::bail!("version not found or already published");
        }
        self.conn.execute(
            "UPDATE saas_workflow_templates SET status = 'published', updated_at = ?1 WHERE id = ?2",
            [&now, template_id],
        )?;
        Ok(())
    }

    pub fn get_workflow_template_by_id(
        &self,
        id: &str,
    ) -> anyhow::Result<Option<crate::saas::WorkflowTemplateRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tenant_id, slug, name, description, status, created_at, updated_at
             FROM saas_workflow_templates WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(crate::saas::WorkflowTemplateRecord {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                slug: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }
}

pub fn init_saas_sqlite(db_path: impl AsRef<Path>) -> anyhow::Result<SaasSqliteStore> {
    SaasSqliteStore::new(db_path)
}

fn ensure_column(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
    column_definition: &str,
) -> anyhow::Result<()> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut stmt = conn.prepare(&pragma)?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == column_name {
            return Ok(());
        }
    }

    let alter = format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {column_definition}");
    conn.execute(&alter, [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_saas_schema_in_memory() {
        let conn = Connection::open_in_memory().unwrap();
        let store = SaasSqliteStore::from_connection(conn).unwrap();

        let count: i64 = store
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE 'saas_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(count >= 12, "expected SaaS tables to be created");
    }

    /// 旧库仅有「无 project_id / parent_task_id」的 saas_tasks 时，init_schema 不得在建索引阶段失败。
    #[test]
    fn legacy_saas_tasks_without_project_columns_migrates() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("saas.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                r"
            PRAGMA foreign_keys = OFF;
            CREATE TABLE saas_tenants (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE saas_organizations (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                name TEXT NOT NULL,
                slug TEXT,
                industry TEXT,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id)
            );
            CREATE TABLE saas_tasks (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                organization_id TEXT NOT NULL,
                team_id TEXT,
                workspace_id TEXT,
                title TEXT NOT NULL,
                description TEXT,
                assignee_agent_id TEXT,
                creator_user_id TEXT,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id),
                FOREIGN KEY (organization_id) REFERENCES saas_organizations(id)
            );
            ",
            )
            .unwrap();
        }

        let store = SaasSqliteStore::new(&db_path).expect("migrate legacy saas_tasks");
        let n: i64 = store
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('saas_tasks') WHERE name = 'project_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn test_workflow_template_create_publish() {
        let conn = Connection::open_in_memory().unwrap();
        let store = SaasSqliteStore::from_connection(conn).unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let tid = "t1";
        store
            .create_tenant(&crate::saas::Tenant {
                id: tid.to_string(),
                name: "T".to_string(),
                status: crate::saas::TenantStatus::Active,
                created_at: now.clone(),
                updated_at: now.clone(),
            })
            .unwrap();
        let def = crate::saas::WorkflowDefinitionJson {
            steps: vec![crate::saas::WorkflowDefinitionStep {
                key: None,
                title: "一步".to_string(),
                task_kind: None,
                default_agent_template_id: None,
                instructions: None,
            }],
            team_filter: None,
        };
        let rec = crate::saas::WorkflowTemplateRecord {
            id: "wf1".to_string(),
            tenant_id: tid.to_string(),
            slug: "my_flow".to_string(),
            name: "My".to_string(),
            description: None,
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        store.create_workflow_template(&rec, &def).unwrap();
        store.publish_workflow_template_version("wf1", 1).unwrap();
        let resolved = store
            .resolve_published_workflow_for_start(tid, "my_flow", None)
            .unwrap();
        assert!(resolved.is_some());
        let (_, ver, d) = resolved.unwrap();
        assert_eq!(ver, 1);
        assert_eq!(d.steps.len(), 1);
    }
}
