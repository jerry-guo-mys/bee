//! Mock 工具实现

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::tool::{Tool, ToolMetadata, ToolRegistry};

/// Mock 工具，用于测试
pub struct MockTool {
    name: String,
    description: String,
    response: String,
    should_fail: bool,
    call_count: std::sync::atomic::AtomicUsize,
}

impl MockTool {
    /// 创建新的 Mock 工具
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            response: format!("Mock {} response", name),
            should_fail: false,
            call_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// 设置响应内容
    pub fn with_response(mut self, response: &str) -> Self {
        self.response = response.to_string();
        self
    }

    /// 设置为失败模式
    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    /// 获取调用次数
    pub fn call_count(&self) -> usize {
        self.call_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 重置调用计数
    pub fn reset_call_count(&self) {
        self.call_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }

    async fn execute(&self, _args: Value) -> Result<String, String> {
        self.call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if self.should_fail {
            Err("Mock tool failed".to_string())
        } else {
            Ok(self.response.clone())
        }
    }
}

/// 创建包含 Mock 工具的注册表
pub fn create_mock_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(MockTool::new("mock_echo", "Mock echo tool"));
    registry.register(MockTool::new("mock_cat", "Mock cat tool"));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_mock_tool_execute() {
        let tool = MockTool::new("test", "Test tool");
        let result = tool.execute(json!({})).await;

        assert!(result.is_ok());
        assert!(result.unwrap().contains("Mock test response"));
    }

    #[tokio::test]
    async fn test_mock_tool_failure() {
        let tool = MockTool::new("test", "Test tool").with_failure();
        let result = tool.execute(json!({})).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Mock tool failed");
    }

    #[tokio::test]
    async fn test_mock_tool_call_count() {
        let tool = MockTool::new("test", "Test tool");

        assert_eq!(tool.call_count(), 0);

        let _ = tool.execute(json!({})).await;
        assert_eq!(tool.call_count(), 1);

        let _ = tool.execute(json!({})).await;
        assert_eq!(tool.call_count(), 2);
    }

    #[test]
    fn test_create_mock_registry() {
        let registry = create_mock_registry();

        assert!(registry.get("mock_echo").is_some());
        assert!(registry.get("mock_cat").is_some());
    }
}
