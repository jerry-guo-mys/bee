use std::collections::BTreeMap;
use std::path::Path;

use bee::saas::{ResolvedWorkflowTemplate, SaasSqliteStore, WorkflowDefinitionJson};

use super::task_service::{build_task, CreateTaskRequest, Task, TaskStatus};

/// 与前端 `WorkflowTemplateSummary` 对齐；`id` 对外为 slug（内置模板 id 即 slug）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowTemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub team_hint: String,
    pub steps: Vec<String>,
    /// 0 表示内置；租户已发布版本为真实版本号
    #[serde(default)]
    pub version: i32,
    #[serde(default)]
    pub source: String,
}

/// 启动流程时的请求上下文（`template_id` / `template_version` 在路由层已用于解析，此处保留供扩展与调试）
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WorkflowStartRequest {
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub team_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub template_id: String,
    pub template_version: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowRunResult {
    pub workflow_run_id: String,
    pub workflow_template_id: String,
    #[serde(default)]
    pub workflow_template_version: i32,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskBoardColumn {
    pub status: String,
    pub tasks: Vec<Task>,
}

/// 内置模板（只读）。与租户库中同 `slug` 并存时，**租户已发布版本覆盖内置**（见 `merged_workflow_templates_for_tenant`）。
pub fn builtin_workflow_templates() -> Vec<WorkflowTemplateSummary> {
    vec![
        WorkflowTemplateSummary {
            id: "sales_followup".to_string(),
            name: "销售线索跟进".to_string(),
            description: "从线索分级到首轮跟进再到商机推进。".to_string(),
            team_hint: "sales".to_string(),
            steps: vec![
                "线索分级".to_string(),
                "首轮联系".to_string(),
                "商机推进".to_string(),
            ],
            version: 0,
            source: "builtin".to_string(),
        },
        WorkflowTemplateSummary {
            id: "support_ticket".to_string(),
            name: "客服工单处理".to_string(),
            description: "收单、排查、回复和回访的标准流程。".to_string(),
            team_hint: "service".to_string(),
            steps: vec![
                "问题分流".to_string(),
                "解决方案整理".to_string(),
                "用户回访".to_string(),
            ],
            version: 0,
            source: "builtin".to_string(),
        },
        WorkflowTemplateSummary {
            id: "recruiting_pipeline".to_string(),
            name: "招聘推进".to_string(),
            description: "从 JD 校对到简历筛选、面试安排与反馈汇总。".to_string(),
            team_hint: "hr".to_string(),
            steps: vec![
                "JD 校对".to_string(),
                "简历筛选".to_string(),
                "面试安排".to_string(),
                "反馈汇总".to_string(),
            ],
            version: 0,
            source: "builtin".to_string(),
        },
        WorkflowTemplateSummary {
            id: "content_production".to_string(),
            name: "内容生产".to_string(),
            description: "选题、初稿、审校和发布排期。".to_string(),
            team_hint: "marketing".to_string(),
            steps: vec![
                "选题策划".to_string(),
                "初稿产出".to_string(),
                "审校修改".to_string(),
                "发布排期".to_string(),
            ],
            version: 0,
            source: "builtin".to_string(),
        },
    ]
}

/// 兼容旧调用：等同内置列表（测试或脚本）
#[allow(dead_code)]
pub fn list_workflow_templates() -> Vec<WorkflowTemplateSummary> {
    builtin_workflow_templates()
}

pub fn saas_db_path(workspace: &Path) -> std::path::PathBuf {
    workspace.join(".bee").join("saas.db")
}

/// 合并内置 + 租户已发布模板（同 slug 时租户覆盖内置）
pub fn merged_workflow_templates_for_tenant(
    tenant_id: &str,
    workspace: &Path,
) -> Vec<WorkflowTemplateSummary> {
    let mut map: BTreeMap<String, WorkflowTemplateSummary> = builtin_workflow_templates()
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect();
    let Ok(store) = SaasSqliteStore::new(saas_db_path(workspace)) else {
        return map.into_values().collect();
    };
    let Ok(rows) = store.list_published_workflow_templates(tenant_id) else {
        return map.into_values().collect();
    };
    for (rec, ver, def_json) in rows {
        let Ok(def) = WorkflowDefinitionJson::parse(&def_json) else {
            continue;
        };
        let steps: Vec<String> = def.steps.iter().map(|s| s.title.clone()).collect();
        if steps.is_empty() {
            continue;
        }
        let team_hint = def
            .team_filter
            .as_ref()
            .and_then(|t| t.team_code.clone())
            .unwrap_or_default();
        map.insert(
            rec.slug.clone(),
            WorkflowTemplateSummary {
                id: rec.slug.clone(),
                name: rec.name,
                description: rec.description.unwrap_or_default(),
                team_hint,
                steps,
                version: ver,
                source: "tenant".to_string(),
            },
        );
    }
    map.into_values().collect()
}

fn resolve_builtin_workflow_template(template_key: &str) -> Option<ResolvedWorkflowTemplate> {
    let t = builtin_workflow_templates()
        .into_iter()
        .find(|x| x.id == template_key)?;
    Some(ResolvedWorkflowTemplate {
        template_key: t.id.clone(),
        version: 0,
        steps: t
            .steps
            .iter()
            .map(|title| bee::saas::ResolvedWorkflowStep {
                title: title.clone(),
                default_agent_template_id: None,
            })
            .collect(),
    })
}

/// 解析用于启动 run：先查租户已发布，再回退内置
pub fn resolve_workflow_template_for_start(
    workspace: &Path,
    tenant_id: &str,
    template_key: &str,
    template_version: Option<i32>,
) -> Result<ResolvedWorkflowTemplate, String> {
    let key = template_key.trim();
    if key.is_empty() {
        return Err("template_id is required".to_string());
    }
    if let Ok(store) = SaasSqliteStore::new(saas_db_path(workspace)) {
        match store.resolve_published_workflow_for_start(tenant_id, key, template_version) {
            Ok(Some((_uuid, ver, def))) => {
                if def.steps.is_empty() {
                    return Err("workflow definition has no steps".to_string());
                }
                return Ok(ResolvedWorkflowTemplate {
                    template_key: key.to_string(),
                    version: ver,
                    steps: def.to_resolved_steps(),
                });
            }
            Ok(None) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    resolve_builtin_workflow_template(key)
        .ok_or_else(|| format!("workflow template not found: {key}"))
}

pub fn start_workflow_run(
    req: &WorkflowStartRequest,
    resolved: &ResolvedWorkflowTemplate,
    store: Option<&SaasSqliteStore>,
) -> Result<WorkflowRunResult, String> {
    let workflow_run_id = format!("wfrun_{}", uuid::Uuid::new_v4().simple());
    let mut tasks = Vec::new();

    for (index, step) in resolved.steps.iter().enumerate() {
        let step_label = step.title.clone();
        let mut assignee_ids: Vec<String> = Vec::new();
        if let (Some(s), Some(team_id), Some(tpl_id)) = (
            store,
            req.team_id.as_deref(),
            step.default_agent_template_id.as_deref(),
        ) {
            if let Ok(Some(instance_id)) = s.find_agent_instance_for_team_template(team_id, tpl_id)
            {
                assignee_ids.push(instance_id);
            }
        }
        let mut task = build_task(
            &CreateTaskRequest {
                title: format!("{} / {}", req.title, step_label),
                description: Some(format!(
                    "{}\n\n流程步骤 {}: {}",
                    req.description.clone().unwrap_or_default(),
                    index + 1,
                    step_label
                )),
                assignee_ids: Vec::new(),
                coordinator_id: None,
                tenant_id: req.tenant_id.clone(),
                organization_id: req.organization_id.clone(),
                team_id: req.team_id.clone(),
                workflow_template_id: Some(resolved.template_key.clone()),
                workflow_run_id: Some(workflow_run_id.clone()),
                workflow_template_version: Some(resolved.version),
                internal_group: false,
                project_id: None,
                task_kind: None,
                artifacts: None,
                execution: None,
                review_report: None,
            },
            assignee_ids,
            None,
            req.tenant_id.clone(),
            req.organization_id.clone(),
            req.team_id.clone(),
            Some(resolved.template_key.clone()),
            Some(workflow_run_id.clone()),
            false,
            None,
            None,
        );
        if index == 0 {
            task.status = TaskStatus::InProgress;
        }
        tasks.push(task);
    }

    Ok(WorkflowRunResult {
        workflow_run_id,
        workflow_template_id: resolved.template_key.clone(),
        workflow_template_version: resolved.version,
        tasks,
    })
}

pub fn build_task_board(
    tasks: &[Task],
    tenant_id: Option<&str>,
    organization_id: Option<&str>,
    team_id: Option<&str>,
) -> Vec<TaskBoardColumn> {
    let filtered: Vec<Task> = tasks
        .iter()
        .filter(|task| match tenant_id {
            Some(value) => task.tenant_id.as_deref() == Some(value),
            None => true,
        })
        .filter(|task| match organization_id {
            Some(value) => task.organization_id.as_deref() == Some(value),
            None => true,
        })
        .filter(|task| match team_id {
            Some(value) => task.team_id.as_deref() == Some(value),
            None => true,
        })
        .cloned()
        .collect();

    vec![
        TaskBoardColumn {
            status: "todo".to_string(),
            tasks: filtered
                .iter()
                .filter(|task| matches!(task.status, TaskStatus::Todo))
                .cloned()
                .collect(),
        },
        TaskBoardColumn {
            status: "in_progress".to_string(),
            tasks: filtered
                .iter()
                .filter(|task| matches!(task.status, TaskStatus::InProgress))
                .cloned()
                .collect(),
        },
        TaskBoardColumn {
            status: "done".to_string(),
            tasks: filtered
                .iter()
                .filter(|task| matches!(task.status, TaskStatus::Done))
                .cloned()
                .collect(),
        },
    ]
}
