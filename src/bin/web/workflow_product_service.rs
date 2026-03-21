use super::task_service::{build_task, CreateTaskRequest, Task, TaskStatus};

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowTemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub team_hint: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowStartRequest {
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub team_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub template_id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowRunResult {
    pub workflow_run_id: String,
    pub workflow_template_id: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskBoardColumn {
    pub status: String,
    pub tasks: Vec<Task>,
}

pub fn list_workflow_templates() -> Vec<WorkflowTemplateSummary> {
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
        },
    ]
}

pub fn start_workflow_run(req: &WorkflowStartRequest) -> Result<WorkflowRunResult, String> {
    let template = list_workflow_templates()
        .into_iter()
        .find(|template| template.id == req.template_id)
        .ok_or_else(|| "workflow template not found".to_string())?;
    let workflow_run_id = format!("wfrun_{}", uuid::Uuid::new_v4().simple());
    let mut tasks = Vec::new();

    for (index, step) in template.steps.iter().enumerate() {
        let mut task = build_task(
            &CreateTaskRequest {
                title: format!("{} / {}", req.title, step),
                description: Some(format!(
                    "{}\n\n工作流步骤 {}: {}",
                    req.description.clone().unwrap_or_default(),
                    index + 1,
                    step
                )),
                assignee_ids: Vec::new(),
                coordinator_id: None,
                tenant_id: req.tenant_id.clone(),
                organization_id: req.organization_id.clone(),
                team_id: req.team_id.clone(),
                workflow_template_id: Some(template.id.clone()),
                workflow_run_id: Some(workflow_run_id.clone()),
                internal_group: false,
            },
            Vec::new(),
            None,
            req.tenant_id.clone(),
            req.organization_id.clone(),
            req.team_id.clone(),
            Some(template.id.clone()),
            Some(workflow_run_id.clone()),
            false,
        );
        if index == 0 {
            task.status = TaskStatus::InProgress;
        }
        tasks.push(task);
    }

    Ok(WorkflowRunResult {
        workflow_run_id,
        workflow_template_id: template.id,
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
