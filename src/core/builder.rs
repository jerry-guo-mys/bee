//! Agent 构建器：统一的 Agent 初始化逻辑
//!
//! 解决问题 1.1：消除 TUI 与 Headless 的工具注册差异

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::core::{RecoveryEngine, TaskScheduler};
use crate::llm::LlmClient;
use crate::react::{Critic, Planner};
use crate::skills::{SkillCache, SkillLoader};
#[cfg(feature = "browser")]
use crate::tools::BrowserTool;
use crate::tools::{
    CatTool, CodeEditTool, CodeGrepTool, CodeReadTool, CodeWriteTool, DeepSearchTool, EchoTool,
    ExchangeRateTool, GitCommitTool, GitHubRepoInspectTool, KnowledgeGraphBuilder, LsTool,
    MarketQuoteTool, NewsTool, PluginTool, ReportGeneratorTool, SearchTool, ShellTool,
    SourceValidatorTool, SportsScoreTool, TestCheckTool, TestRunTool, ToolExecutor, ToolRegistry,
    WeatherTool,
};
#[cfg(feature = "web")]
use crate::tools::{CreateGroupTool, CreateTool, ListAgentsTool, SendTool};

/// Agent 构建器：统一配置和初始化 Agent 的各个组件
pub struct AgentBuilder {
    config: AppConfig,
    workspace: PathBuf,
    system_prompt: String,
    enable_critic: bool,
    enable_skills: bool,
}

impl AgentBuilder {
    /// 创建新的构建器
    pub fn new(config: AppConfig, workspace: PathBuf) -> Self {
        Self {
            config,
            workspace,
            system_prompt: String::new(),
            enable_critic: true,
            enable_skills: true,
        }
    }

