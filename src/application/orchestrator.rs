//! Agent 编排器：UI 与 Agent 服务之间的桥梁
//!
//! 负责：建立 UI 与 Agent 服务之间的命令/状态/流通道

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, watch, Mutex};

use super::agent_service::{AgentService, AgentServiceImpl};
use crate::config::AppConfig;
use crate::core::{AgentPhase, SessionSupervisor, UiState};
use crate::llm::{
    create_deepseek_client, LlmClient, ModelCapabilities, ModelRouter, OpenAiClient,
    RoutingLlmClient,
};
use crate::memory::SqlitePersistence;
use crate::react::ReactEvent;

/// 从 UI 发往编排器的用户命令
#[derive(Debug, Clone)]
pub enum Command {
    /// 提交用户输入，触发 ReAct 循环
    Submit(String),
    /// 取消当前生成（Stop generating）
    Cancel,
    /// 清空对话与 Working Memory
    Clear,
    /// 退出应用
    Quit,
}

/// 根据配置与环境变量选择 LLM 后端（DeepSeek / OpenAI 兼容 / Mock）
/// 使用模型路由器自动选择最合适的模型
pub fn create_llm_from_config(cfg: &AppConfig) -> Arc<dyn LlmClient> {
    let provider = cfg.llm.provider.to_lowercase();
    let use_deepseek = std::env::var("DEEPSEEK_API_KEY").is_ok()
        || (provider == "deepseek" && std::env::var("OPENAI_API_KEY").is_ok());
    let use_openai = std::env::var("OPENAI_API_KEY").is_ok() && provider != "deepseek";

    let mut router = ModelRouter::new();

    // 添加 DeepSeek 模型（如果有 API Key）
    if use_deepseek {
        let chat_model = cfg
            .llm
            .deepseek
            .model
            .clone()
            .or_else(|| Some(cfg.llm.model.clone()))
            .unwrap_or_else(|| "deepseek-chat".to_string());

        // 尝试添加 DeepSeek Chat（快速模型）
        tracing::info!("Using DeepSeek Chat LLM ({})", chat_model);
        let chat_client: Arc<dyn LlmClient> = Arc::new(create_deepseek_client(Some(&chat_model)));
        router.add_model(
            ModelCapabilities::new("deepseek-chat")
                .with_code(85)
                .with_reasoning(75)
                .with_speed(90)
                .with_cost(85),
            chat_client,
        );

        // 如果配置了 Reasoner 模型，也添加它（推理模型）
        if let Some(reasoner_model) = cfg.llm.deepseek.model.clone() {
            if reasoner_model != chat_model && reasoner_model.contains("reasoner") {
                tracing::info!("Using DeepSeek Reasoner LLM ({})", reasoner_model);
                let reasoner_client: Arc<dyn LlmClient> = Arc::new(create_deepseek_client(Some(&reasoner_model)));
                router.add_model(
                    ModelCapabilities::new("deepseek-reasoner")
                        .with_code(95)
                        .with_reasoning(98)
                        .with_speed(40)
                        .with_cost(30),
                    reasoner_client,
                );
            }
        }
    }

    // 添加 OpenAI 模型（如果有 API Key）
    if use_openai {
        let model = cfg
            .llm
            .openai
            .model
            .clone()
            .unwrap_or_else(|| "gpt-4o-mini".to_string());
        let base = cfg.llm.base_url.as_deref();
        tracing::info!("Using OpenAI LLM ({})", model);
        let openai_client: Arc<dyn LlmClient> = Arc::new(OpenAiClient::new(
            base,
            &model,
            std::env::var("OPENAI_API_KEY").ok().as_deref(),
        ));
        router.add_model(
            ModelCapabilities::new(&model)
                .with_code(80)
                .with_reasoning(85)
                .with_speed(75)
                .with_cost(70),
            openai_client,
        );
    }

    // 如果没有添加任何模型，使用 Mock
    if router.model_count() == 0 {
        tracing::warn!("No API key set or provider unknown, using Mock LLM");
        return Arc::new(crate::llm::MockLlmClient);
    }

    // 使用路由客户端包装路由器
    Arc::new(RoutingLlmClient::new(router))
}

