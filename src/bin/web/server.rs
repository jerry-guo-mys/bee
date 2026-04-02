//! 共享 HTTP 实现：bee-web（对话 + 静态）与 bee-admin（管理 API）
//!
//! bee-web: `cargo run --bin bee-web --features web`
//! bee-admin: `cargo run --bin bee-admin --features web`

#[path = "handlers/admin.rs"]
pub(crate) mod admin_handlers;

#[path = "assistant_catalog.rs"]
mod assistant_catalog;
#[path = "dynamic_agent_catalog.rs"]
mod dynamic_agent_catalog;
#[path = "inbox_service.rs"]
mod inbox_service;
#[path = "session_store.rs"]
mod session_store;
#[path = "task_coordinator_service.rs"]
mod task_coordinator_service;
#[path = "task_repository.rs"]
mod task_repository;
#[path = "task_service.rs"]
mod task_service;
#[path = "workflow_product_service.rs"]
mod workflow_product_service;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        Html, Response,
    },
    routing::{get, post},
    Json, Router,
};
use bee::memory::{Message, Role};
use bytes::Bytes;
use futures_util::stream::{self, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use bee::agent::{
    consolidate_memory_with_llm, create_agent_components,
    create_context_with_long_term_for_assistant, create_vector_long_term_for_assistant,
    process_message, process_message_stream, process_message_stream_with_cancel,
};
use bee::config::{load_config, AppConfig};
use bee::core::AgentComponents;
use bee::memory::InMemoryVectorLongTerm;
use bee::memory::{
    append_heartbeat_log, consolidate_memory, memory_root, record_error as learnings_record_error,
    record_learning as learnings_record_learning,
};
use bee::react::{compact_context, ContextManager, Planner, ReactEvent};
use bee::saas::{
    append_audit_log, audit_detail_json, bootstrap_workspace_saas, build_bootstrap_plan,
    default_low_risk_tools, ensure_access, instantiate_team_templates, list_audit_logs,
    list_team_templates, list_tool_policies, persist_bootstrap_plan,
    resolve_effective_tool_allowlist, upsert_tool_policy, AccessContext, AccessRequirement,
    AuditActor, AuditLogInput, IndustryTemplate, OrganizationBootstrapRequest, SaasSqliteStore,
    SaasTemplateRepository, TeamTemplateInstantiationRequest, ToolPolicyInput, ToolPolicyScope,
    WorkflowDefinitionJson, WorkflowTemplateRecord,
};
use bee::skills::{Skill, SkillLoader};
use bee::tool_policy::refine_allowed_tools_for_input;
use bee::tools::{tool_call_schema_json, CreateTool, DynamicAgent};

use assistant_catalog::{
    build_prompt_with_skills, load_assistants, load_knowledge_overrides, load_skills_overrides,
    platform_template_id, save_knowledge_overrides, save_skills_overrides, AssistantEntry,
    AssistantInfo,
};
use session_store::{
    group_messages_to_llm_messages, load_group_session, load_groups_from_disk,
    load_session_from_disk, save_group_session, save_groups_to_disk, save_session_to_disk,
    session_key, session_path, GroupChatMessage, GroupInfo, SessionSnapshot, WebSessionScope,
};
use task_repository::{TaskPersistenceMode, TaskRepository};
use task_service::{
    apply_task_update, build_task, status_label, CreateTaskRequest, Task, TaskStatus,
    UpdateTaskRequest,
};
use workflow_product_service::{
    build_task_board, merged_workflow_templates_for_tenant, resolve_workflow_template_for_start,
    start_workflow_run, TaskBoardColumn,
    WorkflowRunResult as ProductWorkflowRunResult, WorkflowStartRequest, WorkflowTemplateSummary,
};

const DEFAULT_MAX_TURNS: usize = 20;

/// 拓扑事件（Phase 4）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkspaceEvent {
    GroupCreated {
        id: String,
        name: Option<String>,
        member_ids: Vec<String>,
    },
    MessageCreated {
        group_id: String,
        from: Option<String>,
        to: Option<String>,
        content_preview: String,
    },
    AgentCreated {
        id: String,
        role: String,
        parent_id: Option<String>,
    },
    TaskCreated {
        id: String,
        title: String,
    },
    TaskUpdated {
        id: String,
        status: String,
    },
}

struct CreateObservationParsed {
    id: String,
    role: String,
    parent_id: Option<String>,
}

/// 从 create 工具 Observation preview 解析 id、role
fn parse_create_observation(preview: &str) -> Option<CreateObservationParsed> {
    let re = regex::Regex::new(r"id=([a-zA-Z0-9_-]+),\s*role=([^.]+)").ok()?;
    let cap = re.captures(preview)?;
    let id = cap.get(1)?.as_str().to_string();
    let role = cap.get(2)?.as_str().trim().to_string();
    Some(CreateObservationParsed {
        id,
        role,
        parent_id: None,
    })
}

fn emit_event(bus: &broadcast::Sender<String>, ev: WorkspaceEvent) {
    if let Ok(json) = serde_json::to_string(&ev) {
        let _ = bus.send(json);
    }
}

/// 心跳时发给 Agent 的提示：根据长期记忆与当前状态检查待办或需跟进事项
const HEARTBEAT_PROMPT: &str = "Heartbeat: 你正在后台自主运行。请根据长期记忆与当前状态，检查是否有待办或需跟进的事项；若有则输出一条简短建议，若无则仅回复 OK。可使用 cat/ls 查看 workspace 下 memory 或任务文件。";

pub(crate) struct AppState {
    /// 应用配置（解决问题 1.2）
    config: AppConfig,
    /// 可运行时替换，以支持「多 LLM 后端切换」与配置热更新（白皮书 Phase 5）
    components: Arc<RwLock<Arc<AgentComponents>>>,
    sessions: Arc<RwLock<HashMap<String, ContextManager>>>,
    sessions_dir: PathBuf,
    /// 记忆根目录（workspace/memory），用于短期日志与长期 Markdown
    memory_root: PathBuf,
    workspace: PathBuf,
    /// 每个助手的向量长期记忆（assistant_id -> Arc），启用时按需创建
    shared_vector_by_assistant: Arc<RwLock<HashMap<String, Arc<InMemoryVectorLongTerm>>>>,
    /// 多助手：列表与 id -> 完整 system prompt（含 tool schema）
    assistants: Vec<AssistantInfo>,
    assistant_prompts: Arc<RwLock<HashMap<String, String>>>,
    /// 每个智能体可用的技能（工具名列表），空表示全部可用
    assistant_skills: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 工具列表（id, name, description），用于技能配置
    tool_descriptions: Vec<(String, String)>,
    /// 助手元数据（prompt 路径等），用于重建 prompt
    assistant_entries: HashMap<String, AssistantEntry>,
    config_base: PathBuf,
    /// 可切换模型：列表与 id -> 模型配置
    models: Vec<ModelInfo>,
    model_configs: HashMap<String, ModelEntry>,
    /// 技能加载器
    skill_loader: Arc<SkillLoader>,
    /// 群组：id -> GroupInfo
    groups: Arc<RwLock<HashMap<String, GroupInfo>>>,
    /// 群组持久化路径
    groups_path: PathBuf,
    /// 拓扑事件广播（SSE /api/events）
    event_bus: broadcast::Sender<String>,
    /// 正在运行的流式会话取消令牌（session_key -> token）
    active_cancellations: Arc<RwLock<HashMap<String, CancellationToken>>>,
    /// 任务持久化：`TASK_PERSISTENCE` / `BEE_TASK_PERSISTENCE`（见 task_repository）
    task_persistence: task_repository::TaskPersistenceMode,
}

