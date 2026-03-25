//! SaaS 旧数据迁移引导
//!
//! 将当前 workspace 下的文件型业务数据导入 Phase 1 的 sqlite schema，
//! 作为正式 repository 实现前的过渡迁移层。

use std::path::Path;

use anyhow::Context;
use rusqlite::params;

use crate::memory::Role;
use crate::saas::sqlite::SaasSqliteStore;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub tenants_seeded: usize,
    pub organizations_seeded: usize,
    pub workspaces_seeded: usize,
    pub users_seeded: usize,
    pub agent_templates_seeded: usize,
    pub agent_instances_imported: usize,
    pub groups_imported: usize,
    pub conversations_imported: usize,
    pub conversation_messages_imported: usize,
    pub tasks_imported: usize,
}

pub struct LegacyWorkspaceImporter<'a> {
    store: &'a SaasSqliteStore,
}

impl<'a> LegacyWorkspaceImporter<'a> {
    pub fn new(store: &'a SaasSqliteStore) -> Self {
        Self { store }
    }

    pub fn import_workspace(
        &self,
        workspace: &Path,
        tenant_id: &str,
        organization_id: &str,
        workspace_id: &str,
    ) -> anyhow::Result<MigrationReport> {
        let mut report = MigrationReport::default();
        self.seed_default_scope(
            workspace,
            tenant_id,
            organization_id,
            workspace_id,
            &mut report,
        )?;
        self.import_dynamic_agents(workspace, tenant_id, organization_id, &mut report)?;
        self.import_groups(
            workspace,
            tenant_id,
            organization_id,
            workspace_id,
            &mut report,
        )?;
        self.import_tasks(
            workspace,
            tenant_id,
            organization_id,
            workspace_id,
            &mut report,
        )?;
        self.import_sessions(workspace, tenant_id, organization_id, &mut report)?;
        Ok(report)
    }

    fn seed_default_scope(
        &self,
        workspace: &Path,
        tenant_id: &str,
        organization_id: &str,
        workspace_id: &str,
        report: &mut MigrationReport,
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.store.connection();

        let tenant_inserted = conn.execute(
            "INSERT OR IGNORE INTO saas_tenants (id, name, status, created_at, updated_at) VALUES (?1, ?2, 'active', ?3, ?3)",
            params![tenant_id, "Default Tenant", now],
        )?;
        report.tenants_seeded += tenant_inserted as usize;

        let org_inserted = conn.execute(
            "INSERT OR IGNORE INTO saas_organizations (id, tenant_id, name, slug, industry, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?6)",
            params![
                organization_id,
                tenant_id,
                "Default Organization",
                "default-org",
                format!("Imported from workspace {}", workspace.display()),
                now
            ],
        )?;
        report.organizations_seeded += org_inserted as usize;

        let workspace_inserted = conn.execute(
            "INSERT OR IGNORE INTO saas_workspaces (id, tenant_id, organization_id, team_id, name, root_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?6)",
            params![
                workspace_id,
                tenant_id,
                organization_id,
                "Default Workspace",
                workspace.display().to_string(),
                now
            ],
        )?;
        report.workspaces_seeded += workspace_inserted as usize;

        let user_inserted = conn.execute(
            "INSERT OR IGNORE INTO saas_users (id, external_user_id, display_name, email, created_at, updated_at)
             VALUES ('legacy-user', NULL, 'Legacy Imported User', NULL, ?1, ?1)",
            params![now],
        )?;
        report.users_seeded += user_inserted as usize;
        conn.execute(
            "INSERT OR IGNORE INTO saas_memberships
             (id, tenant_id, organization_id, user_id, team_id, role, created_at, updated_at)
             VALUES ('legacy-user-default-membership', ?1, ?2, 'legacy-user', NULL, 'org_admin', ?3, ?3)",
            params![tenant_id, organization_id, now],
        )?;

        let template_inserted = conn.execute(
            "INSERT OR IGNORE INTO saas_agent_templates (id, tenant_id, name, description, prompt, tool_ids_json, model_id, created_at, updated_at)
             VALUES ('legacy-dynamic-agent', ?1, 'Legacy Dynamic Agent Template', 'Imported from legacy agents.json', NULL, '[]', NULL, ?2, ?2)",
            params![tenant_id, now],
        )?;
        report.agent_templates_seeded += template_inserted as usize;

        let default_template_inserted = conn.execute(
            "INSERT OR IGNORE INTO saas_agent_templates (id, tenant_id, name, description, prompt, tool_ids_json, model_id, created_at, updated_at)
             VALUES ('default', ?1, 'Default Assistant Template', 'Seeded for legacy session imports', NULL, '[]', NULL, ?2, ?2)",
            params![tenant_id, now],
        )?;
        report.agent_templates_seeded += default_template_inserted as usize;

        conn.execute(
            "INSERT OR IGNORE INTO saas_agent_instances
             (id, tenant_id, organization_id, team_id, template_id, name, status, prompt_override, tool_ids_override_json, model_id_override, knowledge_base_ids_override_json, created_at, updated_at)
             VALUES ('default', ?1, ?2, NULL, 'default', 'Default Assistant', 'active', NULL, '[]', NULL, '[]', ?3, ?3)",
            params![tenant_id, organization_id, now],
        )?;

        Ok(())
    }

