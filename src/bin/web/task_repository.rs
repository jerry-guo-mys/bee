//! 任务持久化：JSON 文件、`saas_tasks`（SQLite）与双写切换。
//!
//! 环境变量（优先 `TASK_PERSISTENCE`，其次 `BEE_TASK_PERSISTENCE`）：
//! - 未设置 / `json`：读写在 `tasks.json`（与历史行为一致）
//! - `sql` / `sqlite`：以 SQLite `saas_tasks` 为准，不读 `tasks.json`
//! - `dual_write` / `dual`：写入 SQL + 全量回写 `tasks.json`；读取来自 SQL
//!
//! **TaskRepository** trait：`WorkspaceTaskRepo` 为默认实现；`patch_task` 仍用模块级函数（闭包更新）。

use std::path::{Path, PathBuf};

use rusqlite::{params, Row};

use bee::saas::SaasSqliteStore;

use super::saas_db_path;
use super::task_service::{load_tasks, save_tasks, Task, TaskStatus};

/// 与 `TASK_PERSISTENCE` / `BEE_TASK_PERSISTENCE` 对齐
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskPersistenceMode {
    Json,
    Sql,
    DualWrite,
}

impl TaskPersistenceMode {
    pub fn from_env() -> Self {
        let raw = std::env::var("TASK_PERSISTENCE")
            .or_else(|_| std::env::var("BEE_TASK_PERSISTENCE"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        match raw.as_str() {
            "sql" | "sqlite" => Self::Sql,
            "dual_write" | "dual" => Self::DualWrite,
            _ => Self::Json,
        }
    }
}

pub fn task_status_to_db(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
    }
}

pub fn task_status_from_db(s: &str) -> Option<TaskStatus> {
    match s {
        "todo" => Some(TaskStatus::Todo),
        "in_progress" => Some(TaskStatus::InProgress),
        "done" => Some(TaskStatus::Done),
        _ => None,
    }
}

/// 工作台任务持久化契约（M3）；列表 / 读 / 写 / 批量追加。
pub trait TaskRepository {
    fn list_filtered(
        &self,
        status: Option<TaskStatus>,
        tenant_id: Option<&str>,
        organization_id: Option<&str>,
        team_id: Option<&str>,
        workflow_run_id: Option<&str>,
    ) -> Result<Vec<Task>, String>;

    fn get_by_id(&self, id: &str) -> Result<Option<Task>, String>;

    fn upsert(&self, task: &Task) -> Result<(), String>;

    fn append(&self, tasks: &[Task]) -> Result<(), String>;
}

/// 绑定 workspace + 持久化模式的默认 `TaskRepository` 实现。
#[derive(Clone, Debug)]
pub struct WorkspaceTaskRepo {
    pub workspace: PathBuf,
    pub mode: TaskPersistenceMode,
}

impl WorkspaceTaskRepo {
    pub fn new(workspace: impl Into<PathBuf>, mode: TaskPersistenceMode) -> Self {
        Self {
            workspace: workspace.into(),
            mode,
        }
    }
}

impl TaskRepository for WorkspaceTaskRepo {
    fn list_filtered(
        &self,
        status: Option<TaskStatus>,
        tenant_id: Option<&str>,
        organization_id: Option<&str>,
        team_id: Option<&str>,
        workflow_run_id: Option<&str>,
    ) -> Result<Vec<Task>, String> {
        list_tasks(
            &self.workspace,
            self.mode,
            status,
            tenant_id,
            organization_id,
            team_id,
            workflow_run_id,
        )
    }

    fn get_by_id(&self, id: &str) -> Result<Option<Task>, String> {
        get_task(&self.workspace, self.mode, id)
    }

    fn upsert(&self, task: &Task) -> Result<(), String> {
        upsert_task(&self.workspace, self.mode, task)
    }

