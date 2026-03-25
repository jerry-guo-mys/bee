//! 工具注册表

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::tool::metadata::ToolMetadata;
use crate::domain::tool::trait_::Tool;

/// 工具注册表
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册工具
    pub fn register(&mut self, tool: impl Tool + 'static) {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
    }

    /// 获取工具
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// 执行工具
    pub async fn execute(&self, name: &str, args: Value) -> Result<String, String> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| format!("Unknown tool: {name}"))?;
        tool.execute(args).await
    }

    /// 获取工具名称列表
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// 获取工具元数据
    pub fn tool_metadata(&self, name: &str) -> Option<ToolMetadata> {
        self.tools.get(name).map(|tool| tool.metadata())
    }

    /// 获取工具描述列表
    pub fn tool_descriptions(&self) -> Vec<(String, String)> {
        self.tools
            .iter()
            .map(|(name, tool)| (name.clone(), tool.description().to_string()))
            .collect()
    }
}

use serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockTool;

    #[tokio::test]
    async fn test_registry_register_and_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(MockTool::new("test", "Test tool"));

        let result = registry.execute("test", serde_json::json!({})).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_registry_tool_names() {
        let mut registry = ToolRegistry::new();
        registry.register(MockTool::new("tool1", "Tool 1"));
        registry.register(MockTool::new("tool2", "Tool 2"));

        let names = registry.tool_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"tool1".to_string()));
        assert!(names.contains(&"tool2".to_string()));
    }
}