impl AppState {
    pub(crate) fn task_repo(&self) -> task_repository::WorkspaceTaskRepo {
        task_repository::WorkspaceTaskRepo::new(self.workspace.clone(), self.task_persistence)
    }
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    session_id: Option<String>,
    /// 多助手：选用的助手 id，缺省为 "default"
    #[serde(default)]
    assistant_id: Option<String>,
    /// 群聊：group_id 与 assistant_id 互斥，有 group_id 时为群聊模式
    #[serde(default)]
    group_id: Option<String>,
    /// 可切换模型：选用的模型 id，缺省为 "default"（使用配置）
    #[serde(default)]
    model_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    reply: String,
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateGroupRequest {
    name: Option<String>,
    member_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreateAgentRequest {
    role: String,
    #[serde(default)]
    guidance: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InboxProcessRequest {
    assistant_id: String,
}

#[derive(Debug, Deserialize)]
struct BootstrapOrganizationRequest {
    organization_name: String,
    #[serde(default)]
    industry: Option<String>,
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    workspace_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Serialize)]
struct BootstrapOrganizationResponse {
    tenant_id: String,
    organization_id: String,
    workspace_id: String,
    industry: String,
    team_count: usize,
    team_names: Vec<String>,
    agent_template_count: usize,
    agent_instance_count: usize,
}

#[derive(Debug, Deserialize)]
struct AgentTemplatesQuery {
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Serialize)]
struct AgentTemplateSummary {
    id: String,
    tenant_id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_id: Option<String>,
    tool_ids: Vec<String>,
    knowledge_base_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct InstantiateTeamTemplatesRequest {
    organization_id: String,
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    template_ids: Vec<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Serialize)]
struct InstantiateTeamTemplatesResponse {
    tenant_id: String,
    organization_id: String,
    team_id: String,
    created_count: usize,
    existing_count: usize,
    instance_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TaskBoardQuery {
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct WorkflowTemplatesQuery {
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct StartWorkflowRequest {
    template_id: String,
    #[serde(default)]
    template_version: Option<i32>,
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct ToolPoliciesQuery {
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct UpdateToolPolicyRequest {
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(default)]
    allowed_tool_ids: Vec<String>,
    #[serde(default)]
    denied_tool_ids: Vec<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct AuditLogsQuery {
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default = "default_audit_limit")]
    limit: usize,
    #[serde(flatten)]
    scope: WebScopeParams,
}

fn default_audit_limit() -> usize {
    50
}

#[derive(Debug, Deserialize)]
struct HistoryQuery {
    session_id: Option<String>,
    #[serde(default)]
    assistant_id: Option<String>,
    /// 群聊：有 group_id 时按群加载历史，返回消息含 assistant_id
    #[serde(default)]
    group_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Serialize)]
struct HistoryMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    assistant_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct HistoryResponse {
    session_id: String,
    messages: Vec<HistoryMessage>,
}

#[derive(Debug, Deserialize)]
struct ConsolidateQuery {
    #[serde(default)]
    since_days: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ConsolidateResponse {
    dates_processed: Vec<String>,
    blocks_added: usize,
}

#[derive(Debug, Deserialize)]
struct ClearSessionRequest {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    assistant_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct CancelSessionRequest {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    assistant_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Default, Deserialize)]
struct WebScopeParams {
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    organization_id: Option<String>,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(default)]
    agent_instance_id: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
}

impl WebScopeParams {
    fn to_scope(&self, session_id: &str, assistant_id: &str) -> WebSessionScope {
        WebSessionScope {
            tenant_id: self
                .tenant_id
                .clone()
                .or_else(|| Some("tenant-default".to_string())),
            organization_id: self
                .organization_id
                .clone()
                .or_else(|| Some("org-default".to_string())),
            team_id: self.team_id.clone(),
            agent_instance_id: self
                .agent_instance_id
                .clone()
                .or_else(|| Some(assistant_id.to_string())),
            user_id: self
                .user_id
                .clone()
                .or_else(|| Some(session_id.to_string())),
        }
    }

    fn management_tenant_id(&self) -> String {
        self.tenant_id
            .clone()
            .unwrap_or_else(|| "tenant-default".to_string())
    }

    fn management_organization_id(&self) -> Option<String> {
        self.organization_id
            .clone()
            .or_else(|| Some("org-default".to_string()))
    }

    fn management_user_id(&self) -> String {
        self.user_id
            .clone()
            .unwrap_or_else(|| "user-default".to_string())
    }

    fn to_access_context(
        &self,
        tenant_id: String,
        organization_id: Option<String>,
        team_id: Option<String>,
    ) -> AccessContext {
        AccessContext {
            tenant_id,
            organization_id,
            team_id,
            user_id: self.management_user_id(),
        }
    }

    fn to_audit_actor(
        &self,
        tenant_id: String,
        organization_id: Option<String>,
        team_id: Option<String>,
    ) -> AuditActor {
        AuditActor {
            tenant_id,
            organization_id,
            team_id,
            user_id: Some(self.management_user_id()),
        }
    }
}

/// 会话列表项
#[derive(Debug, Serialize)]
struct SessionListItem {
    /// 复合 key：{session_id}::{assistant_id}，用于 API 调用
    id: String,
    /// 会话 id
    session_id: String,
    /// 助手 id，该会话属于该助手的独立记忆
    assistant_id: String,
    title: String,
    message_count: usize,
    updated_at: String,
    /// 日期 YYYY-MM-DD，用于前端分组（今天/昨天/上周/更早）
    date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RenameSessionRequest {
    session_id: String,
    title: String,
}

/// 工具信息：供前端技能配置使用
#[derive(Debug, Clone, Serialize)]
struct ToolInfo {
    id: String,
    name: String,
    description: String,
}

/// 可切换模型：前端展示用
#[derive(Debug, Clone, Serialize)]
struct ModelInfo {
    id: String,
    name: String,
}

/// 技能信息：前端展示用
#[derive(Debug, Clone, Serialize)]
struct SkillInfo {
    id: String,
    name: String,
    description: String,
    tags: Vec<String>,
    capability: String,
    template: Option<String>,
    has_script: bool,
}

impl From<&Skill> for SkillInfo {
    fn from(s: &Skill) -> Self {
        Self {
            id: s.meta.id.clone(),
            name: s.meta.name.clone(),
            description: s.meta.description.clone(),
            tags: s.meta.tags.clone(),
            capability: s.capability.clone(),
            template: s.template.clone(),
            has_script: s.script_path.is_some(),
        }
    }
}

/// models.toml 中单条配置
#[derive(Debug, Clone, Deserialize)]
struct ModelEntry {
    id: String,
    name: String,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    api_key_env: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelsConfig {
    models: Vec<ModelEntry>,
}

/// 自动分派：根据用户提问调用 LLM 选择最合适的助手，返回 assistant_id
async fn dispatch_assistant(state: &AppState, message: &str) -> Result<String, String> {
    let candidates: Vec<&AssistantInfo> =
        state.assistants.iter().filter(|a| a.id != "auto").collect();
    if candidates.is_empty() {
        return Ok("default".to_string());
    }
    let list_text = candidates
        .iter()
        .map(|a| format!("- {} (id={}): {}", a.name, a.id, a.description))
        .collect::<Vec<_>>()
        .join("\n");
    let system = format!(
        "You are a router. Given the user's question and the list of assistants below, choose the most suitable one.\n\
         Reply with ONLY the assistant id (e.g. default, media, student, money, viral). No explanation, no punctuation.\n\n\
         Available assistants:\n{}",
        list_text
    );
    let user_msg = format!("User question:\n{}", message);
    let messages = vec![Message::user(user_msg)];
    let components = state.components.read().await;
    let output = components
        .planner
        .plan_with_system(&messages, &system)
        .await
        .map_err(|e| e.to_string())?;
    let id = output
        .trim()
        .split(|c: char| c.is_whitespace() || c == '.' || c == '。')
        .next()
        .unwrap_or("default")
        .to_lowercase();
    let valid = candidates.iter().any(|a| a.id == id);
    Ok(if valid { id } else { "default".to_string() })
}

/// 从 workspace/agents.json 加载动态创建的 sub-agent（Phase 3）
/// 从 config/models.toml 加载可切换模型
fn load_models(config_base: &std::path::Path) -> (Vec<ModelInfo>, HashMap<String, ModelEntry>) {
    let toml_path = [
        config_base.join("models.toml"),
        std::path::Path::new("config/models.toml").to_path_buf(),
        std::path::Path::new("../config/models.toml").to_path_buf(),
    ]
    .into_iter()
    .find(|p| p.exists());

    let entries: Vec<ModelEntry> = match toml_path.and_then(|p| std::fs::read_to_string(p).ok()) {
        Some(s) => toml::from_str::<ModelsConfig>(&s)
            .map(|c| c.models)
            .unwrap_or_default(),
        None => vec![ModelEntry {
            id: "default".to_string(),
            name: "默认（配置）".to_string(),
            base_url: None,
            model: None,
            api_key_env: None,
        }],
    };

    let list: Vec<ModelInfo> = entries
        .iter()
        .map(|e| ModelInfo {
            id: e.id.clone(),
            name: e.name.clone(),
        })
        .collect();

    let mut configs = HashMap::new();
    for e in entries {
        configs.insert(e.id.clone(), e);
    }
    (list, configs)
}

/// 根据模型配置创建 LlmClient（OpenAI 兼容）
fn create_llm_for_model(entry: &ModelEntry) -> Arc<dyn bee::llm::LlmClient> {
    let base_url = entry.base_url.as_deref();
    let model = entry.model.as_deref().unwrap_or("gpt-4o-mini");
    let api_key = entry
        .api_key_env
        .as_deref()
        .and_then(|k| std::env::var(k).ok())
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());
    Arc::new(bee::llm::OpenAiClient::new(
        base_url,
        model,
        api_key.as_deref(),
    ))
}

pub(crate) fn saas_db_path(workspace: &std::path::Path) -> PathBuf {
    workspace.join(".bee").join("saas.db")
}

fn write_audit_log(
    workspace: &std::path::Path,
    actor: AuditActor,
    action: impl Into<String>,
    resource_type: impl Into<String>,
    resource_id: impl Into<String>,
    detail: serde_json::Value,
) -> Result<(), String> {
    let store = SaasSqliteStore::new(saas_db_path(workspace)).map_err(|err| err.to_string())?;
    append_audit_log(
        &store,
        AuditLogInput {
            actor,
            action: action.into(),
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            detail_json: Some(audit_detail_json(detail).map_err(|err| err.to_string())?),
        },
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn require_management_access(
    workspace: &std::path::Path,
    ctx: &AccessContext,
    requirement: AccessRequirement,
) -> Result<(), (StatusCode, String)> {
    let store = SaasSqliteStore::new(saas_db_path(workspace))
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    ensure_access(&store, ctx, requirement)
        .map(|_| ())
        .map_err(|err| (StatusCode::FORBIDDEN, err))
}

fn assistant_knowledge_bases(state: &AppState, assistant_id: &str) -> Vec<String> {
    state
        .assistant_entries
        .get(assistant_id)
        .and_then(|entry| entry.knowledge_bases.clone())
        .unwrap_or_default()
}

fn audit_knowledge_access(
    state: &AppState,
    assistant_id: &str,
    scope: &WebSessionScope,
    session_id: &str,
) {
    let knowledge_base_ids = assistant_knowledge_bases(state, assistant_id);
    if knowledge_base_ids.is_empty() {
        return;
    }
    let tenant_id = scope
        .tenant_id
        .clone()
        .unwrap_or_else(|| "tenant-default".to_string());
    let organization_id = scope
        .organization_id
        .clone()
        .or_else(|| Some("org-default".to_string()));
    let _ = write_audit_log(
        &state.workspace,
        AuditActor {
            tenant_id: tenant_id.clone(),
            organization_id: organization_id.clone(),
            team_id: scope.team_id.clone(),
            user_id: scope
                .user_id
                .clone()
                .or_else(|| Some(session_id.to_string())),
        },
        "knowledge_base.access",
        "assistant_session",
        session_id.to_string(),
        serde_json::json!({
            "assistant_id": assistant_id,
            "knowledge_base_ids": knowledge_base_ids,
            "agent_instance_id": scope.agent_instance_id,
        }),
    );
}

fn audit_tool_failure(
    state: &AppState,
    scope: &WebSessionScope,
    session_id: &str,
    tool: &str,
    reason: &str,
) {
    let tenant_id = scope
        .tenant_id
        .clone()
        .unwrap_or_else(|| "tenant-default".to_string());
    let organization_id = scope
        .organization_id
        .clone()
        .or_else(|| Some("org-default".to_string()));
    let _ = write_audit_log(
        &state.workspace,
        AuditActor {
            tenant_id,
            organization_id,
            team_id: scope.team_id.clone(),
            user_id: scope
                .user_id
                .clone()
                .or_else(|| Some(session_id.to_string())),
        },
        "tool.failure",
        "tool_execution",
        tool.to_string(),
        serde_json::json!({
            "session_id": session_id,
            "reason": reason,
            "agent_instance_id": scope.agent_instance_id,
        }),
    );
}

fn parse_industry_template(raw: Option<&str>) -> Result<IndustryTemplate, String> {
    let value = raw.unwrap_or("general").trim().to_ascii_lowercase();
    match value.as_str() {
        "" | "general" => Ok(IndustryTemplate::General),
        "sales_service" | "sales-service" | "sales" => Ok(IndustryTemplate::SalesService),
        "marketing_studio" | "marketing-studio" | "marketing" => {
            Ok(IndustryTemplate::MarketingStudio)
        }
        "recruiting_agency" | "recruiting-agency" | "recruiting" | "hr" => {
            Ok(IndustryTemplate::RecruitingAgency)
        }
        "software_delivery" | "software-delivery" | "software" | "engineering" => {
            Ok(IndustryTemplate::SoftwareDelivery)
        }
        _ => Err(format!("unsupported industry template: {}", value)),
    }
}

fn industry_template_code(template: IndustryTemplate) -> &'static str {
    match template {
        IndustryTemplate::General => "general",
        IndustryTemplate::SalesService => "sales_service",
        IndustryTemplate::MarketingStudio => "marketing_studio",
        IndustryTemplate::RecruitingAgency => "recruiting_agency",
        IndustryTemplate::SoftwareDelivery => "software_delivery",
    }
}

async fn resolve_allowed_tools_for_scope(
    state: &AppState,
    assistant_id: &str,
    scope: &WebSessionScope,
) -> Vec<String> {
    let base_tools = state
        .assistant_skills
        .read()
        .await
        .get(assistant_id)
        .cloned()
        .unwrap_or_else(|| {
            state
                .tool_descriptions
                .iter()
                .map(|(name, _)| name.clone())
                .collect()
        });

    let tenant_id = scope
        .tenant_id
        .clone()
        .unwrap_or_else(|| "tenant-default".to_string());
    let organization_id = scope
        .organization_id
        .clone()
        .or_else(|| Some("org-default".to_string()));
    let default_tools = if scope
        .team_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        base_tools
    } else {
        default_low_risk_tools(&base_tools)
    };

    if let Ok(store) = SaasSqliteStore::new(saas_db_path(&state.workspace)) {
        if let Ok(resolved) = resolve_effective_tool_allowlist(
            &store,
            &ToolPolicyScope {
                tenant_id,
                organization_id,
                team_id: scope.team_id.clone(),
            },
            &default_tools,
        ) {
            return resolved;
        }
    }
    default_tools
}

fn init_tracing_subscriber() {
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with(fmt::layer())
        .try_init();
}

/// 构建与 bee-web / bee-admin 共享的 `AppState`
async fn build_app_state() -> anyhow::Result<Arc<AppState>> {
    let cfg = load_config(None).unwrap_or_default();
    let workspace = cfg
        .app
        .workspace_root
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap().join("workspace"));
    let workspace = workspace.canonicalize().unwrap_or(workspace);
    std::fs::create_dir_all(&workspace).ok();
    match bootstrap_workspace_saas(&workspace) {
        Ok(result) => {
            tracing::info!(
                "SaaS bootstrap initialized at {} (agents={}, groups={}, conversations={}, tasks={})",
                result.db_path.display(),
                result.report.agent_instances_imported,
                result.report.groups_imported,
                result.report.conversations_imported,
                result.report.tasks_imported
            );
        }
        Err(err) => {
            tracing::warn!("SaaS bootstrap failed: {}", err);
        }
    }

    let config_base = std::path::Path::new("config");
    let system_prompt = [
        config_base.join("prompts/system.md"),
        std::path::Path::new("../config/prompts/system.md").to_path_buf(),
    ]
    .into_iter()
    .find_map(|p| std::fs::read_to_string(&p).ok())
    .unwrap_or_else(|| {
        "You are Bee, a helpful AI assistant. Use tools: cat, ls, echo, shell, search.".to_string()
    });

    let (models, model_configs) = load_models(config_base);

    let config_base = config_base.to_path_buf();
    let components_inner = create_agent_components(&cfg, &workspace);
    let tool_descriptions = components_inner.executor.tool_descriptions();
    let skill_loader = components_inner.skill_loader.clone();
    let saas_db = saas_db_path(&workspace);
    let (mut assistants, mut prompts_map, mut skills_map, assistant_entries) = load_assistants(
        &config_base,
        &tool_descriptions,
        Some(&saas_db),
        "tenant-default",
    );

    let dynamic = dynamic_agent_catalog::load_dynamic_agents(&workspace);
    let all_tool_list: String = tool_descriptions
        .iter()
        .map(|(n, d)| format!("- {}: {}", n, d))
        .collect::<Vec<_>>()
        .join("\n");
    let tool_schema = tool_call_schema_json();
    for da in &dynamic {
        if !prompts_map.contains_key(&da.id) {
            let prompt =
                dynamic_agent_catalog::build_dynamic_agent_prompt(da, &all_tool_list, &tool_schema);
            prompts_map.insert(da.id.clone(), prompt);
        }
        if assistants.iter().all(|a| a.id != da.id) {
            assistants.push(AssistantInfo {
                id: da.id.clone(),
                name: da.role.clone(),
                description: da.guidance.clone().unwrap_or_else(|| da.role.clone()),
                skills: Some(tool_descriptions.iter().map(|(n, _)| n.clone()).collect()),
            });
        }
        if !skills_map.contains_key(&da.id) {
            skills_map.insert(
                da.id.clone(),
                tool_descriptions.iter().map(|(n, _)| n.clone()).collect(),
            );
        }
    }

    if !prompts_map.contains_key("default") {
        let fallback = assistants
            .iter()
            .find(|a| a.id != "auto")
            .and_then(|a| prompts_map.get(&a.id).cloned())
            .unwrap_or_else(|| system_prompt.clone());
        prompts_map.insert("default".to_string(), fallback);
    }
    let assistant_prompts = Arc::new(RwLock::new(prompts_map));
    let assistant_skills = Arc::new(RwLock::new(skills_map));
    let components = Arc::new(RwLock::new(Arc::new(components_inner)));
    assistants.insert(
        0,
        AssistantInfo {
            id: "auto".to_string(),
            name: "自动分派助手".to_string(),
            description: "根据提问自动选择最合适的助手".to_string(),
            skills: None,
        },
    );

    let sessions_dir = workspace.join("sessions");
    let memory_root = memory_root(&workspace);
    std::fs::create_dir_all(&sessions_dir).ok();
    std::fs::create_dir_all(&memory_root).ok();

    let shared_vector_by_assistant = Arc::new(RwLock::new(HashMap::new()));

    let groups_path = workspace.join("groups.json");
    let groups = load_groups_from_disk(&groups_path);
    let (event_bus, _) = broadcast::channel::<String>(64);
    let task_persistence = TaskPersistenceMode::from_env();
    tracing::info!(?task_persistence, "task persistence mode");

    let state = Arc::new(AppState {
        config: cfg.clone(),
        components,
        sessions: Arc::new(RwLock::new(HashMap::new())),
        sessions_dir,
        memory_root: memory_root.clone(),
        workspace: workspace.clone(),
        shared_vector_by_assistant,
        assistants,
        assistant_prompts,
        assistant_skills,
        tool_descriptions,
        assistant_entries,
        config_base,
        models,
        model_configs,
        skill_loader,
        groups,
        groups_path,
        event_bus,
        active_cancellations: Arc::new(RwLock::new(HashMap::new())),
        task_persistence,
    });

    Ok(state)
}

/// 管理类 REST（供 web-ui / 运维）；不含对话与静态页
fn router_admin_api(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        // 原有的管理 API
        .route("/api/assistants", get(api_assistants_list))
        .route("/api/agent-templates", get(api_agent_templates_list))
        .route("/api/agents", get(api_agents_list).post(api_agents_create))
        .route("/api/groups", get(api_groups_list).post(api_groups_create))
        .route(
            "/api/organizations/bootstrap",
            post(api_organizations_bootstrap),
        )
        .route("/api/task-board", get(api_task_board))
        .route("/api/workflow-templates", get(api_workflow_templates_list))
        .route("/api/workflows/start", post(api_workflows_start))
        .route(
            "/api/admin/workflow-templates",
            get(api_admin_workflow_templates_list).post(api_admin_workflow_templates_create),
        )
        .route(
            "/api/admin/workflow-templates/:id/versions",
            post(api_admin_workflow_template_add_version),
        )
        .route(
            "/api/admin/workflow-templates/:id/publish",
            post(api_admin_workflow_template_publish),
        )
        .route(
            "/api/teams/:team_id/agent-instances/bootstrap",
            post(api_team_agent_instances_bootstrap),
        )
        .route("/api/tasks", get(api_tasks_list).post(api_tasks_create))
        .route("/api/tasks/:id", axum::routing::patch(api_tasks_update))
        .route("/api/tasks/:id/start", post(api_tasks_start))
        .route("/api/tools", get(api_tools_list))
        .route(
            "/api/tool-policies",
            get(api_tool_policies_list).put(api_tool_policies_put),
        )
        .route("/api/audit-logs", get(api_audit_logs_list))
        .route(
            "/api/assistant/:id/skills",
            axum::routing::put(api_assistant_skills_put),
        )
        .route(
            "/api/assistant/:id/knowledge-bases",
            axum::routing::put(api_assistant_knowledge_bases_put),
        )
        .route("/api/models", get(api_models_list))
        .route("/api/skills", get(api_skills_list))
        .route("/api/skills/:id", get(api_skill_get))
        .route("/api/skills/:id", axum::routing::put(api_skill_update))
        .route(
            "/api/skills/import-openclaw",
            post(api_skill_import_openclaw),
        )
        // 新增管理 API (租户/组织/团队/成员管理)
        .merge(admin_handlers::create_router(&state.workspace))
        // 观测性 API
        .route("/api/health", get(|| async { "OK" }))
        .route("/api/metrics", get(api_metrics))
        .route("/api/metrics/prometheus", get(api_metrics_prometheus))
        .route("/api/events", get(api_events_sse))
        .route("/api/traces/recent", get(api_traces_recent))
        .route("/api/traces/:request_id", get(api_traces_get))
        // 记忆管理 API
        .route("/api/memory/consolidate", post(api_memory_consolidate))
        .route(
            "/api/memory/consolidate-llm",
            post(api_memory_consolidate_llm),
        )
        .route("/api/config/reload", post(api_config_reload))
        .with_state(state)
}

/// 对话、收件箱与静态资源（仅 bee-web）
fn router_chat_and_static() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index))
        .route("/metrics", get(serve_metrics_dashboard))
        .route("/traces", get(serve_traces_page))
        .route("/traces.html", get(serve_traces_page))
        .route("/js/marked.min.js", get(serve_marked_js))
        .route("/js/highlight.min.js", get(serve_highlight_js))
        .route("/css/github-dark.min.css", get(serve_highlight_css))
        .route("/api/chat", post(api_chat))
        .route("/api/chat/stream", post(api_chat_stream))
        .route("/api/history", get(api_history))
        .route("/api/sessions", get(api_sessions_list))
        .route("/api/session/clear", post(api_session_clear))
        .route("/api/session/cancel", post(api_session_cancel))
        .route("/api/compact", post(api_compact))
        .route("/api/session/rename", post(api_session_rename))
        .route("/api/inbox/process", post(api_inbox_process))
        .route("/swarm", get(serve_swarm_page))
        .route("/tasks", get(serve_tasks_page))
}

