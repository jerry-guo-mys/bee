//! Test Generator Plugin
//!
//! 自动生成单元测试的插件

use bee::{Plugin, PluginContext, PluginMetadata, PluginState, PluginError};
use std::any::Any;

mod generator;
mod strategies;

pub use generator::TestGenerator;

/// Test Generator 插件
pub struct TestGeneratorPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    context: Option<PluginContext>,
}

impl TestGeneratorPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::new(
                "test-generator",
                "Test Generator",
                "1.0.0",
            )
            .with_description("自动生成单元测试的插件，支持多种测试框架")
            .with_author("Bee Team"),
            state: PluginState::Created,
            context: None,
        }
    }
}

#[async_trait::async_trait]
impl Plugin for TestGeneratorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, ctx: &PluginContext) -> Result<(), PluginError> {
        self.context = Some(ctx.clone());
        self.state = PluginState::Initialized;
        tracing::info!("Test Generator plugin initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        self.state = PluginState::Stopped;
        tracing::info!("Test Generator plugin shutdown");
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
        let mut plugin = TestGeneratorPlugin::new();
        assert_eq!(plugin.state(), PluginState::Created);

        let context = PluginContext::new("/tmp");
        plugin.initialize(&context).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Initialized);

        plugin.shutdown().await.unwrap();
        assert_eq!(plugin.state(), PluginState::Stopped);
    }
}
