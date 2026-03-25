//! Code Analyzer Plugin
//!
//! 代码静态分析插件入口

use bee::{Plugin, PluginContext, PluginMetadata, PluginState, PluginError};
use std::any::Any;

mod analyzer;
mod complexity;
mod smells;

pub use analyzer::CodeAnalyzer as AnalyzerImpl;

/// Code Analyzer 插件
pub struct CodeAnalyzerPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    context: Option<PluginContext>,
}

impl CodeAnalyzerPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "code-analyzer",
                "Code Analyzer",
                "1.0.0",
            )
            .with_description("代码静态分析插件，支持复杂度分析、代码 smell 检测")
            .with_author("Bee Team"),
            state: PluginState::Created,
            context: None,
        }
    }
}

#[async_trait::async_trait]
impl Plugin for CodeAnalyzerPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, ctx: &PluginContext) -> Result<(), PluginError> {
        self.context = Some(ctx.clone());
        self.state = PluginState::Initialized;
        tracing::info!("Code Analyzer plugin initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        self.state = PluginState::Stopped;
        tracing::info!("Code Analyzer plugin shutdown");
        Ok(())
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = CodeAnalyzerPlugin::new();
        assert_eq!(plugin.state(), PluginState::Created);

        let context = PluginContext::new("/tmp");
        plugin.initialize(&context).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Initialized);

        plugin.shutdown().await.unwrap();
        assert_eq!(plugin.state(), PluginState::Stopped);
    }
}