fn spawn_background_tasks(state: &Arc<AppState>, cfg: &AppConfig) {
    // 定期整理记忆：每 24 小时将近期短期日志归纳写入长期记忆
    let memory_root_periodic = state.memory_root.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Ok(r) = consolidate_memory(&memory_root_periodic, 7) {
                if !r.dates_processed.is_empty() {
                    tracing::info!(
                        "memory consolidated: {} days, {} blocks",
                        r.dates_processed.len(),
                        r.blocks_added
                    );
                }
            }
        }
    });

    // 向量快照定期保存（每 5 分钟）
    let vec_by_assistant_ref = state.shared_vector_by_assistant.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        interval.tick().await;
        loop {
            interval.tick().await;
            let map = vec_by_assistant_ref.read().await;
            for v in map.values() {
                v.save_snapshot();
            }
        }
    });

    // 心跳：若配置启用了 heartbeat，后台定期让 Agent 自主检查待办与反思
    if cfg.heartbeat.enabled {
        let heartbeat_state = Arc::clone(state);
        let interval_secs = cfg.heartbeat.interval_secs;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            interval.tick().await; // 跳过启动后立即执行
            loop {
                interval.tick().await;
                let shared_vec = {
                    let map = heartbeat_state.shared_vector_by_assistant.read().await;
                    map.get("default").cloned()
                };
                let mut context = create_context_with_long_term_for_assistant(
                    &heartbeat_state.config,
                    DEFAULT_MAX_TURNS,
                    Some(&heartbeat_state.workspace),
                    shared_vec,
                    Some("default"),
                );
                let guard = heartbeat_state.components.read().await;
                match process_message(&**guard, &mut context, HEARTBEAT_PROMPT, None).await {
                    Ok(reply) => {
                        tracing::info!("heartbeat ok: {}", reply.trim());
                        append_heartbeat_log(&heartbeat_state.memory_root, &reply);
                    }
                    Err(e) => {
                        tracing::warn!("heartbeat error: {:?}", e);
                        append_heartbeat_log(
                            &heartbeat_state.memory_root,
                            &format!("[heartbeat error] {:?}", e),
                        );
                    }
                }
            }
        });
        tracing::info!("heartbeat enabled, interval {}s", interval_secs);
    }
}

/// bee-web：对话 UI + 静态资源 + 管理 API（与历史行为一致）
#[allow(dead_code)] // bee-admin 二进制不调用；由 `bee-web` 使用
pub async fn run_web_server() -> anyhow::Result<()> {
    init_tracing_subscriber();
    match bee::observability::init_tracing_system().await {
        Ok(_) => tracing::info!("Tracing system initialized"),
        Err(e) => tracing::warn!("Tracing system initialization failed: {}", e),
    }

    let state = build_app_state().await?;
    let cfg = state.config.clone();
    let app = router_chat_and_static()
        .merge(router_admin_api(state.clone()))
        .with_state(Arc::clone(&state));

    spawn_background_tasks(&state, &cfg);

    let port = std::env::var("BEE_WEB_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(cfg.web.port);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Bee Web UI: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// bee-admin：仅管理 REST，默认端口 8081（`BEE_ADMIN_PORT`）
#[allow(dead_code)] // bee-web 二进制不调用；由 `bee-admin` 使用
pub async fn run_admin_server() -> anyhow::Result<()> {
    init_tracing_subscriber();
    match bee::observability::init_tracing_system().await {
        Ok(_) => tracing::info!("Tracing system initialized"),
        Err(e) => tracing::warn!("Tracing system initialization failed: {}", e),
    }

    let state = build_app_state().await?;
    let cfg = state.config.clone();
    let app = router_admin_api(state.clone());

    spawn_background_tasks(&state, &cfg);

    let port = std::env::var("BEE_ADMIN_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8081);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Bee Admin API: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let app_with_state = app.with_state(Arc::clone(&state));
    axum::serve(listener, app_with_state).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../../../static/index.html"))
}

async fn serve_metrics_dashboard() -> Html<&'static str> {
    Html(include_str!("../../../static/metrics.html"))
}

async fn serve_marked_js() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )
        .body(Body::from(include_str!("../../../static/js/marked.min.js")))
        .unwrap()
}

async fn serve_highlight_js() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )
        .body(Body::from(include_str!("../../../static/js/highlight.min.js")))
        .unwrap()
}

async fn serve_highlight_css() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
        .body(Body::from(include_str!(
            "../../../static/css/github-dark.min.css"
        )))
        .unwrap()
}

/// 获取或创建指定助手的向量长期记忆
async fn get_or_create_vector_for_assistant(
    state: &AppState,
    assistant_id: &str,
) -> Option<Arc<InMemoryVectorLongTerm>> {
    let aid = if assistant_id.is_empty() {
        "default"
    } else {
        assistant_id
    };
    {
        let map = state.shared_vector_by_assistant.read().await;
        if let Some(v) = map.get(aid) {
            return Some(Arc::clone(v));
        }
    }
    if let Some(vec) =
        create_vector_long_term_for_assistant(&state.workspace, &state.config, Some(aid))
    {
        let mut map = state.shared_vector_by_assistant.write().await;
        map.insert(aid.to_string(), Arc::clone(&vec));
        Some(vec)
    } else {
        None
    }
}

/// POST /api/memory/consolidate?since_days=7：手动触发记忆整理（截断式），将近期短期日志归纳写入长期记忆
async fn api_memory_consolidate(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConsolidateQuery>,
) -> Result<Json<ConsolidateResponse>, (StatusCode, String)> {
    let since_days = q.since_days.unwrap_or(7);
    let r = consolidate_memory(&state.memory_root, since_days)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ConsolidateResponse {
        dates_processed: r.dates_processed,
        blocks_added: r.blocks_added,
    }))
}

/// POST /api/memory/consolidate-llm?since_days=7：用 LLM 对近期每日日志做摘要后写入长期记忆（EVOLUTION §3.3）
async fn api_memory_consolidate_llm(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ConsolidateQuery>,
) -> Result<Json<ConsolidateResponse>, (StatusCode, String)> {
    let since_days = q.since_days.unwrap_or(7);
    let components = state.components.read().await;
    let r = consolidate_memory_with_llm(&components.planner, &state.workspace, since_days)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ConsolidateResponse {
        dates_processed: r.dates_processed,
        blocks_added: r.blocks_added,
    }))
}

/// POST /api/config/reload：重新加载配置并重建 Agent 组件（LLM/Planner/Recovery/Critic 等），实现运行时多 LLM 后端切换（白皮书 Phase 5）
async fn api_config_reload(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, String)> {
    let _ = bee::config::reload_config();
    let cfg = load_config(None).unwrap_or_default();
    let new_components = Arc::new(create_agent_components(&cfg, &state.workspace));
    let mut guard = state.components.write().await;
    *guard = new_components;
    Ok(StatusCode::OK)
}