    fn ensure_agent_instance_exists(
        &self,
        tenant_id: &str,
        organization_id: &str,
        agent_instance_id: &str,
        now: &str,
    ) -> anyhow::Result<()> {
        let conn = self.store.connection();
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM saas_agent_instances WHERE id = ?1",
            params![agent_instance_id],
            |row| row.get(0),
        )?;
        if exists > 0 {
            return Ok(());
        }

        let display_name = if agent_instance_id == "default" {
            "Default Assistant".to_string()
        } else if agent_instance_id == "auto" {
            "Auto Router".to_string()
        } else {
            format!("Legacy Agent {}", agent_instance_id)
        };

        conn.execute(
            "INSERT OR IGNORE INTO saas_agent_instances
             (id, tenant_id, organization_id, team_id, template_id, name, status, prompt_override, tool_ids_override_json, model_id_override, knowledge_base_ids_override_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, 'legacy-dynamic-agent', ?4, 'active', NULL, '[]', NULL, '[]', ?5, ?5)",
            params![agent_instance_id, tenant_id, organization_id, display_name, now],
        )?;
        Ok(())
    }

    fn import_groups(
        &self,
        workspace: &Path,
        tenant_id: &str,
        organization_id: &str,
        workspace_id: &str,
        report: &mut MigrationReport,
    ) -> anyhow::Result<()> {
        let path = workspace.join("groups.json");
        if !path.exists() {
            return Ok(());
        }
        let groups: std::collections::HashMap<String, LegacyGroupRecord> = serde_json::from_str(
            &std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
        )
        .with_context(|| format!("parse {}", path.display()))?;

        let conn = self.store.connection();
        for (_, group) in groups {
            conn.execute(
                "INSERT OR IGNORE INTO saas_collaboration_groups
                 (id, tenant_id, organization_id, workspace_id, name, member_ids_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    group.id,
                    tenant_id,
                    organization_id,
                    workspace_id,
                    group.name,
                    serde_json::to_string(&group.member_ids)?,
                    group.created_at
                ],
            )?;
            report.groups_imported += 1;

            let group_session_path = workspace
                .join("sessions")
                .join(format!("group_{}.json", sanitize_group_id(&group.id)));
            if !group_session_path.exists() {
                continue;
            }

            let snapshot: LegacyGroupSessionSnapshot = serde_json::from_str(
                &std::fs::read_to_string(&group_session_path)
                    .with_context(|| format!("read {}", group_session_path.display()))?,
            )
            .with_context(|| format!("parse {}", group_session_path.display()))?;

            let conv_inserted = conn.execute(
                "INSERT OR IGNORE INTO saas_conversations
                 (id, tenant_id, organization_id, team_id, user_id, agent_instance_id, collaboration_group_id, title, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, 'legacy-user', NULL, ?4, ?5, 'active', ?6, ?6)",
                params![
                    format!("group:{}", group.id),
                    tenant_id,
                    organization_id,
                    group.id,
                    group.name,
                    group.created_at
                ],
            )?;
            report.conversations_imported += conv_inserted as usize;

            for (index, message) in snapshot.messages.into_iter().enumerate() {
                let role = if message.role == "user" {
                    "user"
                } else {
                    "assistant"
                };
                let metadata_json = message.assistant_id.map(|assistant_id| {
                    serde_json::json!({ "assistant_id": assistant_id }).to_string()
                });
                let inserted = conn.execute(
                    "INSERT OR IGNORE INTO saas_conversation_messages
                     (id, conversation_id, role, content, tool_name, metadata_json, created_at)
                     VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
                    params![
                        format!("group:{}-{}", group.id, index),
                        format!("group:{}", group.id),
                        role,
                        message.content,
                        metadata_json,
                        group.created_at
                    ],
                )?;
                report.conversation_messages_imported += inserted as usize;
            }
        }

        Ok(())
    }

    fn import_dynamic_agents(
        &self,
        workspace: &Path,
        tenant_id: &str,
        organization_id: &str,
        report: &mut MigrationReport,
    ) -> anyhow::Result<()> {
        let path = workspace.join("agents.json");
        if !path.exists() {
            return Ok(());
        }
        let agents: Vec<LegacyDynamicAgent> = serde_json::from_str(
            &std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
        )
        .with_context(|| format!("parse {}", path.display()))?;

        let conn = self.store.connection();
        for agent in agents {
            let inserted = conn.execute(
                "INSERT OR IGNORE INTO saas_agent_instances
                 (id, tenant_id, organization_id, team_id, template_id, name, status, prompt_override, tool_ids_override_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, 'legacy-dynamic-agent', ?4, 'active', ?5, '[]', ?6, ?6)",
                params![
                    agent.id,
                    tenant_id,
                    organization_id,
                    agent.role,
                    agent.guidance,
                    agent.created_at
                ],
            )?;
            report.agent_instances_imported += inserted as usize;
        }

        Ok(())
    }

    fn import_tasks(
        &self,
        workspace: &Path,
        tenant_id: &str,
        organization_id: &str,
        workspace_id: &str,
        report: &mut MigrationReport,
    ) -> anyhow::Result<()> {
        let path = workspace.join("tasks.json");
        if !path.exists() {
            return Ok(());
        }
        let tasks: Vec<LegacyTaskRecord> = serde_json::from_str(
            &std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
        )
        .with_context(|| format!("parse {}", path.display()))?;

        let conn = self.store.connection();
        for task in tasks {
            if let Some(agent_id) = task.coordinator_id.as_deref() {
                self.ensure_agent_instance_exists(
                    tenant_id,
                    organization_id,
                    agent_id,
                    &task.created_at,
                )?;
            }
            let inserted = conn.execute(
                "INSERT OR IGNORE INTO saas_tasks
                 (id, tenant_id, organization_id, team_id, workspace_id, title, description, assignee_agent_id, creator_user_id, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, 'legacy-user', ?8, ?9, ?10)",
                params![
                    task.id,
                    tenant_id,
                    organization_id,
                    workspace_id,
                    task.title,
                    task.description,
                    task.coordinator_id,
                    map_task_status(&task.status),
                    task.created_at,
                    task.updated_at
                ],
            )?;
            report.tasks_imported += inserted as usize;
        }

        Ok(())
    }

    fn import_sessions(
        &self,
        workspace: &Path,
        tenant_id: &str,
        organization_id: &str,
        report: &mut MigrationReport,
    ) -> anyhow::Result<()> {
        let sessions_dir = workspace.join("sessions");
        if !sessions_dir.exists() {
            return Ok(());
        }

        let conn = self.store.connection();
        let entries = std::fs::read_dir(&sessions_dir)
            .with_context(|| format!("read {}", sessions_dir.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |ext| ext != "json") {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            let (conversation_id, agent_instance_id) = if let Some(idx) = stem.find("---") {
                let (sid, aid) = stem.split_at(idx);
                (
                    sid.to_string(),
                    Some(aid.trim_start_matches("---").to_string()),
                )
            } else if stem.starts_with("group_") {
                continue;
            } else {
                (stem.to_string(), Some("default".to_string()))
            };

            let snapshot: LegacySessionSnapshot = serde_json::from_str(
                &std::fs::read_to_string(&path)
                    .with_context(|| format!("read {}", path.display()))?,
            )
            .with_context(|| format!("parse {}", path.display()))?;

            let now = chrono::Utc::now().to_rfc3339();
            if let Some(agent_id) = agent_instance_id.as_deref() {
                self.ensure_agent_instance_exists(tenant_id, organization_id, agent_id, &now)?;
            }
            let inserted = conn.execute(
                "INSERT OR IGNORE INTO saas_conversations
                 (id, tenant_id, organization_id, team_id, user_id, agent_instance_id, title, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, 'legacy-user', ?4, NULL, 'active', ?5, ?5)",
                params![
                    conversation_id,
                    tenant_id,
                    organization_id,
                    agent_instance_id,
                    now
                ],
            )?;
            report.conversations_imported += inserted as usize;

            for (index, message) in snapshot.messages.into_iter().enumerate() {
                let inserted = conn.execute(
                    "INSERT OR IGNORE INTO saas_conversation_messages
                     (id, conversation_id, role, content, tool_name, metadata_json, created_at)
                     VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5)",
                    params![
                        format!("{}-{}", conversation_id, index),
                        conversation_id,
                        map_message_role(&message.role),
                        message.content,
                        now
                    ],
                )?;
                report.conversation_messages_imported += inserted as usize;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LegacyDynamicAgent {
    id: String,
    role: String,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    guidance: Option<String>,
    created_at: String,
}

fn map_task_status(status: &LegacyTaskStatus) -> &'static str {
    match status {
        LegacyTaskStatus::Todo => "todo",
        LegacyTaskStatus::InProgress => "in_progress",
        LegacyTaskStatus::Done => "done",
    }
}

fn map_message_role(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

#[derive(Debug, serde::Deserialize)]
struct LegacySessionSnapshot {
    messages: Vec<crate::memory::Message>,
    #[allow(dead_code)]
    max_turns: usize,
}

#[derive(Debug, serde::Deserialize)]
struct LegacyTaskRecord {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: LegacyTaskStatus,
    #[serde(default)]
    coordinator_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum LegacyTaskStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, serde::Deserialize)]
struct LegacyGroupRecord {
    id: String,
    name: Option<String>,
    member_ids: Vec<String>,
    created_at: String,
}

#[derive(Debug, serde::Deserialize)]
struct LegacyGroupSessionSnapshot {
    messages: Vec<LegacyGroupSessionMessage>,
    #[allow(dead_code)]
    max_turns: usize,
}

#[derive(Debug, serde::Deserialize)]
struct LegacyGroupSessionMessage {
    role: String,
    content: String,
    #[serde(default)]
    assistant_id: Option<String>,
}

fn sanitize_group_id(group_id: &str) -> String {
    group_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_import_workspace_seeds_and_imports_files() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        std::fs::create_dir_all(workspace.join("sessions")).unwrap();

        std::fs::write(
            workspace.join("agents.json"),
            r#"[{"id":"agent-1","role":"research","parent_id":"default","guidance":"focus","created_at":"2026-03-21T00:00:00Z"}]"#,
        )
        .unwrap();
        std::fs::write(
            workspace.join("tasks.json"),
            r#"[{"id":"task-1","title":"Task","description":"Desc","status":"todo","coordinator_id":"agent-1","created_at":"2026-03-21T00:00:00Z","updated_at":"2026-03-21T00:00:00Z"}]"#,
        )
        .unwrap();
        std::fs::write(
            workspace.join("sessions").join("session-1---default.json"),
            r#"{"messages":[{"role":"user","content":"hello"},{"role":"assistant","content":"world"}],"max_turns":20}"#,
        )
        .unwrap();

        let store =
            SaasSqliteStore::new(dir.path().join("saas.db")).expect("sqlite store should init");
        let importer = LegacyWorkspaceImporter::new(&store);
        let report = importer
            .import_workspace(&workspace, "tenant-1", "org-1", "ws-1")
            .expect("import should succeed");

        assert_eq!(report.agent_instances_imported, 1);
        assert_eq!(report.groups_imported, 0);
        assert_eq!(report.tasks_imported, 1);
        assert_eq!(report.conversations_imported, 1);
        assert_eq!(report.conversation_messages_imported, 2);

        let conn = store.connection();
        let default_agent_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM saas_agent_instances WHERE id = 'default'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(default_agent_count, 1);
    }

    #[test]
    fn test_import_workspace_creates_placeholder_agent_for_unknown_session_scope() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        std::fs::create_dir_all(workspace.join("sessions")).unwrap();

        std::fs::write(
            workspace.join("sessions").join("session-1---auto.json"),
            r#"{"messages":[{"role":"user","content":"hello"}],"max_turns":20}"#,
        )
        .unwrap();

        let store =
            SaasSqliteStore::new(dir.path().join("saas.db")).expect("sqlite store should init");
        let importer = LegacyWorkspaceImporter::new(&store);
        importer
            .import_workspace(&workspace, "tenant-1", "org-1", "ws-1")
            .expect("import should succeed");

        let conn = store.connection();
        let auto_agent_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM saas_agent_instances WHERE id = 'auto'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(auto_agent_count, 1);
    }
}
