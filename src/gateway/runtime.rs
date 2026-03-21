//! Agent Runtime（代理运行时）
//!
//! 实际的 AI 处理逻辑，与 Gateway 解耦

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;

use super::message::{GatewayMessage, MessageType, SessionStatus};
use super::session::SessionScope;
use super::session_store::SessionStore;
use crate::agent::{create_agent_components, create_context_with_long_term_for_assistant};
use crate::config::AppConfig;
use crate::core::{AgentComponents, AgentError};
use crate::react::{react_loop, ReactEvent};
use crate::saas::{
    default_low_risk_tools, resolve_effective_tool_allowlist, SaasSqliteStore, ToolPolicyScope,
};
use crate::skills::SkillSelector;
use crate::tool_policy::refine_allowed_tools_for_input;
use crate::tool_router::{deterministic_route, execute_direct_route};

fn allowed_tools_hint(allowed_tools: &[String]) -> Option<String> {
    if allowed_tools.is_empty() {
        None
    } else {
        Some(format!(
            "For this conversation, you may use only these tools: {}. Do not call any other tool.",
            allowed_tools.join(", ")
        ))
    }
}

/// Runtime 配置
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// 应用配置
    pub app_config: AppConfig,
    /// 工作目录
    pub workspace: PathBuf,
    /// 系统提示词
    pub system_prompt: String,
    /// 最大并发请求数
    pub max_concurrent: usize,
    /// 启用技能选择
    pub enable_skills: bool,
    /// 会话数据库路径（None 表示使用内存存储）
    pub session_db_path: Option<PathBuf>,
    /// 任务队列数据库路径（None 表示使用内存存储）
    pub task_db_path: Option<PathBuf>,
    /// 用户记忆快照目录
    pub user_memory_dir: Option<PathBuf>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            app_config: AppConfig::default(),
            workspace: PathBuf::from("."),
            system_prompt: "You are a helpful AI assistant.".to_string(),
            max_concurrent: 10,
            enable_skills: true,
            session_db_path: None,
            task_db_path: None,
            user_memory_dir: None,
        }
    }
}

/// Agent Runtime - AI 处理核心
pub struct AgentRuntime {
    config: RuntimeConfig,
    components: AgentComponents,
    session_store: Arc<dyn SessionStore>,
}

impl AgentRuntime {
    pub fn new(config: RuntimeConfig, session_store: Arc<dyn SessionStore>) -> Self {
        let components = create_agent_components(&config.app_config, &config.workspace);
        Self {
            config,
            components,
            session_store,
        }
    }

    /// 获取 Agent 组件（用于共享 LLM 等）
    pub fn components(&self) -> &AgentComponents {
        &self.components
    }

    /// 获取会话存储
    pub fn session_store(&self) -> &Arc<dyn SessionStore> {
        &self.session_store
    }