/// POST /api/compact：对指定会话执行 Context Compaction（摘要写入长期记忆并替换为摘要消息），请求体 { "session_id": "...", "assistant_id": "..." }
async fn api_compact(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClearSessionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let session_id = match req.session_id.filter(|s| !s.is_empty()) {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                "session_id is required".to_string(),
            ))
        }
    };
    let assistant_id = req.assistant_id.as_deref().unwrap_or("default");
    let scope = req.scope.to_scope(&session_id, assistant_id);
    audit_knowledge_access(&state, assistant_id, &scope, &session_id);
    let key = session_key(&session_id, assistant_id, Some(&scope));
    let vector = get_or_create_vector_for_assistant(&state, assistant_id).await;
    let mut context = state
        .sessions
        .write()
        .await
        .remove(&key)
        .unwrap_or_else(|| {
            load_session_from_disk(
                &state.sessions_dir,
                &session_id,
                assistant_id,
                &state.workspace,
                &state.config,
                vector.clone(),
                Some(&scope),
            )
            .unwrap_or_else(|| {
                create_context_with_long_term_for_assistant(
                    &state.config,
                    DEFAULT_MAX_TURNS,
                    Some(&state.workspace),
                    vector,
                    Some(assistant_id),
                )
            })
        });
    let components = state.components.read().await;
    match compact_context(&components.planner, &mut context).await {
        Ok(()) => {
            save_session_to_disk(
                &state.sessions_dir,
                &state.workspace,
                &session_id,
                assistant_id,
                &context,
                Some(&scope),
            );
            state.sessions.write().await.insert(key, context);
            Ok(StatusCode::OK)
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Compaction failed: {}", e),
        )),
    }
}

/// POST /api/session/clear：清除指定会话（从内存移除并删除磁盘文件），请求体可选 { "session_id": "...", "assistant_id": "..." }
async fn api_session_clear(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClearSessionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let session_id = match req.session_id.filter(|s| !s.is_empty()) {
        Some(s) => s,
        None => return Ok(StatusCode::OK),
    };
    let assistant_id = req.assistant_id.as_deref().unwrap_or("default");
    let scope = req.scope.to_scope(&session_id, assistant_id);
    let key = session_key(&session_id, assistant_id, Some(&scope));
    {
        let mut sessions = state.sessions.write().await;
        sessions.remove(&key);
    }
    let path = session_path(&state.sessions_dir, &session_id, assistant_id, Some(&scope));
    let _ = std::fs::remove_file(&path);
    // 兼容旧格式：若存在 session_id.json 也删除
    if assistant_id == "default" {
        let legacy = state.sessions_dir.join(format!(
            "{}.json",
            session_id.replace('/', "_").replace('\\', "_")
        ));
        let _ = std::fs::remove_file(legacy);
    }
    Ok(StatusCode::OK)
}

/// POST /api/session/cancel：取消指定会话当前正在运行的流式请求
async fn api_session_cancel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CancelSessionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let session_id = match req.session_id.filter(|s| !s.is_empty()) {
        Some(s) => s,
        None => return Ok(StatusCode::OK),
    };
    let assistant_id = req.assistant_id.as_deref().unwrap_or("default");
    let scope = req.scope.to_scope(&session_id, assistant_id);
    let key = session_key(&session_id, assistant_id, Some(&scope));

    let token = {
        let active = state.active_cancellations.read().await;
        active.get(&key).cloned()
    };
    if let Some(token) = token {
        token.cancel();
    }
    Ok(StatusCode::OK)
}

/// GET /api/sessions：列出所有会话（从磁盘读取），按更新时间倒序。每个 (session_id, assistant_id) 为独立会话
async fn api_sessions_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SessionListItem>>, (StatusCode, String)> {
    let mut items = Vec::new();
    let entries = std::fs::read_dir(&state.sessions_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if stem.is_empty() {
            continue;
        }
        let (session_id, assistant_id) = if let Some(idx) = stem.find("---") {
            let (sid, rest) = stem.split_at(idx);
            let assistant_with_scope = rest.trim_start_matches("---");
            let assistant_id = assistant_with_scope
                .split("---")
                .next()
                .unwrap_or(assistant_with_scope);
            (sid.to_string(), assistant_id.to_string())
        } else {
            (stem.to_string(), "default".to_string())
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let snap: SessionSnapshot = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let id = session_key(&session_id, &assistant_id, snap.scope.as_ref());

        let title = snap
            .messages
            .iter()
            .find(|m| {
                matches!(m.role, Role::User) && !m.content.trim().starts_with("Observation from ")
            })
            .map(|m| {
                let t = m.content.trim();
                if t.chars().count() > 50 {
                    format!("{}...", t.chars().take(50).collect::<String>())
                } else {
                    t.to_string()
                }
            })
            .unwrap_or_else(|| "新对话".to_string());

        let (updated_at, date) = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                (
                    dt.format("%m-%d %H:%M").to_string(),
                    dt.format("%Y-%m-%d").to_string(),
                )
            })
            .unwrap_or_else(|_| (String::new(), String::new()));

        items.push(SessionListItem {
            id: id.clone(),
            session_id,
            assistant_id,
            title,
            message_count: snap.messages.len(),
            updated_at,
            date,
            tenant_id: snap
                .scope
                .as_ref()
                .and_then(|scope| scope.tenant_id.clone()),
            organization_id: snap
                .scope
                .as_ref()
                .and_then(|scope| scope.organization_id.clone()),
            team_id: snap.scope.as_ref().and_then(|scope| scope.team_id.clone()),
            agent_instance_id: snap
                .scope
                .as_ref()
                .and_then(|scope| scope.agent_instance_id.clone()),
            user_id: snap.scope.as_ref().and_then(|scope| scope.user_id.clone()),
        });
    }

    items.sort_by(|a, b| b.date.cmp(&a.date).then(b.updated_at.cmp(&a.updated_at)));

    Ok(Json(items))
}

/// POST /api/session/rename：重命名会话（更新标题，存储在元数据中）
async fn api_session_rename(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<RenameSessionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // TODO: 实现会话元数据存储以支持自定义标题
    Ok(StatusCode::OK)
}

/// GET /api/agents：返回动态创建的 sub-agent 列表（Phase 3，含 parent_id 用于树状展示）
async fn api_agents_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<DynamicAgent>>, (StatusCode, String)> {
    let list = dynamic_agent_catalog::load_dynamic_agents(&state.workspace);
    Ok(Json(list))
}

/// POST /api/agents：前端创建 agent，body: { role, guidance? }，parent_id 为 human
async fn api_agents_create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<DynamicAgent>), (StatusCode, String)> {
    let role = req.role.trim().to_string();
    if role.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "role is required".to_string()));
    }
    let create_tool = CreateTool::new(&state.workspace);
    let guidance = req.guidance.as_deref().and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    });
    let agent = create_tool
        .create_agent_direct(&role, guidance.as_deref(), "human")
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    dynamic_agent_catalog::reload_dynamic_agents_into_state(&state).await;
    emit_event(
        &state.event_bus,
        WorkspaceEvent::AgentCreated {
            id: agent.id.clone(),
            role: agent.role.clone(),
            parent_id: agent.parent_id.clone(),
        },
    );
    Ok((StatusCode::CREATED, Json(agent)))
}

/// GET /api/assistants：返回多助手列表（含 skills），供前端选择与配置；动态 agent 从 agents.json 合并
async fn api_assistants_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<AssistantInfo>>, (StatusCode, String)> {
    dynamic_agent_catalog::reload_dynamic_agents_into_state(&state).await;
    let skills = state.assistant_skills.read().await;
    let mut list: Vec<AssistantInfo> = state
        .assistants
        .iter()
        .map(|a| {
            let skills_val = skills.get(&a.id).cloned();
            AssistantInfo {
                id: a.id.clone(),
                name: a.name.clone(),
                description: a.description.clone(),
                skills: skills_val.or(a.skills.clone()),
            }
        })
        .collect();
    let dynamic = dynamic_agent_catalog::load_dynamic_agents(&state.workspace);
    let existing_ids: std::collections::HashSet<String> =
        list.iter().map(|a| a.id.clone()).collect();
    for da in &dynamic {
        if !existing_ids.contains(&da.id) {
            list.push(AssistantInfo {
                id: da.id.clone(),
                name: da.role.clone(),
                description: da.guidance.clone().unwrap_or_else(|| da.role.clone()),
                skills: skills.get(&da.id).cloned(),
            });
        }
    }
    Ok(Json(list))
}

/// GET /api/groups：列出所有群组
async fn api_groups_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<GroupInfo>>, (StatusCode, String)> {
    let groups = state.groups.read().await;
    let list: Vec<GroupInfo> = groups.values().cloned().collect();
    Ok(Json(list))
}

/// POST /api/groups：创建群组，body: { name?, member_ids }，返回创建的群组
async fn api_groups_create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<(StatusCode, Json<GroupInfo>), (StatusCode, String)> {
    if req.member_ids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "member_ids cannot be empty".into()));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let name = req.name.unwrap_or_else(|| format!("群聊 {}", &id[..8]));
    let group = GroupInfo {
        id: id.clone(),
        name: Some(name),
        member_ids: req.member_ids,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    {
        let mut groups = state.groups.write().await;
        groups.insert(id.clone(), group.clone());
        save_groups_to_disk(&state.groups_path, &*groups);
    }
    emit_event(
        &state.event_bus,
        WorkspaceEvent::GroupCreated {
            id: group.id.clone(),
            name: group.name.clone(),
            member_ids: group.member_ids.clone(),
        },
    );
    Ok((StatusCode::CREATED, Json(group)))
}

/// POST /api/organizations/bootstrap：根据行业模板初始化公司、团队、默认 Agent 与工作空间
async fn api_organizations_bootstrap(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BootstrapOrganizationRequest>,
) -> Result<(StatusCode, Json<BootstrapOrganizationResponse>), (StatusCode, String)> {
    let organization_name = req.organization_name.trim().to_string();
    if organization_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "organization_name is required".to_string(),
        ));
    }

    let tenant_id = req
        .tenant_id
        .clone()
        .unwrap_or_else(|| format!("tenant_{}", uuid::Uuid::new_v4()));
    let organization_id = req
        .organization_id
        .clone()
        .unwrap_or_else(|| format!("org_{}", uuid::Uuid::new_v4()));
    require_management_access(
        &state.workspace,
        &req.scope.to_access_context(tenant_id.clone(), None, None),
        AccessRequirement::PlatformAdmin,
    )?;

    let industry = parse_industry_template(req.industry.as_deref())
        .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let bootstrap_req = OrganizationBootstrapRequest {
        tenant_id: tenant_id.clone(),
        organization_id: organization_id.clone(),
        organization_name,
        admin_user_id: req.scope.management_user_id(),
        industry,
        workspace_id: req
            .workspace_id
            .unwrap_or_else(|| format!("ws_{}", uuid::Uuid::new_v4())),
    };
    let plan = build_bootstrap_plan(&bootstrap_req);
    let team_names = plan.teams.iter().map(|team| team.name.clone()).collect();
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let result = persist_bootstrap_plan(&store, &plan)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let workspace_id_for_audit = result.workspace_id.clone();
    let _ = write_audit_log(
        &state.workspace,
        req.scope.to_audit_actor(
            result.tenant_id.clone(),
            Some(result.organization_id.clone()),
            None,
        ),
        "organization.bootstrap",
        "organization",
        result.organization_id.clone(),
        serde_json::json!({
            "workspace_id": workspace_id_for_audit,
            "industry": industry_template_code(industry),
            "team_count": result.team_count,
            "agent_template_count": result.agent_template_count,
            "agent_instance_count": result.agent_instance_count
        }),
    );

    Ok((
        StatusCode::CREATED,
        Json(BootstrapOrganizationResponse {
            tenant_id: result.tenant_id,
            organization_id: result.organization_id,
            workspace_id: result.workspace_id,
            industry: industry_template_code(industry).to_string(),
            team_count: result.team_count,
            team_names,
            agent_template_count: result.agent_template_count,
            agent_instance_count: result.agent_instance_count,
        }),
    ))
}

/// GET /api/agent-templates：列出租户下可用模板
async fn api_agent_templates_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AgentTemplatesQuery>,
) -> Result<Json<Vec<AgentTemplateSummary>>, (StatusCode, String)> {
    let tenant_id = query
        .tenant_id
        .unwrap_or_else(|| "tenant-default".to_string());
    require_management_access(
        &state.workspace,
        &query.scope.to_access_context(
            tenant_id.clone(),
            query.scope.management_organization_id(),
            None,
        ),
        AccessRequirement::OrgAdmin,
    )?;
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let templates = list_team_templates(&store, &tenant_id)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(
        templates
            .into_iter()
            .map(|template| AgentTemplateSummary {
                id: template.id,
                tenant_id: template.tenant_id,
                name: template.name,
                description: template.description,
                model_id: template.model_id,
                tool_ids: template.tool_ids,
                knowledge_base_ids: template.knowledge_base_ids,
            })
            .collect(),
    ))
}

