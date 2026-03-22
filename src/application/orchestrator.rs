//! Agent 编排器：UI 与 Agent 服务之间的桥梁
//!
//! 负责：建立 UI 与 Agent 服务之间的命令/状态/流通道

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, watch, Mutex};

use super::agent_service::{AgentService, AgentServiceImpl};
use crate::config::AppConfig;
use crate::core::{AgentPhase, SessionSupervisor, UiState};
use crate::llm::{create_deepseek_client, LlmClient, OpenAiClient};
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
pub fn create_llm_from_config(cfg: &AppConfig) -> Arc<dyn LlmClient> {
    let provider = cfg.llm.provider.to_lowercase();
    let use_deepseek = std::env::var("DEEPSEEK_API_KEY").is_ok()
        || (provider == "deepseek" && std::env::var("OPENAI_API_KEY").is_ok());
    let use_openai = std::env::var("OPENAI_API_KEY").is_ok() && provider != "deepseek";

    if use_deepseek {
        let model = cfg
            .llm
            .deepseek
            .model
            .clone()
            .or_else(|| Some(cfg.llm.model.clone()))
            .unwrap_or_else(|| "deepseek-chat".to_string());
        tracing::info!("Using DeepSeek LLM ({})", model);
        Arc::new(create_deepseek_client(Some(&model)))
    } else if use_openai {
        let model = cfg
            .llm
            .openai
            .model
            .clone()
            .unwrap_or_else(|| "gpt-4o-mini".to_string());
        let base = cfg.llm.base_url.as_deref();
        tracing::info!("Using OpenAI LLM ({})", model);
        Arc::new(OpenAiClient::new(
            base,
            &model,
            std::env::var("OPENAI_API_KEY").ok().as_deref(),
        ))
    } else {
        tracing::warn!("No API key set or provider unknown, using Mock LLM");
        Arc::new(crate::llm::MockLlmClient)
    }
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

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        Command::Submit(input) => {
                            let _cancel_token = supervisor.reset_cancel_token();

                            // 更新为 Thinking 状态
                            let _ = state_tx.send(UiState {
                                phase: AgentPhase::Thinking,
                                history: vec![],
                                active_tool: None,
                                input_locked: true,
                                error_message: None,
                            });

                            // 调用 Agent 服务处理消息
                            let result = agent_service_clone.process_message(&session_id_clone, &input).await;

                            match result {
                                Ok(response) => {
                                    let _ = state_tx.send(UiState {
                                        phase: AgentPhase::Idle,
                                        history: response.messages,
                                        active_tool: None,
                                        input_locked: false,
                                        error_message: None,
                                    });
                                }
                                Err(e) => {
                                    let _ = state_tx.send(UiState {
                                        phase: AgentPhase::Error,
                                        history: vec![],
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