    /// 处理用户消息
    pub async fn process_message(
        &self,
        session_id: &str,
        user_input: &str,
        assistant_id: Option<&str>,
        model: Option<&str>,
        response_tx: mpsc::UnboundedSender<GatewayMessage>,
    ) -> Result<String, AgentError> {
        let request_id = uuid::Uuid::new_v4().to_string();

        self.session_store
            .set_status(session_id, SessionStatus::Processing)
            .await;

        response_tx
            .send(GatewayMessage::new(
                Some(session_id.to_string()),
                MessageType::ResponseStart {
                    request_id: request_id.clone(),
                },
            ))
            .ok();

        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ReactEvent>();

        let response_tx_clone = response_tx.clone();
        let request_id_clone = request_id.clone();
        let session_id_owned = session_id.to_string();

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let msg = match event {
                    ReactEvent::Thinking => continue,
                    ReactEvent::ThinkingContent { text } => GatewayMessage::new(
                        Some(session_id_owned.clone()),
                        MessageType::Thinking {
                            request_id: request_id_clone.clone(),
                            content: text,
                        },
                    ),
                    ReactEvent::ToolCall { tool, args } => GatewayMessage::new(
                        Some(session_id_owned.clone()),
                        MessageType::ToolCall {
                            request_id: request_id_clone.clone(),
                            tool_name: tool,
                            arguments: args,
                        },
                    ),
                    ReactEvent::Observation { tool, preview } => GatewayMessage::new(
                        Some(session_id_owned.clone()),
                        MessageType::ToolResult {
                            request_id: request_id_clone.clone(),
                            tool_name: tool,
                            result: preview,
                            success: true,
                        },
                    ),
                    ReactEvent::MessageChunk { text } => GatewayMessage::new(
                        Some(session_id_owned.clone()),
                        MessageType::ResponseChunk {
                            request_id: request_id_clone.clone(),
                            content: text,
                        },
                    ),
                    ReactEvent::MessageDone => continue,
                    ReactEvent::ToolFailure { tool, reason } => GatewayMessage::new(
                        Some(session_id_owned.clone()),
                        MessageType::ToolResult {
                            request_id: request_id_clone.clone(),
                            tool_name: tool,
                            result: reason,
                            success: false,
                        },
                    ),
                    ReactEvent::Error { text } => GatewayMessage::new(
                        Some(session_id_owned.clone()),
                        MessageType::Error {
                            request_id: Some(request_id_clone.clone()),
                            code: "react_error".to_string(),
                            message: text,
                        },
                    ),
                    _ => continue,
                };
                if response_tx_clone.send(msg).is_err() {
                    break;
                }
            }
        });

        let result = self
            .run_react_loop(session_id, user_input, event_tx, assistant_id, model)
            .await;

        self.session_store
            .set_status(session_id, SessionStatus::Idle)
            .await;

        match &result {
            Ok(response) => {
                response_tx
                    .send(GatewayMessage::new(
                        Some(session_id.to_string()),
                        MessageType::ResponseEnd {
                            request_id,
                            full_content: response.clone(),
                        },
                    ))
                    .ok();
            }
            Err(e) => {
                response_tx
                    .send(GatewayMessage::new(
                        Some(session_id.to_string()),
                        MessageType::Error {
                            request_id: Some(request_id),
                            code: "runtime_error".to_string(),
                            message: e.to_string(),
                        },
                    ))
                    .ok();
            }
        }

        result
    }

    async fn run_react_loop(
        &self,
        session_id: &str,
        user_input: &str,
        event_tx: mpsc::UnboundedSender<ReactEvent>,
        assistant_id: Option<&str>,
        _model: Option<&str>,
    ) -> Result<String, AgentError> {
        let cancel_token = self
            .session_store
            .new_cancel_token(session_id)
            .await
            .unwrap_or_else(tokio_util::sync::CancellationToken::new);
        let scope = self
            .session_store
            .get_scope(session_id)
            .await
            .unwrap_or_default();
        let allowed_tools =
            resolve_allowed_tools_for_scope(&self.components, &self.config.workspace, &scope);
        let allowed_tool_metadata = self
            .components
            .executor
            .tool_metadata_for_names(&allowed_tools);
        let policy_decision = refine_allowed_tools_for_input(user_input, &allowed_tool_metadata);
        let allowed_tools_hint = allowed_tools_hint(&policy_decision.allowed_tools);

        let mut context = self
            .session_store
            .get_context(session_id)
            .await
            .unwrap_or_else(|| {
                let scoped_workspace = scoped_runtime_workspace(&self.config.workspace, &scope);
                create_context_with_long_term_for_assistant(
                    &self.config.app_config,
                    self.config.app_config.app.max_context_turns,
                    Some(&scoped_workspace),
                    None,
                    assistant_id,
                )
            });

        let system_prompt = if self.config.enable_skills {
            let selector = SkillSelector::new(
                self.components.skill_cache(),
                Arc::clone(&self.components.llm),
            );
            let skills = selector.select(user_input).await;
            if skills.is_empty() {
                None
            } else {
                let skills_prompt = SkillSelector::build_skills_prompt(&skills);
                Some(format!(
                    "{}\n\n{}",
                    self.config.system_prompt, skills_prompt
                ))
            }
        } else {
            None
        };
        let system_prompt = match (
            system_prompt,
            policy_decision.system_hint.as_deref(),
            allowed_tools_hint.as_deref(),
        ) {
            (Some(base), Some(policy_hint), Some(allowed_hint)) => {
                Some(format!("{base}\n\n{policy_hint}\n{allowed_hint}"))
            }
            (Some(base), Some(policy_hint), None) => Some(format!("{base}\n\n{policy_hint}")),
            (Some(base), None, Some(allowed_hint)) => Some(format!("{base}\n\n{allowed_hint}")),
            (Some(base), None, None) => Some(base),
            (None, Some(policy_hint), Some(allowed_hint)) => {
                Some(format!("{policy_hint}\n{allowed_hint}"))
            }
            (None, Some(policy_hint), None) => Some(policy_hint.to_string()),
            (None, None, Some(allowed_hint)) => Some(allowed_hint.to_string()),
            (None, None, None) => None,
        };

        if let Some(route) =
            deterministic_route(user_input, Some(policy_decision.allowed_tools.as_slice()))
        {
            let result = execute_direct_route(
                &self.components.executor,
                &mut context,
                user_input,
                &route,
                Some(&event_tx),
                cancel_token,
            )
            .await;
            self.session_store.set_context(session_id, context).await;
            if let Ok(ref direct_result) = result {
                crate::observability::Metrics::global()
                    .tools
                    .record_direct_route_hit();
                for msg in &direct_result.messages {
                    self.session_store
                        .add_message(session_id, msg.clone())
                        .await;
                }
            }
            return result.map(|r| r.response);
        }

        let result = react_loop(
            &self.components.planner,
            &self.components.executor,
            &self.components.recovery,
            &mut context,
            user_input,
            None,
            Some(&event_tx),
            cancel_token,
            self.components.critic.as_ref(),
            Some(&self.components.task_scheduler),
            system_prompt.as_deref(),
            Some(policy_decision.allowed_tools.as_slice()),
        )
        .await;

        self.session_store.set_context(session_id, context).await;

        if let Ok(ref react_result) = result {
            for msg in &react_result.messages {
                self.session_store
                    .add_message(session_id, msg.clone())
                    .await;
            }
        }

        result.map(|r| r.response)
    }

    /// 取消正在进行的请求
    pub async fn cancel(&self, session_id: &str) {
        self.session_store.cancel(session_id).await;
    }

    /// 获取会话历史
    pub async fn get_history(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Vec<(String, String)> {
        self.session_store.get_history(session_id, limit).await
    }
}