/// POST /api/teams/:team_id/agent-instances/bootstrap：把模板实例化到团队
async fn api_team_agent_instances_bootstrap(
    Path(team_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<InstantiateTeamTemplatesRequest>,
) -> Result<(StatusCode, Json<InstantiateTeamTemplatesResponse>), (StatusCode, String)> {
    let organization_id = req.organization_id.trim().to_string();
    if organization_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "organization_id is required".to_string(),
        ));
    }
    let team_id = team_id.trim().to_string();
    if team_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "team_id is required".to_string()));
    }

    let tenant_id = req
        .tenant_id
        .unwrap_or_else(|| "tenant-default".to_string());
    require_management_access(
        &state.workspace,
        &req.scope.to_access_context(
            tenant_id.clone(),
            Some(organization_id.clone()),
            Some(team_id.clone()),
        ),
        AccessRequirement::TeamAdmin,
    )?;
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let template_ids = req.template_ids.clone();
    let result = instantiate_team_templates(
        &store,
        &TeamTemplateInstantiationRequest {
            tenant_id: tenant_id.clone(),
            organization_id: organization_id.clone(),
            team_id: team_id.clone(),
            template_ids: template_ids.clone(),
        },
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;
    let _ = write_audit_log(
        &state.workspace,
        req.scope.to_audit_actor(
            tenant_id.clone(),
            Some(organization_id.clone()),
            Some(team_id.clone()),
        ),
        "team.agent_instances.bootstrap",
        "team",
        team_id.clone(),
        serde_json::json!({
            "template_ids": template_ids,
            "created_count": result.created_count,
            "existing_count": result.existing_count,
            "instance_ids": result.instances.iter().map(|instance| instance.id.clone()).collect::<Vec<_>>()
        }),
    );

    Ok((
        StatusCode::CREATED,
        Json(InstantiateTeamTemplatesResponse {
            tenant_id,
            organization_id,
            team_id,
            created_count: result.created_count,
            existing_count: result.existing_count,
            instance_ids: result
                .instances
                .into_iter()
                .map(|instance| instance.id)
                .collect(),
        }),
    ))
}

/// GET /api/task-board：按看板列返回当前组织/团队任务
async fn api_task_board(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TaskBoardQuery>,
) -> Result<Json<Vec<TaskBoardColumn>>, (StatusCode, String)> {
    let tenant_id = query
        .tenant_id
        .clone()
        .unwrap_or_else(|| query.scope.management_tenant_id());
    let organization_id = query
        .organization_id
        .clone()
        .or_else(|| query.scope.management_organization_id());
    let team_id = query
        .team_id
        .clone()
        .or_else(|| query.scope.team_id.clone());
    require_management_access(
        &state.workspace,
        &query
            .scope
            .to_access_context(tenant_id.clone(), organization_id.clone(), team_id.clone()),
        if team_id.is_some() {
            AccessRequirement::TeamAdmin
        } else {
            AccessRequirement::OrgAdmin
        },
    )?;
    let tasks = state
        .task_repo()
        .list_filtered(
            None,
            Some(tenant_id.as_str()),
            organization_id.as_deref(),
            team_id.as_deref(),
            None,
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(build_task_board(
        &tasks,
        Some(tenant_id.as_str()),
        organization_id.as_deref(),
        team_id.as_deref(),
    )))
}

/// GET /api/workflow-templates：内置模板 + 租户已发布模板合并（同 slug 时租户覆盖内置）
async fn api_workflow_templates_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WorkflowTemplatesQuery>,
) -> Result<Json<Vec<WorkflowTemplateSummary>>, (StatusCode, String)> {
    let tenant_id = query.scope.management_tenant_id();
    require_management_access(
        &state.workspace,
        &query.scope.to_access_context(
            tenant_id.clone(),
            query.scope.management_organization_id(),
            query.scope.team_id.clone(),
        ),
        AccessRequirement::OrgAdmin,
    )?;
    Ok(Json(merged_workflow_templates_for_tenant(
        &tenant_id,
        &state.workspace,
    )))
}

/// POST /api/workflows/start：根据产品级模板创建一组任务
async fn api_workflows_start(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartWorkflowRequest>,
) -> Result<(StatusCode, Json<ProductWorkflowRunResult>), (StatusCode, String)> {
    let tenant_id = req
        .tenant_id
        .clone()
        .unwrap_or_else(|| req.scope.management_tenant_id());
    let organization_id = req
        .organization_id
        .clone()
        .or_else(|| req.scope.management_organization_id());
    let team_id = req.team_id.clone().or_else(|| req.scope.team_id.clone());
    let title = req.title.trim().to_string();
    if title.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "title is required".to_string()));
    }
    require_management_access(
        &state.workspace,
        &req.scope
            .to_access_context(tenant_id.clone(), organization_id.clone(), team_id.clone()),
        if team_id.is_some() {
            AccessRequirement::TeamAdmin
        } else {
            AccessRequirement::OrgAdmin
        },
    )?;

    let template_key = req.template_id.trim().to_string();
    if template_key.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "template_id is required".to_string()));
    }
    let resolved = resolve_workflow_template_for_start(
        &state.workspace,
        &tenant_id,
        &template_key,
        req.template_version,
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace)).ok();
    let workflow = start_workflow_run(
        &WorkflowStartRequest {
            tenant_id: Some(tenant_id.clone()),
            organization_id: organization_id.clone(),
            team_id: team_id.clone(),
            title,
            description: req.description.clone(),
            template_id: template_key.clone(),
            template_version: req.template_version,
        },
        &resolved,
        store.as_ref(),
    )
    .map_err(|err| (StatusCode::BAD_REQUEST, err))?;
    for task in &workflow.tasks {
        emit_event(
            &state.event_bus,
            WorkspaceEvent::TaskCreated {
                id: task.id.clone(),
                title: task.title.clone(),
            },
        );
    }
    state
        .task_repo()
        .append(&workflow.tasks)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let _ = write_audit_log(
        &state.workspace,
        req.scope
            .to_audit_actor(tenant_id.clone(), organization_id.clone(), team_id.clone()),
        "workflow.start",
        "workflow_run",
        workflow.workflow_run_id.clone(),
        serde_json::json!({
            "workflow_template_id": workflow.workflow_template_id,
            "workflow_template_version": workflow.workflow_template_version,
            "task_ids": workflow.tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>(),
            "team_id": team_id,
        }),
    );

    Ok((StatusCode::CREATED, Json(workflow)))
}

// --- 工作流模板管理（专家 / OrgAdmin）---

#[derive(Debug, Deserialize)]
struct AdminWorkflowTemplatesListQuery {
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Serialize)]
struct AdminWorkflowVersionSummary {
    version: i32,
    published_at: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct AdminWorkflowTemplateDetail {
    id: String,
    slug: String,
    name: String,
    description: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
    versions: Vec<AdminWorkflowVersionSummary>,
}

#[derive(Debug, Serialize)]
struct AdminWorkflowTemplatesListResponse {
    templates: Vec<AdminWorkflowTemplateDetail>,
}

#[derive(Debug, Deserialize)]
struct AdminCreateWorkflowTemplateBody {
    slug: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    definition: WorkflowDefinitionJson,
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct AdminAddWorkflowVersionBody {
    definition: WorkflowDefinitionJson,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct AdminPublishWorkflowTemplateBody {
    version: i32,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Serialize)]
struct AdminWorkflowTemplateCreateResponse {
    id: String,
    slug: String,
}

async fn api_admin_workflow_templates_list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AdminWorkflowTemplatesListQuery>,
) -> Result<Json<AdminWorkflowTemplatesListResponse>, (StatusCode, String)> {
    let tenant_id = q
        .tenant_id
        .clone()
        .unwrap_or_else(|| q.scope.management_tenant_id());
    require_management_access(
        &state.workspace,
        &q.scope.to_access_context(
            tenant_id.clone(),
            q.scope.management_organization_id(),
            q.scope.team_id.clone(),
        ),
        AccessRequirement::OrgAdmin,
    )?;
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let rows = store
        .list_workflow_templates_for_tenant(&tenant_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut templates = Vec::with_capacity(rows.len());
    for r in rows {
        let versions_raw = store
            .list_workflow_template_versions(&r.id)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let versions = versions_raw
            .into_iter()
            .map(|v| AdminWorkflowVersionSummary {
                version: v.version,
                published_at: v.published_at,
                created_at: v.created_at,
            })
            .collect();
        templates.push(AdminWorkflowTemplateDetail {
            id: r.id,
            slug: r.slug,
            name: r.name,
            description: r.description,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
            versions,
        });
    }
    Ok(Json(AdminWorkflowTemplatesListResponse { templates }))
}

async fn api_admin_workflow_templates_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AdminCreateWorkflowTemplateBody>,
) -> Result<(StatusCode, Json<AdminWorkflowTemplateCreateResponse>), (StatusCode, String)> {
    let tenant_id = body
        .tenant_id
        .clone()
        .unwrap_or_else(|| body.scope.management_tenant_id());
    require_management_access(
        &state.workspace,
        &body.scope.to_access_context(
            tenant_id.clone(),
            body.scope.management_organization_id(),
            body.scope.team_id.clone(),
        ),
        AccessRequirement::OrgAdmin,
    )?;
    let slug = body.slug.trim().to_string();
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err((
            StatusCode::BAD_REQUEST,
            "slug must be non-empty alphanumeric/underscore/dash".to_string(),
        ));
    }
    if body.definition.steps.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "definition.steps must not be empty".to_string(),
        ));
    }
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name is required".to_string()));
    }
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    let record = WorkflowTemplateRecord {
        id: id.clone(),
        tenant_id: tenant_id.clone(),
        slug: slug.clone(),
        name,
        description: body.description.clone(),
        status: "draft".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    store
        .create_workflow_template(&record, &body.definition)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let _ = write_audit_log(
        &state.workspace,
        body
            .scope
            .to_audit_actor(tenant_id.clone(), body.scope.management_organization_id(), body.scope.team_id.clone()),
        "workflow_template.create",
        "workflow_template",
        id.clone(),
        serde_json::json!({ "slug": slug }),
    );
    Ok((
        StatusCode::CREATED,
        Json(AdminWorkflowTemplateCreateResponse { id, slug }),
    ))
}

async fn api_admin_workflow_template_add_version(
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<String>,
    Json(body): Json<AdminAddWorkflowVersionBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let tenant_id = body.scope.management_tenant_id();
    require_management_access(
        &state.workspace,
        &body.scope.to_access_context(
            tenant_id.clone(),
            body.scope.management_organization_id(),
            body.scope.team_id.clone(),
        ),
        AccessRequirement::OrgAdmin,
    )?;
    if body.definition.steps.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "definition.steps must not be empty".to_string(),
        ));
    }
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let meta = store
        .get_workflow_template_by_id(&template_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "template not found".to_string()))?;
    if meta.tenant_id != tenant_id {
        return Err((StatusCode::FORBIDDEN, "template tenant mismatch".to_string()));
    }
    let version = store
        .add_workflow_template_version(&template_id, &body.definition)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let _ = write_audit_log(
        &state.workspace,
        body
            .scope
            .to_audit_actor(tenant_id.clone(), body.scope.management_organization_id(), body.scope.team_id.clone()),
        "workflow_template.version.create",
        "workflow_template",
        template_id.clone(),
        serde_json::json!({ "version": version }),
    );
    Ok(Json(serde_json::json!({ "version": version })))
}

async fn api_admin_workflow_template_publish(
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<String>,
    Json(body): Json<AdminPublishWorkflowTemplateBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    let tenant_id = body.scope.management_tenant_id();
    require_management_access(
        &state.workspace,
        &body.scope.to_access_context(
            tenant_id.clone(),
            body.scope.management_organization_id(),
            body.scope.team_id.clone(),
        ),
        AccessRequirement::OrgAdmin,
    )?;
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let meta = store
        .get_workflow_template_by_id(&template_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "template not found".to_string()))?;
    if meta.tenant_id != tenant_id {
        return Err((StatusCode::FORBIDDEN, "template tenant mismatch".to_string()));
    }
    store
        .publish_workflow_template_version(&template_id, body.version)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let _ = write_audit_log(
        &state.workspace,
        body
            .scope
            .to_audit_actor(tenant_id.clone(), body.scope.management_organization_id(), body.scope.team_id.clone()),
        "workflow_template.publish",
        "workflow_template",
        template_id.clone(),
        serde_json::json!({ "version": body.version }),
    );
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/tasks：列出所有任务（可选 status 过滤）
async fn api_tasks_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<Task>>, (StatusCode, String)> {
    let status_filter = query.get("status").and_then(|s| match s.as_str() {
        "todo" => Some(TaskStatus::Todo),
        "in_progress" => Some(TaskStatus::InProgress),
        "done" => Some(TaskStatus::Done),
        _ => None,
    });
    let tenant_filter = query.get("tenant_id").cloned();
    let org_filter = query.get("organization_id").cloned();
    let team_filter = query.get("team_id").cloned();
    let workflow_run_filter = query.get("workflow_run_id").cloned();
    let list = state
        .task_repo()
        .list_filtered(
            status_filter,
            tenant_filter.as_deref(),
            org_filter.as_deref(),
            team_filter.as_deref(),
            workflow_run_filter.as_deref(),
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(list))
}

