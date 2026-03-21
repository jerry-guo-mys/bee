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
pub enum ToolOutputShape {
    PlainText,
    StructuredJson,
    Mixed,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolMetadata {
    pub scope: ToolScope,
    pub intents: Vec<ToolIntent>,
    pub risk: ToolRisk,
    pub output_shape: ToolOutputShape,
    pub supports_freshness: bool,
    pub supports_side_effects: bool,
}

impl ToolMetadata {
    pub fn new(scope: ToolScope, intents: Vec<ToolIntent>) -> Self {
        Self {
            scope,
            intents,
            risk: ToolRisk::Low,
            output_shape: ToolOutputShape::PlainText,
            supports_freshness: false,
            supports_side_effects: false,
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

    pub fn with_freshness(mut self, supports_freshness: bool) -> Self {
        self.supports_freshness = supports_freshness;
        self
    }

    pub fn with_side_effects(mut self, supports_side_effects: bool) -> Self {
        self.supports_side_effects = supports_side_effects;
        self
    }
}
