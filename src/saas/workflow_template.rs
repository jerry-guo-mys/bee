//! 工作流模板（租户可配置）领域类型与 definition JSON 解析

use serde::{Deserialize, Serialize};

/// 与详细设计 §3.2 `definition_json` 对齐（v1）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDefinitionJson {
    #[serde(default)]
    pub steps: Vec<WorkflowDefinitionStep>,
    #[serde(default)]
    pub team_filter: Option<WorkflowTeamFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowTeamFilter {
    #[serde(default)]
    pub team_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinitionStep {
    #[serde(default)]
    pub key: Option<String>,
    pub title: String,
    #[serde(default)]
    pub task_kind: Option<String>,
    /// 默认承接的 Agent 模板 ID（bootstrap 后解析为 instance）
    #[serde(default)]
    pub default_agent_template_id: Option<String>,
    #[serde(default)]
    pub instructions: Option<String>,
}

/// 数据库行：模板头
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateRecord {
    pub id: String,
    pub tenant_id: String,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 数据库行：版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplateVersionRecord {
    pub id: String,
    pub template_id: String,
    pub version: i32,
    pub definition_json: String,
    #[serde(default)]
    pub published_at: Option<String>,
    pub created_at: String,
}

/// 已解析、可用来生成任务的模板（一次 run）
#[derive(Debug, Clone)]
pub struct ResolvedWorkflowTemplate {
    /// 对外 template_id（slug 或内置 id）
    pub template_key: String,
    pub version: i32,
    pub steps: Vec<ResolvedWorkflowStep>,
}

#[derive(Debug, Clone)]
pub struct ResolvedWorkflowStep {
    pub title: String,
    pub default_agent_template_id: Option<String>,
}

impl WorkflowDefinitionJson {
    pub fn parse(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn to_resolved_steps(&self) -> Vec<ResolvedWorkflowStep> {
        self.steps
            .iter()
            .map(|s| ResolvedWorkflowStep {
                title: s.title.clone(),
                default_agent_template_id: s.default_agent_template_id.clone(),
            })
            .collect()
    }
}