/// POST /api/tasks：创建任务，可选 assignee_ids 自动建群
async fn api_tasks_create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<Task>), (StatusCode, String)> {
    let title = req.title.trim().to_string();
    if title.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "title is required".to_string()));
    }
    let assignee_ids: Vec<String> = req
        .assignee_ids
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let group_id = if assignee_ids.len() >= 2 {
        let now = chrono::Utc::now().to_rfc3339();
        let gid = uuid::Uuid::new_v4().to_string();
        let group = GroupInfo {
            id: gid.clone(),
            name: Some(format!(
                "任务: {}",
                title.chars().take(20).collect::<String>()
            )),
            member_ids: assignee_ids.clone(),
            created_at: now.clone(),
        };
        {
            let mut groups = state.groups.write().await;
            groups.insert(gid.clone(), group);
            save_groups_to_disk(&state.groups_path, &*groups);
        }
        emit_event(
            &state.event_bus,
            WorkspaceEvent::GroupCreated {
                id: gid.clone(),
                name: Some(format!(
                    "任务: {}",
                    title.chars().take(20).collect::<String>()
                )),
                member_ids: assignee_ids.clone(),
            },
        );
        Some(gid)
    } else {
        None
    };
    let task = build_task(
        &req,
        assignee_ids,
        group_id.clone(),
        req.tenant_id
            .clone()
            .or_else(|| Some("tenant-default".to_string())),
        req.organization_id
            .clone()
            .or_else(|| Some("org-default".to_string())),
        req.team_id.clone(),
        req.workflow_template_id.clone(),
        req.workflow_run_id.clone(),
        req.internal_group || group_id.is_some(),
    );
    state
        .task_repo()
        .upsert(&task)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    emit_event(
        &state.event_bus,
        WorkspaceEvent::TaskCreated {
            id: task.id.clone(),
            title: task.title.clone(),
        },
    );
    Ok((StatusCode::CREATED, Json(task)))
}

/// PATCH /api/tasks/:id：更新任务
async fn api_tasks_update(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<Task>, (StatusCode, String)> {
    let task = task_repository::patch_task(
        &state.workspace,
        state.task_persistence,
        &task_id,
        |task| apply_task_update(task, req),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, "task not found".to_string()))?;
    emit_event(
        &state.event_bus,
        WorkspaceEvent::TaskUpdated {
            id: task.id.clone(),
            status: status_label(task.status).to_string(),
        },
    );
    Ok(Json(task))
}

/// POST /api/tasks/:id/start：启动任务统筹，由 coordinator agent 执行规划与组队
async fn api_tasks_start(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    dynamic_agent_catalog::reload_dynamic_agents_into_state(&state).await;
    task_coordinator_service::start_task(Arc::clone(&state), task_id).await
}

/// POST /api/inbox/process：处理指定 assistant 的收件箱（P2P 未读消息触发 ReAct）
async fn api_inbox_process(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InboxProcessRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    dynamic_agent_catalog::reload_dynamic_agents_into_state(&state).await;
    let assistant_id = req.assistant_id.trim();
    if assistant_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "assistant_id is required".to_string(),
        ));
    }
    let processed = inbox_service::process_inbox(Arc::clone(&state), assistant_id).await?;

    Ok(Json(serde_json::json!({
        "processed": processed,
        "assistant_id": assistant_id
    })))
}

/// GET /api/tools：返回可用工具列表，供技能配置使用
async fn api_tools_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ToolInfo>>, (StatusCode, String)> {
    let list: Vec<ToolInfo> = state
        .tool_descriptions
        .iter()
        .map(|(id, desc)| ToolInfo {
            id: id.clone(),
            name: id.clone(),
            description: desc.clone(),
        })
        .collect();
    Ok(Json(list))
}

/// GET /api/tool-policies：查询租户/组织/团队工具策略
async fn api_tool_policies_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ToolPoliciesQuery>,
) -> Result<Json<Vec<bee::saas::ToolAccessPolicy>>, (StatusCode, String)> {
    let tenant_id = query
        .tenant_id
        .unwrap_or_else(|| query.scope.management_tenant_id());
    let organization_id = query
        .organization_id
        .clone()
        .or_else(|| query.scope.management_organization_id());
    require_management_access(
        &state.workspace,
        &query
            .scope
            .to_access_context(tenant_id.clone(), organization_id.clone(), None),
        AccessRequirement::OrgAdmin,
    )?;

    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let policies = list_tool_policies(&store, &tenant_id, organization_id.as_deref())
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(policies))
}

/// PUT /api/tool-policies：更新租户/组织/团队工具策略
async fn api_tool_policies_put(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateToolPolicyRequest>,
) -> Result<Json<bee::saas::ToolAccessPolicy>, (StatusCode, String)> {
    let tenant_id = req
        .tenant_id
        .clone()
        .unwrap_or_else(|| req.scope.management_tenant_id());
    let organization_id = req
        .organization_id
        .clone()
        .or_else(|| req.scope.management_organization_id());
    let team_id = req.team_id.clone();
    let requirement = if team_id.is_some() {
        AccessRequirement::TeamAdmin
    } else {
        AccessRequirement::OrgAdmin
    };
    require_management_access(
        &state.workspace,
        &req.scope
            .to_access_context(tenant_id.clone(), organization_id.clone(), team_id.clone()),
        requirement,
    )?;

    let all_tools: std::collections::HashSet<_> = state
        .tool_descriptions
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let allowed_tool_ids: Vec<String> = req
        .allowed_tool_ids
        .into_iter()
        .filter(|tool| all_tools.contains(tool.as_str()))
        .collect();
    let denied_tool_ids: Vec<String> = req
        .denied_tool_ids
        .into_iter()
        .filter(|tool| all_tools.contains(tool.as_str()))
        .collect();

    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let policy = upsert_tool_policy(
        &store,
        ToolPolicyInput {
            scope: ToolPolicyScope {
                tenant_id: tenant_id.clone(),
                organization_id: organization_id.clone(),
                team_id: team_id.clone(),
            },
            allowed_tool_ids,
            denied_tool_ids,
        },
    )
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let _ = write_audit_log(
        &state.workspace,
        req.scope
            .to_audit_actor(tenant_id, organization_id.clone(), team_id.clone()),
        "tool.policy.update",
        if team_id.is_some() {
            "team_tool_policy"
        } else if organization_id.is_some() {
            "organization_tool_policy"
        } else {
            "tenant_tool_policy"
        },
        policy.id.clone(),
        serde_json::json!({
            "organization_id": organization_id,
            "team_id": team_id,
            "allowed_tool_ids": policy.allowed_tool_ids,
            "denied_tool_ids": policy.denied_tool_ids
        }),
    );
    Ok(Json(policy))
}

/// GET /api/audit-logs：查询租户/组织审计日志
async fn api_audit_logs_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditLogsQuery>,
) -> Result<Json<Vec<bee::saas::AuditLogRecord>>, (StatusCode, String)> {
    let tenant_id = query
        .tenant_id
        .unwrap_or_else(|| "tenant-default".to_string());
    require_management_access(
        &state.workspace,
        &query.scope.to_access_context(
            tenant_id.clone(),
            query
                .organization_id
                .clone()
                .or_else(|| query.scope.management_organization_id()),
            None,
        ),
        AccessRequirement::OrgAdmin,
    )?;
    let store = SaasSqliteStore::new(saas_db_path(&state.workspace))
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let logs = list_audit_logs(
        &store,
        &tenant_id,
        query.organization_id.as_deref(),
        query.limit,
    )
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(logs))
}

#[derive(Debug, Deserialize)]
struct UpdateSkillsRequest {
    skills: Vec<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

#[derive(Debug, Deserialize)]
struct UpdateKnowledgeBasesRequest {
    knowledge_base_ids: Vec<String>,
    #[serde(flatten)]
    scope: WebScopeParams,
}

/// PUT /api/assistant/:id/skills：更新该智能体的技能配置，持久化到 config/assistant_skills.json
async fn api_assistant_skills_put(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<UpdateSkillsRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if id == "auto" {
        return Err((
            StatusCode::BAD_REQUEST,
            "无法配置自动分派助手的技能".to_string(),
        ));
    }
    let all_tools: std::collections::HashSet<_> = state
        .tool_descriptions
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    require_management_access(
        &state.workspace,
        &req.scope.to_access_context(
            req.scope.management_tenant_id(),
            req.scope.management_organization_id(),
            None,
        ),
        AccessRequirement::OrgAdmin,
    )?;
    let skills: Vec<String> = req
        .skills
        .into_iter()
        .filter(|n| all_tools.contains(n.as_str()))
        .collect();

    let tool_schema = tool_call_schema_json();
    let base = &state.config_base;
    let tool_descriptions = &state.tool_descriptions;
    let entry = state
        .assistant_entries
        .get(&id)
        .cloned()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "智能体不存在".to_string()))?;
    let full = build_prompt_with_skills(base, &entry, &skills, tool_descriptions, &tool_schema);

    {
        let mut prompts = state.assistant_prompts.write().await;
        prompts.insert(id.clone(), full);
    }
    {
        let mut skills_map = state.assistant_skills.write().await;
        skills_map.insert(id.clone(), skills.clone());
    }

    let mut overrides = load_skills_overrides(base);
    overrides.insert(id.clone(), skills.clone());
    save_skills_overrides(base, &overrides).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("保存配置失败: {}", e),
        )
    })?;

    let db_path = saas_db_path(&state.workspace);
    let store = SaasSqliteStore::new(&db_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("打开模板仓储失败: {}", e),
        )
    })?;
    let repo = SaasTemplateRepository::new(&store);
    let template_id = platform_template_id(&id);
    let now = chrono::Utc::now().to_rfc3339();
    let mut template = repo
        .get_agent_template(&template_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("读取模板失败: {}", e),
            )
        })?
        .unwrap_or_else(|| bee::saas::AgentTemplate {
            id: template_id.clone(),
            tenant_id: "tenant-default".to_string(),
            name: entry.name.clone(),
            description: Some(entry.description.clone()),
            prompt: Some(build_prompt_with_skills(
                base,
                &entry,
                &[],
                tool_descriptions,
                "",
            )),
            tool_ids: Vec::new(),
            model_id: None,
            knowledge_base_ids: entry.knowledge_bases.clone().unwrap_or_default(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    template.name = entry.name.clone();
    template.description = Some(entry.description.clone());
    template.tool_ids = skills;
    template.updated_at = now;
    repo.upsert_agent_template(&template).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("写入模板失败: {}", e),
        )
    })?;
    let _ = write_audit_log(
        &state.workspace,
        req.scope.to_audit_actor(
            req.scope.management_tenant_id(),
            req.scope.management_organization_id(),
            None,
        ),
        "assistant.skills.update",
        "agent_template",
        template_id,
        serde_json::json!({
            "assistant_id": id,
            "skills": template.tool_ids
        }),
    );
    Ok(StatusCode::OK)
}

/// PUT /api/assistant/:id/knowledge-bases：更新该智能体的知识库绑定，持久化到 config/assistant_knowledge.json
async fn api_assistant_knowledge_bases_put(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<UpdateKnowledgeBasesRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if id == "auto" {
        return Err((
            StatusCode::BAD_REQUEST,
            "无法配置自动分派助手的知识库".to_string(),
        ));
    }

    require_management_access(
        &state.workspace,
        &req.scope.to_access_context(
            req.scope.management_tenant_id(),
            req.scope.management_organization_id(),
            None,
        ),
        AccessRequirement::OrgAdmin,
    )?;

    let knowledge_base_ids: Vec<String> = req
        .knowledge_base_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();

    let entry = state
        .assistant_entries
        .get(&id)
        .cloned()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "智能体不存在".to_string()))?;

    let mut overrides = load_knowledge_overrides(&state.config_base);
    overrides.insert(id.clone(), knowledge_base_ids.clone());
    save_knowledge_overrides(&state.config_base, &overrides).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("保存知识库配置失败: {}", e),
        )
    })?;

    let db_path = saas_db_path(&state.workspace);
    let store = SaasSqliteStore::new(&db_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("打开模板仓储失败: {}", e),
        )
    })?;
    let repo = SaasTemplateRepository::new(&store);
    let template_id = platform_template_id(&id);
    let now = chrono::Utc::now().to_rfc3339();
    let mut template = repo
        .get_agent_template(&template_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("读取模板失败: {}", e),
            )
        })?
        .unwrap_or_else(|| bee::saas::AgentTemplate {
            id: template_id.clone(),
            tenant_id: "tenant-default".to_string(),
            name: entry.name.clone(),
            description: Some(entry.description.clone()),
            prompt: Some(
                entry
                    .prompt_text
                    .clone()
                    .unwrap_or_else(|| entry.prompt.clone()),
            ),
            tool_ids: entry.skills.clone().unwrap_or_default(),
            model_id: None,
            knowledge_base_ids: Vec::new(),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    template.name = entry.name.clone();
    template.description = Some(entry.description.clone());
    template.knowledge_base_ids = knowledge_base_ids;
    template.updated_at = now;
    repo.upsert_agent_template(&template).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("写入模板失败: {}", e),
        )
    })?;
    let _ = write_audit_log(
        &state.workspace,
        req.scope.to_audit_actor(
            req.scope.management_tenant_id(),
            req.scope.management_organization_id(),
            None,
        ),
        "assistant.knowledge_bases.update",
        "agent_template",
        template_id,
        serde_json::json!({
            "assistant_id": id,
            "knowledge_base_ids": template.knowledge_base_ids
        }),
    );
    Ok(StatusCode::OK)
}

/// GET /api/models：返回可切换模型列表（id、name）
async fn api_models_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ModelInfo>>, (StatusCode, String)> {
    Ok(Json(state.models.clone()))
}

/// GET /api/skills：返回所有技能列表
async fn api_skills_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SkillInfo>>, (StatusCode, String)> {
    let cache = state.skill_loader.cache();
    let skills = cache.read().await;
    let list: Vec<SkillInfo> = skills.values().map(SkillInfo::from).collect();
    Ok(Json(list))
}

