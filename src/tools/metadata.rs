//! 工具元数据：用于路由、策略、审计与提示词构建。

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolScope {
    LocalWorkspace,
    RemoteWeb,
    GitHub,
    System,
    Internal,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolIntent {
    ReadFile,
    ReadCode,
    ListDirectory,
    FetchWebPage,
    InspectRepository,
    BrowseInteractive,
    RunCommand,
    Research,
    WriteFile,
    ExecuteSideEffect,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFreshness {
    Static,
    BestEffort,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutputShape {
    PlainText,
    StructuredJson,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolUseCase {
    DirectExplanation,
    TimeSensitiveCurrent,
    ExternalGitHubRepo,
    LocalWorkspaceInspection,
    Weather,
    News,
    ExchangeRate,
    MarketQuote,
    SportsScore,
    Testing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCriticMode {
    Skip,
    Conservative,
    Normal,
    Always,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolMetadata {
    pub scope: ToolScope,
    pub intents: Vec<ToolIntent>,
    pub risk: ToolRisk,
    pub output_shape: ToolOutputShape,
    pub freshness: ToolFreshness,
    pub supports_side_effects: bool,
    pub preferred_use_cases: Vec<ToolUseCase>,
    pub disallowed_use_cases: Vec<ToolUseCase>,
    pub requires_explicit_user_request: bool,
    pub critic_mode: ToolCriticMode,
}

impl ToolMetadata {
    pub fn new(scope: ToolScope, intents: Vec<ToolIntent>) -> Self {
        Self {
            scope,
            intents,
            risk: ToolRisk::Low,
            output_shape: ToolOutputShape::PlainText,
            freshness: ToolFreshness::Static,
            supports_side_effects: false,
            preferred_use_cases: Vec::new(),
            disallowed_use_cases: Vec::new(),
            requires_explicit_user_request: false,
            critic_mode: ToolCriticMode::Normal,
        }
    }

    pub fn with_risk(mut self, risk: ToolRisk) -> Self {
        self.risk = risk;
        self
    }

    pub fn with_output_shape(mut self, output_shape: ToolOutputShape) -> Self {
        self.output_shape = output_shape;
        self
    }

    pub fn with_freshness(mut self, freshness: ToolFreshness) -> Self {
        self.freshness = freshness;
        self
    }

    pub fn with_side_effects(mut self, supports_side_effects: bool) -> Self {
        self.supports_side_effects = supports_side_effects;
        self
    }

    pub fn with_preferred_use_cases(mut self, use_cases: Vec<ToolUseCase>) -> Self {
        self.preferred_use_cases = use_cases;
        self
    }

    pub fn with_disallowed_use_cases(mut self, use_cases: Vec<ToolUseCase>) -> Self {
        self.disallowed_use_cases = use_cases;
        self
    }

    pub fn with_requires_explicit_user_request(
        mut self,
        requires_explicit_user_request: bool,
    ) -> Self {
        self.requires_explicit_user_request = requires_explicit_user_request;
        self
    }

    pub fn with_critic_mode(mut self, critic_mode: ToolCriticMode) -> Self {
        self.critic_mode = critic_mode;
        self
    }
}
