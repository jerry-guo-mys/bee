use std::path::Path;

use serde::{Deserialize, Serialize};

/// 任务状态：看板列
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

/// 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub organization_id: Option<String>,
    #[serde(default)]
    pub team_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: TaskStatus,
    #[serde(default)]
    pub assignee_ids: Vec<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    /// 统筹负责人 agent id，负责拆分任务、创建子 agent、组队、分配职责
    #[serde(default)]
    pub coordinator_id: Option<String>,
    #[serde(default)]
    pub workflow_template_id: Option<String>,
    #[serde(default)]
    pub workflow_run_id: Option<String>,
    #[serde(default)]
    pub internal_group: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assignee_ids: Vec<String>,
    #[serde(default)]
    pub coordinator_id: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub organization_id: Option<String>,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub workflow_template_id: Option<String>,
    #[serde(default)]
    pub workflow_run_id: Option<String>,
    #[serde(default)]
    pub internal_group: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub assignee_ids: Option<Vec<String>>,
    #[serde(default)]
    pub coordinator_id: Option<String>,
    #[serde(default)]
    pub team_id: Option<String>,
}

const TASKS_FILE: &str = "tasks.json";

pub fn load_tasks(workspace: &Path) -> Vec<Task> {
    let path = workspace.join(TASKS_FILE);
    let data = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_tasks(workspace: &Path, tasks: &[Task]) {
    std::fs::create_dir_all(workspace).ok();
    let path = workspace.join(TASKS_FILE);
    if let Ok(json) = serde_json::to_string_pretty(tasks) {
        let _ = std::fs::write(path, json);
    }
}

pub fn build_task(
    req: &CreateTaskRequest,
    assignee_ids: Vec<String>,
    group_id: Option<String>,
    tenant_id: Option<String>,
    organization_id: Option<String>,
    team_id: Option<String>,
    workflow_template_id: Option<String>,
    workflow_run_id: Option<String>,
    internal_group: bool,
) -> Task {
    let now = chrono::Utc::now().to_rfc3339();
    let title = req.title.trim().to_string();
    Task {
        id: uuid::Uuid::new_v4().to_string(),
        tenant_id,
        organization_id,
        team_id,
        title,
        description: normalize_optional_text(req.description.as_deref()),
        status: TaskStatus::Todo,
        assignee_ids,
        group_id,
        coordinator_id: normalize_optional_text(req.coordinator_id.as_deref()),
        workflow_template_id,
        workflow_run_id,
        internal_group,
        created_at: now.clone(),
        updated_at: now,
    }
}

pub fn apply_task_update(task: &mut Task, req: UpdateTaskRequest) {
    if let Some(title) = req.title {
        let title = title.trim();
        if !title.is_empty() {
            task.title = title.to_string();
        }
    }
    if let Some(description) = req.description {
        task.description = normalize_optional_text(Some(description.as_str()));
    }
    if let Some(status) = req.status {
        task.status = status;
    }
    if let Some(assignee_ids) = req.assignee_ids {
        task.assignee_ids = assignee_ids
            .into_iter()
            .filter(|id| !id.trim().is_empty())
            .map(|id| id.trim().to_string())
            .collect();
    }
    if let Some(coordinator_id) = req.coordinator_id {
        task.coordinator_id = normalize_optional_text(Some(coordinator_id.as_str()));
    }
    if let Some(team_id) = req.team_id {
        task.team_id = normalize_optional_text(Some(team_id.as_str()));
    }
    task.updated_at = chrono::Utc::now().to_rfc3339();
}

pub fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
    }
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
