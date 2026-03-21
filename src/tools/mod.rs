pub mod code_edit;
pub mod code_grep;
pub mod code_read;
pub mod code_review;
pub mod code_write;
pub mod deep_search;
pub mod echo;
pub mod exchange_rate;
pub mod executor;
pub mod filesystem;
pub mod git_commit;
pub mod git_diff;
pub mod github_repo_inspect;
pub mod knowledge_graph;
pub mod market_quote;
pub mod metadata;
pub mod news;
pub mod output;
pub mod plugin;
pub mod registry;
pub mod report_generator;
pub mod schema;
pub mod search;
pub mod shell;
pub mod source_validator;
pub mod sports_score;
pub mod test_check;
pub mod test_run;
pub mod weather;

#[cfg(feature = "web")]
pub mod create;
#[cfg(feature = "web")]
pub mod create_group;
#[cfg(feature = "web")]
pub mod list_agents;
#[cfg(feature = "web")]
pub mod send;

#[cfg(feature = "browser")]
pub mod browser;

pub use code_edit::CodeEditTool;
pub use code_grep::CodeGrepTool;
pub use code_read::CodeReadTool;
pub use code_review::CodeReviewTool;
pub use code_write::CodeWriteTool;
pub use deep_search::DeepSearchTool;
pub use echo::EchoTool;
pub use exchange_rate::ExchangeRateTool;
pub use executor::ToolExecutor;
pub use filesystem::{CatTool, LsTool, SafeFs};
pub use git_commit::GitCommitTool;
pub use git_diff::GitDiffTool;
pub use github_repo_inspect::GitHubRepoInspectTool;
pub use knowledge_graph::KnowledgeGraphBuilder;
pub use market_quote::MarketQuoteTool;
pub use metadata::{
    ToolCapabilityGroup, ToolCapabilitySubgroup, ToolCostClass, ToolCriticMode, ToolFreshness,
    ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk, ToolScope, ToolUseCase,
};
pub use news::NewsTool;
pub use plugin::PluginTool;
pub use registry::{Tool, ToolRegistry};
pub use report_generator::ReportGeneratorTool;
pub use schema::tool_call_schema_json;
pub use search::SearchTool;
pub use shell::ShellTool;
pub use source_validator::SourceValidatorTool;
pub use sports_score::SportsScoreTool;
pub use test_check::TestCheckTool;
pub use test_run::TestRunTool;
pub use weather::WeatherTool;

#[cfg(feature = "web")]
pub use create::{CreateTool, DynamicAgent};
#[cfg(feature = "web")]
pub use create_group::CreateGroupTool;
#[cfg(feature = "web")]
pub use list_agents::ListAgentsTool;
#[cfg(feature = "web")]
pub use send::{SendTool, CURRENT_ASSISTANT_ID};

#[cfg(feature = "browser")]
pub use browser::BrowserTool;
