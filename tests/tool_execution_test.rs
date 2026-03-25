//! 工具执行集成测试
//!
//! 测试工具注册、调用和结果处理

#[cfg(test)]
mod tests {
    use bee::domain::tool::registry::ToolRegistry;
    use bee::domain::tool::Tool;
    use serde_json::Value;

    // 简单的测试工具
    struct EchoTool;

    #[async_trait::async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "Echo back the input message" }

        async fn execute(&self, args: Value) -> Result<String, String> {
            let message = args.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("no message");
            Ok(format!("Echo: {}", message))
        }
    }

    struct MathTool;

    #[async_trait::async_trait]
    impl Tool for MathTool {
        fn name(&self) -> &str { "math" }
        fn description(&self) -> &str { "Perform basic math calculations" }

        async fn execute(&self, args: Value) -> Result<String, String> {
            let operation = args.get("operation")
                .and_then(|v| v.as_str())
                .unwrap_or("add");
            let a = args.get("a")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let b = args.get("b")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    a / b
                }
                _ => return Err(format!("Unknown operation: {}", operation)),
            };

            Ok(result.to_string())
        }
    }

    #[tokio::test]
    async fn test_tool_registry_register_and_execute() {
        let mut registry = ToolRegistry::new();

        registry.register(EchoTool);

        let result = registry
            .execute("echo", serde_json::json!({"message": "Hello"}))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Echo: Hello"));
    }

    #[tokio::test]
    async fn test_tool_registry_multiple_tools() {
        let mut registry = ToolRegistry::new();

        registry.register(EchoTool);
        registry.register(MathTool);

        let tool_names = registry.tool_names();

        assert!(tool_names.contains(&"echo".to_string()));
        assert!(tool_names.contains(&"math".to_string()));
    }

    #[tokio::test]
    async fn test_math_tool_operations() {
        let mut registry = ToolRegistry::new();
        registry.register(MathTool);

        // Test addition
        let result = registry
            .execute("math", serde_json::json!({"operation": "add", "a": 10, "b": 5}))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "15");

        // Test division by zero
        let result = registry
            .execute("math", serde_json::json!({"operation": "divide", "a": 10, "b": 0}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = ToolRegistry::new();

        let result = registry
            .execute("nonexistent", serde_json::json!({}))
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_tool_execution_with_invalid_input() {
        let mut registry = ToolRegistry::new();
        registry.register(MathTool);

        // Test with missing required fields
        let result = registry
            .execute("math", serde_json::json!({}))
            .await;

        // Math tool should handle missing fields gracefully
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0");
    }
}
