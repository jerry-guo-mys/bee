//! SaaS 主数据模型
//!
//! 这些模型用于支撑多租户、组织、团队、Agent 模板/实例、会话与任务的产品化演进。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub status: TenantStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    #[default]
    Active,
    Suspended,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Organization {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub industry: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Team {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub name: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parent_team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserAccount {
    pub id: String,
    #[serde(default)]
    pub external_user_id: Option<String>,
    pub display_name: String,
    #[serde(default)]
    pub email: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Membership {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    pub user_id: String,
    #[serde(default)]
    pub team_id: Option<String>,
    pub role: MembershipRole,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRole {
    PlatformAdmin,
    OrgAdmin,
    TeamAdmin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentTemplate {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub tool_ids: Vec<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentInstance {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    #[serde(default)]
    pub team_id: Option<String>,
    pub template_id: String,
    pub name: String,
    #[serde(default)]
    pub status: AgentInstanceStatus,
    #[serde(default)]
    pub prompt_override: Option<String>,
    #[serde(default)]
    pub tool_ids_override: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentInstanceStatus {
    #[default]
    Active,
    Disabled,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    #[serde(default)]
    pub team_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub root_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollaborationGroup {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub member_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conversation {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    #[serde(default)]
    pub team_id: Option<String>,
    pub user_id: String,
    #[serde(default)]
    pub agent_instance_id: Option<String>,
    #[serde(default)]
    pub collaboration_group_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: ConversationStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConversationStatus {
    #[default]
    Active,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: String,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assignee_agent_id: Option<String>,
    #[serde(default)]
    pub creator_user_id: Option<String>,
    pub status: TaskRecordStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskRecordStatus {
    Todo,
    InProgress,
    Done,
    Failed,
    Cancelled,
}