/// 创建 Agent 运行时：返回命令发送端、状态接收端、流接收端
pub async fn create_agent(
    config_path: Option<PathBuf>,
) -> anyhow::Result<(
    mpsc::UnboundedSender<Command>,
    watch::Receiver<UiState>,
    broadcast::Receiver<String>,
    mpsc::UnboundedReceiver<ReactEvent>,
)> {
    use crate::core::create_agent_builder;

    // 使用统一的 AgentBuilder 构建所有组件
    let builder = create_agent_builder(config_path);
    let workspace = builder.workspace().to_path_buf();
    let cfg = builder.config().clone();

    // 构建核心组件并传递给 AgentService
    let components = builder.build_components();

    // 初始化 SQLite 持久化
    let sqlite_db_path = workspace.join(".bee/conversations.db");
    if let Some(parent) = sqlite_db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let sqlite_persistence = Arc::new(Mutex::new(SqlitePersistence::new(&sqlite_db_path).ok()));

    // 创建 Agent 服务
    let agent_service = Arc::new(AgentServiceImpl::new(
        cfg.clone(),
        components,
        sqlite_persistence.clone(),
    ));

    // 三通道：UI -> Core 命令；Core -> UI 状态快照；Core -> UI Token 流
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (state_tx, state_rx) = watch::channel(UiState::default());
    let (_stream_tx, stream_rx) = broadcast::channel::<String>(16);
    let (_event_tx, event_rx) = mpsc::unbounded_channel::<ReactEvent>();

    let supervisor = SessionSupervisor::new();
    let session_id = uuid::Uuid::new_v4().to_string();
    let session_id_clone = session_id.clone();
    let agent_service_clone = agent_service.clone();
    let mut conversation_history: Vec<crate::memory::Message> = Vec::new();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        Command::Submit(input) => {
                            let _cancel_token = supervisor.reset_cancel_token();

                            // 先添加用户消息到历史
                            let user_msg = crate::memory::Message {
                                role: crate::memory::Role::User,
                                content: input.clone(),
                            };
                            conversation_history.push(user_msg);

                            // 更新为 Thinking 状态（保留历史）
                            let history_clone = conversation_history.clone();
                            let _ = state_tx.send(UiState {
                                phase: AgentPhase::Thinking,
                                history: history_clone,
                                active_tool: None,
                                input_locked: true,
                                error_message: None,
                            });

                            // 调用 Agent 服务处理消息
                            let result = agent_service_clone.process_message(&session_id_clone, &input).await;

                            match result {
                                Ok(response) => {
                                    // 追加助手回复到历史
                                    conversation_history.extend(response.messages);
                                    let history_clone = conversation_history.clone();
                                    let _ = state_tx.send(UiState {
                                        phase: AgentPhase::Idle,
                                        history: history_clone,
                                        active_tool: None,
                                        input_locked: false,
                                        error_message: None,
                                    });
                                }
                                Err(e) => {
                                    let _ = state_tx.send(UiState {
                                        phase: AgentPhase::Error,
                                        history: conversation_history.clone(),
                                        active_tool: None,
                                        input_locked: false,
                                        error_message: Some(e.to_string()),
                                    });
                                }
                            }
                        }
                        Command::Cancel => {
                            supervisor.cancel();
                        }
                        Command::Clear => {
                            let _ = agent_service_clone.clear(&session_id_clone).await;
                            conversation_history.clear();
                            let _ = state_tx.send(UiState {
                                phase: AgentPhase::Idle,
                                history: vec![],
                                active_tool: None,
                                input_locked: false,
                                error_message: None,
                            });
                        }
                        Command::Quit => break,
                    }
                }
                else => break,
            }
        }
    });

    Ok((cmd_tx, state_rx, stream_rx, event_rx))
}
