//! 工具组合原语

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::tool::trait_::Tool;
use crate::domain::tool::ToolMetadata;

/// 工具链：顺序执行多个工具
pub struct ToolChain {
    name: String,
    tools: Vec<Box<dyn Tool>>,
}

impl ToolChain {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tools: vec![],
        }
    }

    pub fn add_tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    pub fn add_tools(mut self, tools: Vec<Box<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }
}

#[async_trait]
impl Tool for ToolChain {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Execute multiple tools in sequence"
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let mut result = String::new();
        for tool in &self.tools {
            let output = tool.execute(args.clone()).await?;
            result.push_str(&output);
            result.push('\n');
        }
        Ok(result)
    }
}

/// 工具管道：前一个工具的输出作为后一个工具的输入
pub struct ToolPipeline {
    name: String,
    stages: Vec<Box<dyn Tool>>,
}

impl ToolPipeline {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            stages: vec![],
        }
    }

    pub fn add_stage(mut self, tool: impl Tool + 'static) -> Self {
        self.stages.push(Box::new(tool));
        self
    }
}

#[async_trait]
impl Tool for ToolPipeline {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Execute tools in pipeline, output of one becomes input of next"
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let mut current = args;
        for stage in &self.stages {
            let output = stage.execute(current).await?;
            current = serde_json::from_str(&output)
                .unwrap_or_else(|_| serde_json::json!({ "output": output }));
        }
        Ok(current.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockTool;

    #[tokio::test]
    async fn test_tool_chain() {
        let chain = ToolChain::new("test_chain")
            .add_tool(MockTool::new("mock1", "Mock 1").with_response("Result 1"))
            .add_tool(MockTool::new("mock2", "Mock 2").with_response("Result 2"));

        let result = chain.execute(serde_json::json!({})).await.unwrap();
        assert!(result.contains("Result 1"));
        assert!(result.contains("Result 2"));
    }
}
