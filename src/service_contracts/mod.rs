//! 服务化共享契约
//!
//! 为后续拆分 `conversation_runtime`、`workflow_task`、`knowledge_memory`
//! 提供统一的租户上下文、请求信封与事件主题定义。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceTenantContext {
    pub tenant_id: String,
    #[serde(default)]
    pub organization_id: Option<String>,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub agent_instance_id: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceRequestEnvelope<T> {
    pub context: ServiceTenantContext,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceDomain {
    ConversationRuntime,
    WorkflowTask,
    KnowledgeMemory,
    OrgCore,
    Iam,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceEventTopic {
    ConversationMessageCreated,
    ConversationToolFailed,
    WorkflowRunStarted,
    WorkflowTaskCreated,
    WorkflowTaskUpdated,
    KnowledgeAccessed,
    ToolPolicyUpdated,
    AuditLogCreated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceEventEnvelope {
    pub topic: ServiceEventTopic,
    pub producer: ServiceDomain,
    pub context: ServiceTenantContext,
    pub resource_type: String,
    pub resource_id: String,
    #[serde(default)]
    pub detail_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentSlice {
    pub service: ServiceDomain,
    pub owns_data: Vec<String>,
    pub reads_from: Vec<ServiceDomain>,
    pub publishes: Vec<ServiceEventTopic>,
    pub subscribes: Vec<ServiceEventTopic>,
}

pub fn default_deployment_slices() -> Vec<DeploymentSlice> {
    vec![
        DeploymentSlice {
            service: ServiceDomain::ConversationRuntime,
            owns_data: vec![
                "sessions".to_string(),
                "conversation_snapshots".to_string(),
                "assistant_runtime_state".to_string(),
            ],
            reads_from: vec![ServiceDomain::Iam, ServiceDomain::OrgCore],
            publishes: vec![
                ServiceEventTopic::ConversationMessageCreated,
                ServiceEventTopic::ConversationToolFailed,
            ],
            subscribes: vec![
                ServiceEventTopic::ToolPolicyUpdated,
                ServiceEventTopic::WorkflowTaskUpdated,
            ],
        },
        DeploymentSlice {
            service: ServiceDomain::WorkflowTask,
            owns_data: vec![
                "workflow_templates".to_string(),
                "workflow_runs".to_string(),
                "tasks".to_string(),
            ],
            reads_from: vec![ServiceDomain::OrgCore, ServiceDomain::Iam],
            publishes: vec![
                ServiceEventTopic::WorkflowRunStarted,
                ServiceEventTopic::WorkflowTaskCreated,
                ServiceEventTopic::WorkflowTaskUpdated,
            ],
            subscribes: vec![ServiceEventTopic::ConversationMessageCreated],
        },
        DeploymentSlice {
            service: ServiceDomain::KnowledgeMemory,
            owns_data: vec![
                "knowledge_bases".to_string(),
                "memory_indexes".to_string(),
                "retrieval_logs".to_string(),
            ],
            reads_from: vec![ServiceDomain::Iam, ServiceDomain::OrgCore],
            publishes: vec![ServiceEventTopic::KnowledgeAccessed],
            subscribes: vec![
                ServiceEventTopic::ConversationMessageCreated,
                ServiceEventTopic::ToolPolicyUpdated,
            ],
        },
    ]
}
