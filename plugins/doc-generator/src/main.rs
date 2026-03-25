//! Documentation Generator Plugin
//!
//! 自动生成代码文档的插件

use bee::{Plugin, PluginContext, PluginMetadata, PluginState, PluginError};
use std::any::Any;

mod generator;
mod templates;

pub use generator::DocGenerator;

/// Documentation Generator 插件
pub struct DocGeneratorPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    context: Option<PluginContext>,
}

impl DocGeneratorPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "doc-generator",
                "Documentation Generator",
                "1.0.0",
            )
            .with_description("自动生成代码文档的插件，支持 Markdown 和 HTML 格式")
            .with_author("Bee Team"),
            state: PluginState::Created,
            context: None,
        }
    }
}

#[async_trait::async_trait]
impl Plugin for DocGeneratorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, ctx: &PluginContext) -> Result<(), PluginError> {
        self.context = Some(ctx.clone());
        self.state = PluginState::Initialized;
        tracing::info!("Documentation Generator plugin initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        self.state = PluginState::Stopped;
        tracing::info!("Documentation Generator plugin shutdown");
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
        let mut plugin = DocGeneratorPlugin::new();
        assert_eq!(plugin.state(), PluginState::Created);

        let context = PluginContext::new("/tmp");
        plugin.initialize(&context).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Initialized);

        plugin.shutdown().await.unwrap();
        assert_eq!(plugin.state(), PluginState::Stopped);
    }
}