    fn append(&self, tasks: &[Task]) -> Result<(), String> {
        append_tasks(&self.workspace, self.mode, tasks)
    }
}

fn open_store(workspace: &Path) -> Result<SaasSqliteStore, String> {
    SaasSqliteStore::new(saas_db_path(workspace)).map_err(|e| e.to_string())
}

fn tenant_or_default(task: &Task) -> String {
    task.tenant_id
        .clone()
        .unwrap_or_else(|| "tenant-default".to_string())
}

fn org_or_default(task: &Task) -> String {
    task.organization_id
        .clone()
        .unwrap_or_else(|| "org-default".to_string())
}

fn assignee_ids_json(task: &Task) -> Result<String, String> {
    serde_json::to_string(&task.assignee_ids).map_err(|e| e.to_string())
}

fn parse_assignee_ids(json: Option<String>, fallback_agent: Option<String>) -> Vec<String> {
    if let Some(ref s) = json {
        if let Ok(v) = serde_json::from_str::<Vec<String>>(s) {
            if !v.is_empty() {
                return v;
            }
        }
    }
    fallback_agent
        .filter(|a| !a.is_empty())
        .map(|a| vec![a])
        .unwrap_or_default()
}

fn task_from_row(row: &Row) -> rusqlite::Result<Task> {
    let status_s: String = row.get("status")?;
    let status = task_status_from_db(&status_s).unwrap_or(TaskStatus::Todo);
    let assignee_json: Option<String> = row.get("assignee_ids_json")?;
    let assignee_agent: Option<String> = row.get("assignee_agent_id")?;
    let assignee_ids = parse_assignee_ids(assignee_json, assignee_agent);

    let workflow_template_version: Option<i32> = row
        .get::<_, Option<i64>>("workflow_template_version")?
        .and_then(|v| i32::try_from(v).ok());

    let internal_group: i64 = row
        .get::<_, Option<i64>>("internal_group")?
        .unwrap_or(0);

    Ok(Task {
        id: row.get("id")?,
        tenant_id: Some(row.get::<_, String>("tenant_id")?),
        organization_id: Some(row.get::<_, String>("organization_id")?),
        team_id: row.get::<_, Option<String>>("team_id")?,
        title: row.get("title")?,
        description: row.get::<_, Option<String>>("description")?,
        status,
        assignee_ids,
        group_id: row.get::<_, Option<String>>("group_id")?,
        coordinator_id: row.get::<_, Option<String>>("coordinator_id")?,
        workflow_template_id: row.get::<_, Option<String>>("workflow_template_id")?,
        workflow_run_id: row.get::<_, Option<String>>("workflow_run_id")?,
        workflow_template_version,
        internal_group: internal_group != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn list_tasks_sql(workspace: &Path) -> Result<Vec<Task>, String> {
    let store = open_store(workspace)?;
    let conn = store.connection();
    let mut stmt = conn
        .prepare(
            "SELECT id, tenant_id, organization_id, team_id, title, description, status,
                    created_at, updated_at, assignee_agent_id,
                    workflow_run_id, workflow_template_id, workflow_template_version,
                    assignee_ids_json, group_id, coordinator_id, internal_group
             FROM saas_tasks",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], task_from_row)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn upsert_task_sql(workspace: &Path, task: &Task) -> Result<(), String> {
    let store = open_store(workspace)?;
    let conn = store.connection();
    let tenant_id = tenant_or_default(task);
    let org_id = org_or_default(task);
    let status = task_status_to_db(task.status);
    let assignee_json = assignee_ids_json(task)?;
    let internal_i: i64 = if task.internal_group { 1 } else { 0 };

    conn.execute(
        "INSERT INTO saas_tasks (
            id, tenant_id, organization_id, team_id, workspace_id,
            title, description, assignee_agent_id, creator_user_id, status,
            created_at, updated_at,
            workflow_run_id, workflow_template_id, workflow_template_version,
            assignee_ids_json, group_id, coordinator_id, internal_group
        ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, NULL, NULL, ?7, ?8, ?9,
                  ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ON CONFLICT(id) DO UPDATE SET
            tenant_id = excluded.tenant_id,
            organization_id = excluded.organization_id,
            team_id = excluded.team_id,
            title = excluded.title,
            description = excluded.description,
            status = excluded.status,
            created_at = excluded.created_at,
            updated_at = excluded.updated_at,
            workflow_run_id = excluded.workflow_run_id,
            workflow_template_id = excluded.workflow_template_id,
            workflow_template_version = excluded.workflow_template_version,
            assignee_ids_json = excluded.assignee_ids_json,
            group_id = excluded.group_id,
            coordinator_id = excluded.coordinator_id,
            internal_group = excluded.internal_group,
            assignee_agent_id = NULL",
        params![
            task.id,
            tenant_id,
            org_id,
            task.team_id,
            task.title,
            task.description,
            status,
            task.created_at,
            task.updated_at,
            task.workflow_run_id,
            task.workflow_template_id,
            task.workflow_template_version,
            assignee_json,
            task.group_id,
            task.coordinator_id,
            internal_i,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn sync_sql_to_json(workspace: &Path) -> Result<(), String> {
    let tasks = list_tasks_sql(workspace)?;
    save_tasks(workspace, &tasks);
    Ok(())
}

fn filter_tasks(
    mut tasks: Vec<Task>,
    status: Option<TaskStatus>,
    tenant_id: Option<&str>,
    organization_id: Option<&str>,
    team_id: Option<&str>,
    workflow_run_id: Option<&str>,
) -> Vec<Task> {
    tasks.retain(|task| status.is_none_or(|s| task.status == s));
    tasks.retain(|task| {
        tenant_id.is_none_or(|tid| task.tenant_id.as_deref() == Some(tid))
    });
    tasks.retain(|task| {
        organization_id.is_none_or(|oid| task.organization_id.as_deref() == Some(oid))
    });
    tasks.retain(|task| team_id.is_none_or(|tm| task.team_id.as_deref() == Some(tm)));
    tasks.retain(|task| {
        workflow_run_id.is_none_or(|w| task.workflow_run_id.as_deref() == Some(w))
    });
    tasks
}

/// 列表（内存过滤，与原先 `tasks.json` 全量加载语义一致）
pub fn list_tasks(
    workspace: &Path,
    mode: TaskPersistenceMode,
    status: Option<TaskStatus>,
    tenant_id: Option<&str>,
    organization_id: Option<&str>,
    team_id: Option<&str>,
    workflow_run_id: Option<&str>,
) -> Result<Vec<Task>, String> {
    let tasks = match mode {
        TaskPersistenceMode::Json => load_tasks(workspace),
        TaskPersistenceMode::Sql | TaskPersistenceMode::DualWrite => list_tasks_sql(workspace)?,
    };
    Ok(filter_tasks(
        tasks,
        status,
        tenant_id,
        organization_id,
        team_id,
        workflow_run_id,
    ))
}

pub fn get_task(
    workspace: &Path,
    mode: TaskPersistenceMode,
    id: &str,
) -> Result<Option<Task>, String> {
    match mode {
        TaskPersistenceMode::Json => Ok(load_tasks(workspace).into_iter().find(|t| t.id == id)),
        TaskPersistenceMode::Sql | TaskPersistenceMode::DualWrite => {
            let store = open_store(workspace)?;
            let conn = store.connection();
            let mut stmt = conn
                .prepare(
                    "SELECT id, tenant_id, organization_id, team_id, title, description, status,
                            created_at, updated_at, assignee_agent_id,
                            workflow_run_id, workflow_template_id, workflow_template_version,
                            assignee_ids_json, group_id, coordinator_id, internal_group
                     FROM saas_tasks WHERE id = ?1",
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt
                .query_map([id], task_from_row)
                .map_err(|e| e.to_string())?;
            Ok(rows.next().transpose().map_err(|e| e.to_string())?)
        }
    }
}

/// 新建或覆盖一条任务（按 id upsert）
pub fn upsert_task(workspace: &Path, mode: TaskPersistenceMode, task: &Task) -> Result<(), String> {
    match mode {
        TaskPersistenceMode::Json => {
            let mut tasks = load_tasks(workspace);
            if let Some(i) = tasks.iter().position(|t| t.id == task.id) {
                tasks[i] = task.clone();
            } else {
                tasks.push(task.clone());
            }
            save_tasks(workspace, &tasks);
            Ok(())
        }
        TaskPersistenceMode::Sql => upsert_task_sql(workspace, task),
        TaskPersistenceMode::DualWrite => {
            upsert_task_sql(workspace, task)?;
            sync_sql_to_json(workspace)?;
            Ok(())
        }
    }
}

/// 追加多条（工作流批量创建）
pub fn append_tasks(workspace: &Path, mode: TaskPersistenceMode, new_tasks: &[Task]) -> Result<(), String> {
    if new_tasks.is_empty() {
        return Ok(());
    }
    match mode {
        TaskPersistenceMode::Json => {
            let mut tasks = load_tasks(workspace);
            tasks.extend_from_slice(new_tasks);
            save_tasks(workspace, &tasks);
            Ok(())
        }
        TaskPersistenceMode::Sql => {
            for t in new_tasks {
                upsert_task_sql(workspace, t)?;
            }
            Ok(())
        }
        TaskPersistenceMode::DualWrite => {
            for t in new_tasks {
                upsert_task_sql(workspace, t)?;
            }
            sync_sql_to_json(workspace)
        }
    }
}

pub fn patch_task<F>(
    workspace: &Path,
    mode: TaskPersistenceMode,
    task_id: &str,
    f: F,
) -> Result<Option<Task>, String>
where
    F: FnOnce(&mut Task),
{
    match mode {
        TaskPersistenceMode::Json => {
            let mut tasks = load_tasks(workspace);
            let Some(i) = tasks.iter().position(|t| t.id == task_id) else {
                return Ok(None);
            };
            f(&mut tasks[i]);
            let out = tasks[i].clone();
            save_tasks(workspace, &tasks);
            Ok(Some(out))
        }
        TaskPersistenceMode::Sql => {
            let mut task = match get_task(workspace, mode, task_id)? {
                Some(t) => t,
                None => return Ok(None),
            };
            f(&mut task);
            upsert_task_sql(workspace, &task)?;
            Ok(Some(task))
        }
        TaskPersistenceMode::DualWrite => {
            let mut task = match get_task(workspace, TaskPersistenceMode::Sql, task_id)? {
                Some(t) => t,
                None => return Ok(None),
            };
            f(&mut task);
            upsert_task_sql(workspace, &task)?;
            sync_sql_to_json(workspace)?;
            Ok(Some(task))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::TaskRepository;

    #[test]
    fn task_status_db_roundtrip() {
        for s in [TaskStatus::Todo, TaskStatus::InProgress, TaskStatus::Done] {
            let db = task_status_to_db(s);
            assert_eq!(task_status_from_db(db), Some(s));
        }
        assert_eq!(task_status_from_db("unknown"), None);
    }

    #[test]
    fn sql_persist_roundtrip() {
        let dir = std::env::temp_dir().join(format!("bee_task_repo_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join(".bee")).unwrap();
        let store = SaasSqliteStore::new(super::saas_db_path(&dir)).unwrap();
        let c = store.connection();
        c.execute(
            "INSERT OR IGNORE INTO saas_tenants (id, name, status, created_at, updated_at)
             VALUES ('tenant-default', 'Default', 'active', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT OR IGNORE INTO saas_organizations (id, tenant_id, name, created_at, updated_at)
             VALUES ('org-default', 'tenant-default', 'Default', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let mode = TaskPersistenceMode::Sql;
        let task = Task {
            id: "t1".to_string(),
            tenant_id: Some("tenant-default".to_string()),
            organization_id: Some("org-default".to_string()),
            team_id: None,
            title: "hello".to_string(),
            description: Some("d".to_string()),
            status: TaskStatus::Todo,
            assignee_ids: vec!["a1".to_string(), "a2".to_string()],
            group_id: None,
            coordinator_id: Some("coord".to_string()),
            workflow_template_id: Some("tpl".to_string()),
            workflow_run_id: Some("run".to_string()),
            workflow_template_version: Some(2),
            internal_group: true,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        upsert_task(&dir, mode, &task).unwrap();
        let got = get_task(&dir, mode, "t1").unwrap().unwrap();
        assert_eq!(got.title, task.title);
        assert_eq!(got.assignee_ids, task.assignee_ids);
        assert_eq!(got.workflow_template_version, task.workflow_template_version);
        assert_eq!(got.status, TaskStatus::Todo);
    }

    #[test]
    fn workspace_task_repo_implements_trait() {
        let dir = std::env::temp_dir().join(format!("bee_task_repo_trait_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join(".bee")).unwrap();
        let store = SaasSqliteStore::new(super::saas_db_path(&dir)).unwrap();
        let c = store.connection();
        c.execute(
            "INSERT OR IGNORE INTO saas_tenants (id, name, status, created_at, updated_at)
             VALUES ('tenant-default', 'Default', 'active', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT OR IGNORE INTO saas_organizations (id, tenant_id, name, created_at, updated_at)
             VALUES ('org-default', 'tenant-default', 'Default', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let repo = WorkspaceTaskRepo::new(&dir, TaskPersistenceMode::Sql);
        let _: &dyn TaskRepository = &repo;
        let task = Task {
            id: "tr1".to_string(),
            tenant_id: Some("tenant-default".to_string()),
            organization_id: Some("org-default".to_string()),
            team_id: None,
            title: "t".to_string(),
            description: None,
            status: TaskStatus::Todo,
            assignee_ids: vec![],
            group_id: None,
            coordinator_id: None,
            workflow_template_id: None,
            workflow_run_id: Some("run-xyz".to_string()),
            workflow_template_version: None,
            internal_group: false,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        TaskRepository::upsert(&repo, &task).unwrap();
        let listed = repo
            .list_filtered(None, None, None, None, Some("run-xyz"))
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "tr1");
    }

    #[test]
    fn sql_task_survives_db_reopen() {
        let dir = std::env::temp_dir().join(format!("bee_task_repo_reopen_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join(".bee")).unwrap();
        {
            let store = SaasSqliteStore::new(super::saas_db_path(&dir)).unwrap();
            let c = store.connection();
            c.execute(
                "INSERT OR IGNORE INTO saas_tenants (id, name, status, created_at, updated_at)
                 VALUES ('tenant-default', 'Default', 'active', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT OR IGNORE INTO saas_organizations (id, tenant_id, name, created_at, updated_at)
                 VALUES ('org-default', 'tenant-default', 'Default', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }
        let mode = TaskPersistenceMode::Sql;
        let task = Task {
            id: "persist1".to_string(),
            tenant_id: Some("tenant-default".to_string()),
            organization_id: Some("org-default".to_string()),
            team_id: None,
            title: "after reopen".to_string(),
            description: None,
            status: TaskStatus::Todo,
            assignee_ids: vec![],
            group_id: None,
            coordinator_id: None,
            workflow_template_id: None,
            workflow_run_id: None,
            workflow_template_version: None,
            internal_group: false,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        upsert_task(&dir, mode, &task).unwrap();
        let got = get_task(&dir, mode, "persist1").unwrap().unwrap();
        assert_eq!(got.title, "after reopen");
    }
}