/// GET /api/skills/:id：获取单个技能详情
async fn api_skill_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<SkillInfo>, (StatusCode, String)> {
    let skill = state
        .skill_loader
        .get(&id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("技能 {} 不存在", id)))?;
    Ok(Json(SkillInfo::from(&skill)))
}

#[derive(Debug, Deserialize)]
struct UpdateSkillRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    capability: Option<String>,
    #[serde(default)]
    template: Option<String>,
}

/// PUT /api/skills/:id：更新技能（保存到文件）
async fn api_skill_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<UpdateSkillRequest>,
) -> Result<Json<SkillInfo>, (StatusCode, String)> {
    let skill = state
        .skill_loader
        .get(&id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("技能 {} 不存在", id)))?;

    let skill_dir = &skill.dir;

    if req.name.is_some() || req.description.is_some() || req.tags.is_some() {
        let mut meta = skill.meta.clone();
        if let Some(name) = req.name {
            meta.name = name;
        }
        if let Some(description) = req.description {
            meta.description = description;
        }
        if let Some(tags) = req.tags {
            meta.tags = tags;
        }

        let toml_content = format!(
            "[skill]\nid = \"{}\"\nname = \"{}\"\ndescription = \"{}\"\ntags = {:?}\n",
            meta.id, meta.name, meta.description, meta.tags
        );
        if let Some(script) = &meta.script {
            let toml_content = format!("{}script = \"{}\"\n", toml_content, script);
            std::fs::write(skill_dir.join("skill.toml"), toml_content)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        } else {
            std::fs::write(skill_dir.join("skill.toml"), toml_content)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    if let Some(capability) = &req.capability {
        std::fs::write(skill_dir.join("capability.md"), capability)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(template) = &req.template {
        if template.is_empty() {
            let _ = std::fs::remove_file(skill_dir.join("template.md"));
        } else {
            std::fs::write(skill_dir.join("template.md"), template)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    state
        .skill_loader
        .load_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updated = state.skill_loader.get(&id).await.ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "重新加载失败".to_string(),
        )
    })?;
    Ok(Json(SkillInfo::from(&updated)))
}

/// OpenClaw skill.json 格式
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenClawSkillJson {
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

/// 导入 OpenClaw 技能请求
#[derive(Debug, Deserialize)]
struct ImportOpenClawRequest {
    /// OpenClaw skill.json 内容 (JSON 字符串)
    skill_json: String,
    /// SKILL.md 内容
    #[serde(default)]
    skill_md: Option<String>,
    /// 可选：覆盖已有的同名技能
    #[serde(default)]
    overwrite: bool,
}

/// 导入 OpenClaw 技能：将 OpenClaw 格式转换为 Bee 格式并保存
async fn api_skill_import_openclaw(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportOpenClawRequest>,
) -> Result<Json<SkillInfo>, (StatusCode, String)> {
    let openclaw: OpenClawSkillJson = serde_json::from_str(&req.skill_json)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("无效的 skill.json: {}", e)))?;

    let skill_id = openclaw
        .name
        .to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "");

    if skill_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "技能名称无效".to_string()));
    }

    let existing = state.skill_loader.get(&skill_id).await;
    if existing.is_some() && !req.overwrite {
        return Err((
            StatusCode::CONFLICT,
            format!("技能 '{}' 已存在，使用 overwrite=true 覆盖", skill_id),
        ));
    }

    let skill_dir = state.skill_loader.skills_dir().join(&skill_id);
    std::fs::create_dir_all(&skill_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("创建目录失败: {}", e),
        )
    })?;

    let description = openclaw
        .description
        .as_deref()
        .unwrap_or("从 OpenClaw 导入的技能");
    let tags = openclaw.tags.unwrap_or_default();
    let toml_content = format!(
        "[skill]\nid = \"{}\"\nname = \"{}\"\ndescription = \"{}\"\ntags = {:?}\n",
        skill_id, openclaw.name, description, tags
    );
    std::fs::write(skill_dir.join("skill.toml"), toml_content).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("写入 skill.toml 失败: {}", e),
        )
    })?;

    let capability = req.skill_md.unwrap_or_else(|| {
        format!(
            "# {}\n\n{}\n\n- Author: {}\n- License: {}",
            openclaw.name,
            description,
            openclaw.author.as_deref().unwrap_or("unknown"),
            openclaw.license.as_deref().unwrap_or("MIT")
        )
    });
    std::fs::write(skill_dir.join("capability.md"), capability).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("写入 capability.md 失败: {}", e),
        )
    })?;

    state
        .skill_loader
        .load_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let imported = state.skill_loader.get(&skill_id).await.ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "导入后无法加载技能".to_string(),
        )
    })?;

    tracing::info!("Imported OpenClaw skill: {} ({})", openclaw.name, skill_id);
    Ok(Json(SkillInfo::from(&imported)))
}

/// GET /api/history?session_id=...&assistant_id=... 或 ?group_id=...：返回该会话的对话列表，过滤掉 Tool call / Observation 等内部消息
async fn api_history(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, (StatusCode, String)> {
    if let Some(ref gid) = q.group_id.filter(|s| !s.is_empty()) {
        let group_msgs = load_group_session(&state.sessions_dir, gid);
        let messages: Vec<HistoryMessage> = group_msgs
            .into_iter()
            .map(|m| HistoryMessage {
                role: m.role,
                content: m.content,
                assistant_id: m.assistant_id,
            })
            .collect();
        return Ok(Json(HistoryResponse {
            session_id: gid.clone(),
            messages,
        }));
    }
    let session_id = match q.session_id.filter(|s| !s.is_empty()) {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                "session_id or group_id is required".to_string(),
            ))
        }
    };
    let assistant_id = q.assistant_id.as_deref().unwrap_or("default");
    let scope = q.scope.to_scope(&session_id, assistant_id);
    let key = session_key(&session_id, assistant_id, Some(&scope));
    let vector = get_or_create_vector_for_assistant(&state, assistant_id).await;
    let context_opt = {
        let sessions = state.sessions.read().await;
        sessions.get(&key).cloned()
    };
    let context = match context_opt {
        Some(c) => c,
        None => {
            if let Some(loaded) = load_session_from_disk(
                &state.sessions_dir,
                &session_id,
                assistant_id,
                &state.workspace,
                &state.config,
                vector,
                Some(&scope),
            ) {
                loaded
            } else {
                return Ok(Json(HistoryResponse {
                    session_id: session_id.clone(),
                    messages: vec![],
                }));
            }
        }
    };
    // 主聊天区不展示内部消息：User 的 "Observation from ..."、Assistant 的 "Tool call: ..."
    let messages: Vec<HistoryMessage> = context
        .messages()
        .iter()
        .filter(|m| !matches!(m.role, Role::System))
        .filter(|m| {
            let c = m.content.trim();
            if matches!(m.role, Role::User) {
                !c.starts_with("Observation from ") && !c.starts_with("Critic 建议：")
            } else {
                !c.starts_with("Tool call:") // 任意 "Tool call:..." 均过滤，不依赖 " | Result: "
            }
        })
        .map(|m: &Message| HistoryMessage {
            role: match m.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
                Role::System => "system".to_string(),
                Role::Tool => "tool".to_string(),
            },
            content: m.content.clone(),
            assistant_id: None,
        })
        .collect();
    Ok(Json(HistoryResponse {
        session_id: session_id.clone(),
        messages,
    }))
}

async fn api_chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let message = req.message.trim();
    if message.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is required".to_string()));
    }

    let session_id = req
        .session_id
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let assistant_id = req.assistant_id.as_deref().unwrap_or("default");
    let scope = req.scope.to_scope(&session_id, assistant_id);
    let key = session_key(&session_id, assistant_id, Some(&scope));
    let vector = get_or_create_vector_for_assistant(&state, assistant_id).await;
    let mut context = {
        let mut sessions = state.sessions.write().await;
        sessions.remove(&key).unwrap_or_else(|| {
            load_session_from_disk(
                &state.sessions_dir,
                &session_id,
                assistant_id,
                &state.workspace,
                &state.config,
                vector.clone(),
                Some(&scope),
            )
            .unwrap_or_else(|| {
                create_context_with_long_term_for_assistant(
                    &state.config,
                    DEFAULT_MAX_TURNS,
                    Some(&state.workspace),
                    vector,
                    Some(assistant_id),
                )
            })
        })
    };

    let components = state.components.read().await.clone();
    let allowed = resolve_allowed_tools_for_scope(&state, assistant_id, &scope).await;
    let reply = process_message(components.as_ref(), &mut context, message, Some(&allowed))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(key, context.clone());
        save_session_to_disk(
            &state.sessions_dir,
            &state.workspace,
            &session_id,
            assistant_id,
            &context,
            Some(&scope),
        );
    }

    Ok(Json(ChatResponse { reply, session_id }))
}

/// 群聊流式：多助手串行回复，共享群历史，各自长期记忆
async fn api_chat_stream_group(
    state: Arc<AppState>,
    group_id: String,
    message: String,
) -> Result<Response, (StatusCode, String)> {
    let group = {
        let groups = state.groups.read().await;
        groups
            .get(&group_id)
            .cloned()
            .ok_or_else(|| (StatusCode::NOT_FOUND, "group not found".to_string()))?
    };
    let member_ids = group.member_ids.clone();
    if member_ids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "group has no members".to_string()));
    }

    let mut group_msgs = load_group_session(&state.sessions_dir, &group_id);
    group_msgs.push(GroupChatMessage {
        role: "user".to_string(),
        content: message.clone(),
        assistant_id: None,
    });
    let preview: String =
        message.chars().take(80).collect::<String>() + if message.len() > 80 { "…" } else { "" };
    emit_event(
        &state.event_bus,
        WorkspaceEvent::MessageCreated {
            group_id: group_id.clone(),
            from: None,
            to: None,
            content_preview: preview,
        },
    );
    let mut llm_history =
        group_messages_to_llm_messages(&group_msgs[..group_msgs.len() - 1], &state.assistants);

    let (line_tx, line_rx) = mpsc::unbounded_channel::<String>();
    let components = state.components.read().await.clone();
    let state_spawn = Arc::clone(&state);
    let group_id_spawn = group_id.clone();
    tokio::spawn(async move {
        let _ = line_tx.send(format!(
            "{}\n",
            serde_json::to_string(&serde_json::json!({
                "type": "session_id",
                "session_id": group_id_spawn
            }))
            .unwrap()
        ));

        for assistant_id in &member_ids {
            let group_scope = WebSessionScope {
                tenant_id: Some("tenant-default".to_string()),
                organization_id: Some("org-default".to_string()),
                team_id: None,
                agent_instance_id: Some(assistant_id.clone()),
                user_id: Some(group_id_spawn.clone()),
            };
            audit_knowledge_access(&state_spawn, assistant_id, &group_scope, &group_id_spawn);
            let _ = line_tx.send(format!(
                "{}\n",
                serde_json::to_string(&serde_json::json!({
                    "type": "group_assistant_start",
                    "assistant_id": assistant_id
                }))
                .unwrap()
            ));

            let vector = get_or_create_vector_for_assistant(&state_spawn, assistant_id).await;
            let mut context = create_context_with_long_term_for_assistant(
                &state_spawn.config,
                DEFAULT_MAX_TURNS,
                Some(&state_spawn.workspace),
                vector,
                Some(assistant_id),
            );
            context.set_messages(llm_history.clone());

            let system_prompt_override = state_spawn
                .assistant_prompts
                .read()
                .await
                .get(assistant_id)
                .cloned();
            let allowed_for_spawn = state_spawn
                .assistant_skills
                .read()
                .await
                .get(assistant_id)
                .cloned();
            let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ReactEvent>();
            let line_tx_fwd = line_tx.clone();
            let event_bus_fwd = state_spawn.event_bus.clone();
            let state_fwd = Arc::clone(&state_spawn);
            let group_id_fwd = group_id_spawn.clone();
            let assistant_id_fwd = assistant_id.clone();
            let forward_handle = tokio::spawn(async move {
                while let Some(ev) = event_rx.recv().await {
                    match &ev {
                        ReactEvent::Observation { tool, preview } => {
                            if tool == "create" {
                                if let Some(agent) = parse_create_observation(preview) {
                                    emit_event(
                                        &event_bus_fwd,
                                        WorkspaceEvent::AgentCreated {
                                            id: agent.id,
                                            role: agent.role,
                                            parent_id: agent.parent_id,
                                        },
                                    );
                                }
                            }
                        }
                        ReactEvent::ToolFailure { tool, reason } => {
                            audit_tool_failure(
                                &state_fwd,
                                &WebSessionScope {
                                    tenant_id: Some("tenant-default".to_string()),
                                    organization_id: Some("org-default".to_string()),
                                    team_id: None,
                                    agent_instance_id: Some(assistant_id_fwd.clone()),
                                    user_id: Some(group_id_fwd.clone()),
                                },
                                &group_id_fwd,
                                tool,
                                reason,
                            );
                        }
                        _ => {}
                    }
                    let _ = line_tx_fwd.send(format!("{}\n", serde_json::to_string(&ev).unwrap()));
                }
            });

            let prompt_ref = system_prompt_override.as_deref();
            let planner_override: Option<Arc<Planner>> = None;
            let allowed = allowed_for_spawn.as_deref();
            let reply = process_message_stream(
                components.as_ref(),
                &mut context,
                &message,
                event_tx,
                prompt_ref,
                planner_override.as_deref(),
                allowed,
                Some(assistant_id.as_str()),
            )
            .await
            .unwrap_or_else(|e| format!("Error: {}", e));

            let _ = forward_handle.await;
            let _ = line_tx.send(format!(
                "{}\n",
                serde_json::to_string(&serde_json::json!({
                    "type": "group_assistant_done",
                    "assistant_id": assistant_id
                }))
                .unwrap()
            ));

            group_msgs.push(GroupChatMessage {
                role: "assistant".to_string(),
                content: reply.clone(),
                assistant_id: Some(assistant_id.clone()),
            });
            let preview: String = reply.chars().take(80).collect::<String>()
                + if reply.len() > 80 { "…" } else { "" };
            emit_event(
                &state_spawn.event_bus,
                WorkspaceEvent::MessageCreated {
                    group_id: group_id_spawn.clone(),
                    from: Some(assistant_id.clone()),
                    to: None,
                    content_preview: preview,
                },
            );
            llm_history = group_messages_to_llm_messages(&group_msgs, &state_spawn.assistants);
        }

        save_group_session(
            &state_spawn.sessions_dir,
            &group_id_spawn,
            &group_msgs,
            DEFAULT_MAX_TURNS,
        );
    });

    type BoxErr = Box<dyn std::error::Error + Send + Sync>;
    let stream = stream::unfold(line_rx, |mut rx| async move {
        rx.recv()
            .await
            .map(|line| (Ok::<Bytes, BoxErr>(Bytes::from(line)), rx))
    });
    let mut res = Response::new(Body::from_stream(stream));
    res.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        "application/x-ndjson; charset=utf-8".parse().unwrap(),
    );
    Ok(res)
}

