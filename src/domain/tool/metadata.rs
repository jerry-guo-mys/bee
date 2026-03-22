//! 工具元数据

use serde::Serialize;

/// 工具作用域
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum ToolScope {
    LocalWorkspace,
    RemoteWeb,
    GitHub,
    System,
    Internal,
    #[default]
    Mixed,
}

/// 工具意图
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum ToolIntent {
    ReadFile,
    ReadCode,
    ListDirectory,
    FetchWebPage,
    InspectRepository,
    RunCommand,
    WriteFile,
    #[default]
    Other,
}

/// 工具风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum ToolRisk {
    Low,
    Medium,
    #[default]
    High,
}

/// 工具输出形状
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum ToolOutputShape {
    PlainText,
    StructuredJson,
    #[default]
    Mixed,
}

/// 工具新鲜度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum ToolFreshness {
    Static,
    BestEffort,
    #[default]
    Live,
}

/// 工具用例
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum ToolUseCase {
    DirectExplanation,
    TimeSensitiveCurrent,
    LocalWorkspaceInspection,
    #[default]
    Other,
}

/// Critic 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum ToolCriticMode {
    Skip,
    Conservative,
    #[default]
    Normal,
    Always,
}

/// 工具元数据
#[derive(Debug, Clone, Serialize, Default)]
pub struct ToolMetadata {
    pub scope: ToolScope,
    pub intents: Vec<ToolIntent>,
    pub risk: ToolRisk,
    pub output_shape: ToolOutputShape,
    pub freshness: ToolFreshness,
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

    pub fn with_critic_mode(mut self, critic_mode: ToolCriticMode) -> Self {
        self.critic_mode = critic_mode;
        self
    }
}