fn scoped_runtime_workspace(base_workspace: &std::path::Path, scope: &SessionScope) -> PathBuf {
    let mut path = base_workspace.join(".bee").join("runtime_scopes");
    path.push(sanitize_scope_segment(
        scope.tenant_id.as_deref().unwrap_or("tenant-default"),
    ));
    path.push(sanitize_scope_segment(
        scope.organization_id.as_deref().unwrap_or("org-default"),
    ));
    if let Some(team_id) = scope.team_id.as_deref() {
        path.push("teams");
        path.push(sanitize_scope_segment(team_id));
    }
    if let Some(user_id) = scope.user_id.as_deref() {
        path.push("users");
        path.push(sanitize_scope_segment(user_id));
    }
    if let Some(agent_instance_id) = scope.agent_instance_id.as_deref() {
        path.push("agents");
        path.push(sanitize_scope_segment(agent_instance_id));
    }
    let _ = std::fs::create_dir_all(&path);
    path
}

fn sanitize_scope_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "default".to_string()
    } else {
        sanitized
    }
}

fn resolve_allowed_tools_for_scope(
    components: &AgentComponents,
    workspace: &std::path::Path,
    scope: &SessionScope,
) -> Vec<String> {
    let tools = components.executor.tool_names();
    let default_tools = if scope
        .team_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        tools
    } else {
        default_low_risk_tools(&tools)
    };
    let db_path = workspace.join(".bee").join("saas.db");
    if let Ok(store) = SaasSqliteStore::new(db_path) {
        if let Ok(resolved) = resolve_effective_tool_allowlist(
            &store,
            &ToolPolicyScope {
                tenant_id: scope
                    .tenant_id
                    .clone()
                    .unwrap_or_else(|| "tenant-default".to_string()),
                organization_id: scope
                    .organization_id
                    .clone()
                    .or_else(|| Some("org-default".to_string())),
                team_id: scope.team_id.clone(),
            },
            &default_tools,
        ) {
            return resolved;
        }
    }
    default_tools
}