/// 流式聊天：NDJSON 流，首行 session_id，后续为 ReactEvent；group_id 时走群聊模式
async fn api_chat_stream(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Response, (StatusCode, String)> {
    let message = req.message.trim().to_string();
    if message.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is required".to_string()));
    }

    if let Some(ref gid) = req.group_id.filter(|s| !s.is_empty()) {
        return api_chat_stream_group(Arc::clone(&state), gid.clone(), message).await;
    }

    dynamic_agent_catalog::reload_dynamic_agents_into_state(&state).await;

    let session_id = req
        .session_id
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let model_id = req.model_id.as_deref().unwrap_or("default").to_string();
    let mut assistant_id = req.assistant_id.as_deref().unwrap_or("default").to_string();
    let mut dispatched_name: Option<String> = None;
    if assistant_id == "auto" {
        match dispatch_assistant(&state, &message).await {
            Ok(id) => {
                assistant_id = id.clone();
                dispatched_name = state
                    .assistants
                    .iter()
                    .find(|a| a.id == id)
                    .map(|a| a.name.clone());
            }
            Err(e) => {
                tracing::warn!("Auto dispatch failed: {}, using default", e);
                assistant_id = "default".to_string();
            }
        }
    }
    let system_prompt_override = state
        .assistant_prompts
        .read()
        .await
        .get(&assistant_id)
        .cloned();

    let scope = req.scope.to_scope(&session_id, &assistant_id);
    audit_knowledge_access(&state, &assistant_id, &scope, &session_id);
    let key = session_key(&session_id, &assistant_id, Some(&scope));
    let vector = get_or_create_vector_for_assistant(&state, &assistant_id).await;
    let context = {
        let mut sessions = state.sessions.write().await;
        sessions.remove(&key).unwrap_or_else(|| {
            load_session_from_disk(
                &state.sessions_dir,
                &session_id,
                &assistant_id,
                &state.workspace,
                &state.config,
                vector.clone(),
                Some(&scope),
            )
            .unwrap_or_else(|| {
                create_context_with_long_term_for_assistant(
                    &state.config,
                    DEFAULT_MAX_TURNS,
                    Some(&state.workspace),
                    vector,
                    Some(&assistant_id),
                )
            })
        })
    };

    let (event_tx, event_rx) = mpsc::unbounded_channel::<ReactEvent>();
    let (context_tx, context_rx) = tokio::sync::oneshot::channel();

    let components = state.components.read().await.clone();
    let resolved_allowed_tools =
        resolve_allowed_tools_for_scope(&state, &assistant_id, &scope).await;
    let allowed_for_spawn = refine_allowed_tools_for_input(
        &message,
        &components
            .executor
            .tool_metadata_for_names(&resolved_allowed_tools),
    )
    .allowed_tools;
    let session_id_clone = session_id.clone();
    let assistant_id_clone = assistant_id.clone();
    let session_key_clone = key.clone();
    let scope_for_spawn = scope.clone();
    let scope_for_stream = scope.clone();
    let state_spawn = Arc::clone(&state);
    let model_configs = state.model_configs.clone();
    let cancel_token = CancellationToken::new();
    {
        let mut active = state.active_cancellations.write().await;
        if let Some(existing) = active.insert(session_key_clone.clone(), cancel_token.clone()) {
            existing.cancel();
        }
    }
    tokio::spawn(async move {
        let mut ctx = context;
        let prompt_ref = system_prompt_override.as_deref();
        let planner_override: Option<Arc<Planner>> = if model_id != "default" {
            model_configs.get(&model_id).map(|entry| {
                let llm = create_llm_for_model(entry);
                let sys = prompt_ref
                    .unwrap_or_else(|| components.planner.base_system_prompt())
                    .to_string();
                Arc::new(Planner::new(llm, sys))
            })
        } else {
            None
        };
        let planner_ref = planner_override.as_deref();
        let allowed = Some(allowed_for_spawn.as_slice());
        let _ = process_message_stream_with_cancel(
            components.as_ref(),
            &mut ctx,
            &message,
            event_tx,
            prompt_ref,
            planner_ref,
            allowed,
            Some(assistant_id_clone.as_str()),
            cancel_token,
        )
        .await;
        // 无论流是否被客户端断开（超时/刷新），都持久化当前会话（含用户刚发的提问），刷新后历史不丢
        save_session_to_disk(
            &state_spawn.sessions_dir,
            &state_spawn.workspace,
            &session_id_clone,
            &assistant_id_clone,
            &ctx,
            Some(&scope_for_spawn),
        );
        let mut sessions = state_spawn.sessions.write().await;
        sessions.insert(session_key_clone.clone(), ctx);
        state_spawn
            .active_cancellations
            .write()
            .await
            .remove(&session_key_clone);
        let _ = context_tx.send(());
    });

    let mut first_line = format!(
        "{}\n",
        serde_json::to_string(&serde_json::json!({
            "type": "session_id",
            "session_id": session_id
        }))
        .unwrap()
    );
    if let Some(ref name) = dispatched_name {
        first_line.push_str(&format!(
            "{}\n",
            serde_json::to_string(&serde_json::json!({
                "type": "assistant_dispatched",
                "assistant_id": assistant_id,
                "assistant_name": name
            }))
            .unwrap()
        ));
    }

    let state_reinsert = Arc::clone(&state);
    let session_id_reinsert = session_id.clone();
    let stream = stream::try_unfold(
        (
            state_reinsert,
            session_id_reinsert,
            context_rx,
            event_rx,
            Some(first_line),
        ),
        move |(state_reinsert, session_id_reinsert, context_rx, mut event_rx, first_line_opt)| {
            let scope_for_stream = scope_for_stream.clone();
            async move {
                if let Some(line) = first_line_opt {
                    return Ok(Some((
                        Bytes::from(line),
                        (
                            state_reinsert,
                            session_id_reinsert,
                            context_rx,
                            event_rx,
                            None,
                        ),
                    )));
                }
                match event_rx.recv().await {
                    Some(ev) => {
                        // 自我改进：工具失败 → ERRORS.md；Critic 纠正 → LEARNINGS.md (correction)
                        match &ev {
                            ReactEvent::ToolFailure { tool, reason } => {
                                learnings_record_error(&state_reinsert.workspace, tool, reason);
                                audit_tool_failure(
                                    &state_reinsert,
                                    &scope_for_stream,
                                    &session_id_reinsert,
                                    tool,
                                    reason,
                                );
                            }
                            ReactEvent::Recovery { action, detail } if action == "Critic" => {
                                learnings_record_learning(
                                    &state_reinsert.workspace,
                                    "correction",
                                    detail,
                                    None,
                                );
                            }
                            ReactEvent::Observation { tool, preview } if tool == "create" => {
                                if let Some(agent) = parse_create_observation(preview) {
                                    emit_event(
                                        &state_reinsert.event_bus,
                                        WorkspaceEvent::AgentCreated {
                                            id: agent.id,
                                            role: agent.role,
                                            parent_id: agent.parent_id,
                                        },
                                    );
                                }
                            }
                            _ => {}
                        }
                        let line = format!("{}\n", serde_json::to_string(&ev).unwrap());
                        Ok(Some((
                            Bytes::from(line),
                            (
                                state_reinsert,
                                session_id_reinsert,
                                context_rx,
                                event_rx,
                                None,
                            ),
                        )))
                    }
                    None => {
                        let _ = context_rx.await;
                        Ok(None)
                    }
                }
            }
        },
    );

    type BoxErr = Box<dyn std::error::Error + Send + Sync>;
    let stream = stream.map_err(|e: tokio::sync::oneshot::error::RecvError| Box::new(e) as BoxErr);

    let mut res = Response::new(Body::from_stream(stream));
    res.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        "application/x-ndjson; charset=utf-8".parse().unwrap(),
    );
    Ok(res)
}

/// GET /api/events：SSE 流，推送 group.created / message.created
async fn api_events_sse(
    State(state): State<Arc<AppState>>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.event_bus.subscribe();
    let event_stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(msg) => return Some((Ok(Event::default().data(msg)), rx)),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keepalive"),
    )
}

/// GET /swarm：蜂群拓扑 Graph 页
async fn serve_swarm_page() -> Html<&'static str> {
    Html(include_str!("../../../static/swarm.html"))
}

/// GET /tasks：任务看板页
async fn serve_tasks_page() -> Html<&'static str> {
    Html(include_str!("../../../static/tasks.html"))
}

async fn serve_traces_page() -> Html<&'static str> {
    Html(include_str!("../../../static/traces.html"))
}

/// GET /api/traces/recent：获取最近的追踪列表
async fn api_traces_recent(Query(params): Query<TracesRecentParams>) -> Json<serde_json::Value> {
    use bee::observability::TraceCollector;

    let limit = params.limit.unwrap_or(50);

    // 尝试从全局获取 TraceCollector
    match TraceCollector::get_global() {
        Some(collector) => {
            let summaries = collector.get_recent_summaries(limit).await;
            Json(serde_json::json!({
                "traces": summaries
            }))
        }
        None => {
            // 返回空列表
            Json(serde_json::json!({
                "traces": []
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
struct TracesRecentParams {
    limit: Option<usize>,
}

/// GET /api/traces/{request_id}: 获取单个追踪详情
async fn api_traces_get(
    Path(request_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use bee::observability::TraceCollector;

    match TraceCollector::get_global() {
        Some(collector) => {
            match collector.get_by_request_id(&request_id).await {
                Some(trace) => {
                    // 转换为 JSON 格式
                    let json = serde_json::json!({
                        "request_id": trace.request_id,
                        "session_id": trace.session_id,
                        "status": format!("{:?}", trace.status).to_lowercase(),
                        "start_timestamp_ms": trace.start_timestamp_ms,
                        "end_timestamp_ms": trace.end_timestamp_ms,
                        "duration_ms": trace.duration_ms,
                        "input_summary": trace.input_summary,
                        "output_summary": trace.output_summary,
                        "error_message": trace.error_message,
                        "react_steps_total": trace.react_steps_total,
                        "llm_calls_count": trace.llm_calls_count,
                        "tool_executions_count": trace.tool_executions_count,
                        "total_tokens": trace.total_tokens,
                        "spans": trace.spans.iter().map(|span| {
                            serde_json::json!({
                                "span_id": span.span_id,
                                "parent_span_id": span.parent_span_id,
                                "operation_kind": span.operation_kind.as_str(),
                                "operation_name": span.operation_name,
                                "start_timestamp_ms": span.start_timestamp_ms,
                                "duration_ms": span.duration_ms,
                                "status": format!("{:?}", span.status).to_lowercase(),
                                "attributes": span.attributes,
                                "error_message": span.error_message,
                                "react_step": span.react_step,
                            })
                        }).collect::<Vec<_>>()
                    });
                    Ok(Json(json))
                }
                None => Err(StatusCode::NOT_FOUND),
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// GET /api/metrics：返回 JSON 格式的 metrics
async fn api_metrics() -> Json<serde_json::Value> {
    let metrics = bee::observability::Metrics::global();
    Json(metrics.to_json())
}

/// GET /api/metrics/prometheus：返回 Prometheus 格式的 metrics
async fn api_metrics_prometheus() -> (axum::http::StatusCode, String) {
    let metrics = bee::observability::Metrics::global();
    (axum::http::StatusCode::OK, metrics.to_prometheus())
}
