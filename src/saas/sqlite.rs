//! SaaS SQLite 存储引导
//!
//! 当前阶段先提供第一版 schema 初始化能力，后续再逐步补齐 repository 实现与旧数据迁移。

use std::path::Path;

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
    "CREATE TABLE IF NOT EXISTS saas_agent_templates (
        id TEXT PRIMARY KEY,
        tenant_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        prompt TEXT,
        tool_ids_json TEXT,
        model_id TEXT,
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
        workspace_id TEXT,
        title TEXT NOT NULL,
        description TEXT,
        assignee_agent_id TEXT,
        creator_user_id TEXT,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (tenant_id) REFERENCES saas_tenants(id) ON DELETE CASCADE,
        FOREIGN KEY (organization_id) REFERENCES saas_organizations(id) ON DELETE CASCADE,
        FOREIGN KEY (team_id) REFERENCES saas_teams(id) ON DELETE SET NULL,
        FOREIGN KEY (workspace_id) REFERENCES saas_workspaces(id) ON DELETE SET NULL,
        FOREIGN KEY (assignee_agent_id) REFERENCES saas_agent_instances(id) ON DELETE SET NULL,
        FOREIGN KEY (creator_user_id) REFERENCES saas_users(id) ON DELETE SET NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_saas_orgs_tenant ON saas_organizations(tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_teams_org ON saas_teams(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_memberships_org ON saas_memberships(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_agent_templates_tenant ON saas_agent_templates(tenant_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_agent_instances_org ON saas_agent_instances(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_workspaces_org ON saas_workspaces(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_groups_org ON saas_collaboration_groups(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_conversations_org ON saas_conversations(organization_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_messages_conversation ON saas_conversation_messages(conversation_id)",
    "CREATE INDEX IF NOT EXISTS idx_saas_tasks_org ON saas_tasks(organization_id)",
];

pub struct SaasSqliteStore {
    conn: Connection,
}

impl SaasSqliteStore {
    pub fn new(db_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        let store = Self { conn };
        store.init_schema()?;
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
        Ok(())
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

pub fn init_saas_sqlite(db_path: impl AsRef<Path>) -> anyhow::Result<SaasSqliteStore> {
    SaasSqliteStore::new(db_path)
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

        assert!(count >= 10, "expected SaaS tables to be created");
    }
}