    /// 设置系统提示词
    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = prompt.to_string();
        self
    }

    /// 从文件加载系统提示词
    pub fn with_system_prompt_from_file(mut self) -> Self {
        // 尝试多个路径：从 workspace 根目录和当前目录
        let prompt_paths = [
            self.workspace.join("config/prompts/system.md"),
            self.workspace.join("../config/prompts/system.md"),
            self.workspace.join("config/prompts/default.md"),
            self.workspace.join("../config/prompts/default.md"),
        ];

        self.system_prompt = prompt_paths
            .iter()
            .find_map(|p| {
                std::fs::read_to_string(p)
                    .inspect_err(|e| tracing::debug!("Failed to read system prompt from {}: {}", p.display(), e))
                    .ok()
            })
            .unwrap_or_else(|| {
                tracing::warn!("No prompt file found, using built-in default");
                "You are Bee, a helpful AI assistant with access to various tools. If the user asks for an open-source address, repository link, GitHub URL, download page, homepage, or similar locator-style information, answer directly when possible or ask a short clarification question; do not call a tool first unless a tool is truly necessary to discover or verify the link. If the user asks a direct explanation question such as what a product or service is, what it does, or what its core functions are, answer directly from the conversation and available context unless the user explicitly asks for verification or browsing. For time-sensitive requests, prefer specialized fresh tools: weather for forecasts, news for headlines, exchange_rate for FX, market_quote for stock/index/crypto prices, and sports_score for live scores. For external GitHub repository architecture or stack questions, prefer github_repo_inspect, and after it returns structured fields like repo_summary, detected_stack, top_level_directories, key_files_found, or file_snippets, answer directly instead of inspecting the local workspace.".to_string()
            });
        self
    }

    /// 是否启用 Critic
    pub fn with_critic(mut self, enable: bool) -> Self {
        self.enable_critic = enable;
        self
    }

    /// 是否启用技能系统
    pub fn with_skills(mut self, enable: bool) -> Self {
        self.enable_skills = enable;
        self
    }

    /// 构建统一的工具注册表（所有接入方式共享同一套工具）
    ///
    /// 需要传入共享的 LLM 客户端供深度研究等工具使用
    pub fn build_tool_registry(&self, llm: Arc<dyn LlmClient>) -> ToolRegistry {
        let mut tools = ToolRegistry::new();

        tools.register(CatTool::new(&self.workspace));
        tools.register(LsTool::new(&self.workspace));
        tools.register(EchoTool);
        tools.register(ShellTool::new(
            self.config.tools.shell.allowed_commands.clone(),
            self.config.tools.tool_timeout_secs,
        ));
        tools.register(SearchTool::new(
            self.config.tools.search.allowed_domains.clone(),
            self.config.tools.search.timeout_secs,
            self.config.tools.search.max_result_chars,
        ));
        tools.register(WeatherTool::new(self.config.tools.search.timeout_secs));
        tools.register(NewsTool::new(self.config.tools.search.timeout_secs));
        tools.register(ExchangeRateTool::new(self.config.tools.search.timeout_secs));
        tools.register(MarketQuoteTool::new(self.config.tools.search.timeout_secs));
        tools.register(SportsScoreTool::new(self.config.tools.search.timeout_secs));
        tools.register(GitHubRepoInspectTool::new(
            self.config.tools.search.max_result_chars,
        ));

        #[cfg(feature = "browser")]
        tools.register(BrowserTool::new(
            self.config.tools.search.allowed_domains.clone(),
            self.config.tools.search.max_result_chars,
        ));

        for entry in &self.config.tools.plugins {
            tools.register(PluginTool::new(
                entry,
                &self.workspace,
                self.config.tools.tool_timeout_secs,
            ));
        }

        tools.register(CodeReadTool::new(&self.workspace));
        tools.register(CodeGrepTool::new(&self.workspace));
        tools.register(CodeEditTool::new(&self.workspace));
        tools.register(CodeWriteTool::new(&self.workspace));
        tools.register(TestRunTool::new(&self.workspace));
        tools.register(TestCheckTool::new(&self.workspace));
        tools.register(GitCommitTool::new(&self.workspace));
        tools.register(DeepSearchTool::new(
            llm.clone(),
            self.config.tools.deep_research.max_rounds,
            self.config.tools.deep_research.max_results_per_round,
            self.config.tools.deep_research.timeout_secs,
            self.config.tools.deep_research.trusted_domains.clone(),
        ));
        tools.register(SourceValidatorTool::new(
            self.config.tools.search.allowed_domains.clone(),
        ));
        tools.register(ReportGeneratorTool::new(llm.clone()));
        tools.register(KnowledgeGraphBuilder::new(llm));

        #[cfg(feature = "web")]
        tools.register(CreateTool::new(&self.workspace));
        #[cfg(feature = "web")]
        tools.register(CreateGroupTool::new(&self.workspace));
        #[cfg(feature = "web")]
        tools.register(ListAgentsTool::new(&self.workspace));
        #[cfg(feature = "web")]
        tools.register(SendTool::new(&self.workspace));

        tools
    }

    /// 构建 LLM 客户端
    pub fn build_llm(&self) -> Arc<dyn LlmClient> {
        crate::application::orchestrator::create_llm_from_config(&self.config)
    }

    /// 构建 Critic（可选，解决问题 4.3：配置化与模型分离）
    pub fn build_critic(&self, planner_llm: Arc<dyn LlmClient>) -> Option<Critic> {
        // enable_critic 为 false 时不创建
        if !self.enable_critic {
            return None;
        }
        // 检查配置是否启用 Critic
        if !self.config.critic.enabled {
            return None;
        }

        // 如果配置了独立的 Critic 模型，使用独立的 LLM 实例
        let critic_llm: Arc<dyn LlmClient> = if let Some(ref model) = self.config.critic.model {
            let provider = self
                .config
                .critic
                .provider
                .as_deref()
                .unwrap_or(&self.config.llm.provider);

            if provider.to_lowercase() == "deepseek" {
                Arc::new(crate::llm::create_deepseek_client(Some(model)))
            } else {
                let base_url = self.config.llm.base_url.as_deref();
                let api_key = std::env::var("OPENAI_API_KEY").ok();
                Arc::new(crate::llm::OpenAiClient::new(
                    base_url,
                    model,
                    api_key.as_deref(),
                ))
            }
        } else {
            planner_llm
        };

        // 尝试从文件加载 prompt，否则使用配置中的模板
        let critic_paths = [
            self.workspace.join("config/prompts/critic.md"),
            self.workspace.join("../config/prompts/critic.md"),
        ];
        let critic_prompt = critic_paths
            .iter()
            .find_map(|p| {
                std::fs::read_to_string(p)
                    .inspect_err(|e| tracing::debug!("Failed to read critic prompt from {}: {}", p.display(), e))
                    .ok()
            })
            .unwrap_or_else(|| self.config.critic.prompt_template.clone());

        // 创建修改后的配置副本，使用文件中的 prompt
        let mut critic_config = self.config.critic.clone();
        critic_config.prompt_template = critic_prompt;

        Some(Critic::from_config(critic_llm, &critic_config))
    }

    /// 构建技能加载器（返回 Arc 可共享）
    ///
    /// 注意：技能加载在调用时同步完成，确保返回的 SkillLoader 已就绪
    pub fn build_skill_loader(&self) -> Arc<SkillLoader> {
        let skill_loader = Arc::new(SkillLoader::from_default());

        if self.enable_skills {
            let loader = skill_loader.clone();
            // 检查是否在 runtime 上下文中
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    // 在 runtime 内：使用 spawn_blocking 避免阻塞 async 任务
                    let future = async move {
                        if let Err(e) = loader.load_all().await {
                            tracing::warn!("Failed to load skills: {}", e);
                        }
                    };
                    // 如果当前是 multi-thread runtime，使用 spawn 并阻塞等待
                    if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread {
                        tokio::task::block_in_place(|| {
                            handle.block_on(future);
                        });
                    } else {
                        // 单线程 runtime (current_thread): 直接 block_on
                        handle.block_on(future);
                    }
                }
                Err(_) => {
                    // 无 runtime: 创建新 runtime
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Err(e) = loader.load_all().await {
                            tracing::warn!("Failed to load skills: {}", e);
                        }
                    });
                }
            }
        }

        skill_loader
    }

    /// 构建完整系统提示词（包含工具 schema）
    pub fn build_full_system_prompt(&self, tool_registry: &ToolRegistry) -> String {
        let has_browser = tool_registry.get("browser").is_some();
        let system_prompt = if has_browser {
            self.system_prompt.clone()
        } else {
            self.system_prompt
                .lines()
                .filter(|line| !line.contains("`browser`") && !line.contains("- browser:"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let tool_schema = tool_registry.to_schema_json();
        if tool_schema.is_empty() || tool_schema == "[]" {
            system_prompt
        } else {
            format!(
                "{}\n\n## Tool call JSON Schema (you must output valid JSON matching this)\n```json\n{}\n```",
                system_prompt, tool_schema
            )
        }
    }

    /// 构建完整的 AgentComponents（供 Headless/Web/WhatsApp/Gateway 使用）
    pub fn build_components(&self) -> AgentComponents {
        let llm = self.build_llm();
        let critic = self.build_critic(llm.clone());
        let tools = self.build_tool_registry(llm.clone());
        let full_system_prompt = self.build_full_system_prompt(&tools);
        let skill_loader = self.build_skill_loader();

        AgentComponents {
            planner: Planner::new(llm.clone(), full_system_prompt),
            executor: ToolExecutor::new(tools, self.config.tools.tool_timeout_secs),
            recovery: RecoveryEngine::new(),
            critic,
            task_scheduler: TaskScheduler::default(),
            skill_loader,
            llm,
            config: self.config.clone(),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// 获取工作目录
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
}

/// 预构建的 Agent 组件：Planner、ToolExecutor、Recovery、Critic、TaskScheduler
/// 可多会话共享
pub struct AgentComponents {
    pub planner: Planner,
    pub executor: ToolExecutor,
    pub recovery: RecoveryEngine,
    pub critic: Option<Critic>,
    pub task_scheduler: TaskScheduler,
    pub skill_loader: Arc<SkillLoader>,
    pub llm: Arc<dyn LlmClient>,
    pub config: AppConfig,
}

impl AgentComponents {
    /// 获取技能缓存引用
    pub fn skill_cache(&self) -> SkillCache {
        self.skill_loader.cache()
    }

    /// 获取 LLM 客户端引用
    pub fn llm(&self) -> &Arc<dyn LlmClient> {
        &self.llm
    }

    /// 获取配置引用
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
}

/// 便捷函数：从默认路径创建 AgentBuilder
pub fn create_agent_builder(config_path: Option<PathBuf>) -> AgentBuilder {
    let config = crate::config::load_config(config_path).unwrap_or_else(|e| {
        tracing::warn!("Config load failed ({}), using defaults", e);
        AppConfig::default()
    });

    let workspace = config
        .app
        .workspace_root
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap().join("workspace"));
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.clone());
    std::fs::create_dir_all(&workspace).ok();

    AgentBuilder::new(config, workspace).with_system_prompt_from_file()
}
