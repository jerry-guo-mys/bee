//! 错误恢复集成测试
//!
//! 测试系统在各種错误场景下的恢复能力

#[cfg(test)]
mod tests {
    use bee::core::recovery::RecoveryEngine;
    use bee::core::{AgentError, RecoveryAction};
    use bee::domain::cognitive::context::ContextManager;
    use bee::memory::Message;

    #[test]
    fn test_recovery_engine_json_parse_error() {
        let engine = RecoveryEngine::new();
        let err = AgentError::JsonParseError("invalid json".to_string());
        let action = engine.handle(&err, &mut []);

        match action {
            RecoveryAction::RetryWithPrompt(msg) => {
                assert!(msg.contains("JSON"));
            }
            _ => panic!("Expected RetryWithPrompt"),
        }
    }

    #[test]
    fn test_recovery_engine_context_exceeded() {
        let engine = RecoveryEngine::new();
        let err = AgentError::ContextWindowExceeded;
        let action = engine.handle(&err, &mut []);

        assert!(matches!(action, RecoveryAction::SummarizeAndPrune));
    }

    #[test]
    fn test_recovery_engine_hallucinated_tool() {
        let engine = RecoveryEngine::new();
        let err = AgentError::HallucinatedTool("fake_tool".to_string());
        let action = engine.handle(&err, &mut []);

        match action {
            RecoveryAction::AskUser(msg) => {
                assert!(msg.contains("fake_tool"));
            }
            _ => panic!("Expected AskUser"),
        }
    }

    #[test]
    fn test_recovery_engine_tool_timeout() {
        let engine = RecoveryEngine::new();
        let err = AgentError::ToolTimeout("math".to_string());
        let action = engine.handle(&err, &mut []);

        match action {
            RecoveryAction::AskUser(msg) => {
                assert!(msg.contains("超时"));
            }
            _ => panic!("Expected AskUser"),
        }
    }

    #[test]
    fn test_recovery_engine_network_timeout() {
        let engine = RecoveryEngine::new();
        let err = AgentError::NetworkTimeout("test".to_string());
        let action = engine.handle(&err, &mut []);

        match action {
            RecoveryAction::RetryWithPrompt(msg) => {
                assert!(msg.contains("重试"));
            }
            _ => panic!("Expected RetryWithPrompt"),
        }
    }

    #[test]
    fn test_recovery_engine_llm_error() {
        let engine = RecoveryEngine::new();
        let err = AgentError::LlmError(bee::llm::LlmError::RateLimited {
            retry_after_ms: 1000,
        });
        let action = engine.handle(&err, &mut []);

        assert!(matches!(action, RecoveryAction::DowngradeModel));
    }

    #[test]
    fn test_context_manager_error_recovery() {
        let mut context = ContextManager::new(5);

        // 添加初始消息
        context.push_message(Message::user("Hello"));
        context.push_message(Message::assistant("Hi! How can I help?"));

        // 模拟错误场景
        context.push_message(Message::assistant("Let me call tool_name..."));
        context.push_message(Message::tool("Error: Tool not found"));

        // 助手应该能够从错误中恢复
        context.push_message(Message::assistant(
            "Sorry, let me try a different approach.",
        ));

        // 验证消息历史完整
        assert_eq!(context.conversation.messages().len(), 5);
    }

    #[test]
    fn test_recovery_from_hallucinated_tool() {
        let mut context = ContextManager::new(5);

        context.push_message(Message::user("Search the web"));
        context.push_message(Message::assistant("I'll use web_search_tool..."));
        context.push_message(Message::tool("Error: Unknown tool 'web_search_tool'"));

        // 恢复：使用正确的工具
        context.push_message(Message::assistant("Let me use the correct tool 'search'."));

        assert_eq!(context.conversation.messages().len(), 4);
    }

    #[test]
    fn test_recovery_from_json_parse_error() {
        let mut context = ContextManager::new(5);

        context.push_message(Message::user("Calculate 2+2"));
        context.push_message(Message::assistant("Let me think..."));
        // 模拟无效的 JSON 响应
        context.push_message(Message::assistant("{invalid json}"));

        // 恢复：重新生成有效响应
        context.push_message(Message::assistant("The answer is 4."));

        assert_eq!(context.conversation.messages().len(), 4);
    }

    #[test]
    fn test_recovery_context_exceeded() {
        let mut context = ContextManager::new(2); // 小上下文以触发剪枝

        // 添加多轮对话
        for i in 0..5 {
            context.push_message(Message::user(&format!("Question {}", i)));
            context.push_message(Message::assistant(&format!("Answer {}", i)));
        }

        // 上下文应该被正确管理
        assert!(context.conversation.messages().len() <= 4);
    }

    #[test]
    fn test_recovery_from_tool_timeout() {
        let mut context = ContextManager::new(5);

        context.push_message(Message::user("Run a long task"));
        context.push_message(Message::assistant("Starting the task..."));
        context.push_message(Message::tool("Error: Tool execution timeout"));

        // 恢复：通知用户超时
        context.push_message(Message::assistant(
            "The task took too long. Let me try a simpler approach.",
        ));

        assert_eq!(context.conversation.messages().len(), 4);
    }

    #[test]
    fn test_recovery_with_alternative_tool() {
        let mut context = ContextManager::new(10);

        // 尝试使用工具失败
        context.push_message(Message::user("Get weather for Paris"));
        context.push_message(Message::assistant("Let me check weather_api..."));
        context.push_message(Message::tool("Error: API unavailable"));

        // 恢复：使用替代工具
        context.push_message(Message::assistant("Let me try openweathermap instead..."));
        context.push_message(Message::tool("Paris: 20°C, Sunny"));
        context.push_message(Message::assistant(
            "The weather in Paris is 20°C and sunny.",
        ));

        assert_eq!(context.conversation.messages().len(), 6);
    }

    #[test]
    fn test_recovery_graceful_degradation() {
        let mut context = ContextManager::new(10);

        // 多次工具失败
        context.push_message(Message::user("Do complex analysis"));
        context.push_message(Message::assistant("Trying advanced tool..."));
        context.push_message(Message::tool("Error: Tool failed"));

        context.push_message(Message::assistant("Trying simpler approach..."));
        context.push_message(Message::tool("Error: Still failed"));

        // 最终回退到基本响应
        context.push_message(Message::assistant("I apologize, but I cannot complete this task with the available tools. However, I can provide general information..."));

        assert_eq!(context.conversation.messages().len(), 6);
    }
}
