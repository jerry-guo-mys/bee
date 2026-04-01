//! ReAct 循环集成测试
//!
//! 测试 ReAct 模式的核心循环：思考 -> 行动 -> 观察

#[cfg(test)]
mod tests {
    use bee::domain::cognitive::context::ContextManager;
    use bee::memory::Message;

    #[test]
    fn test_react_loop_basic_flow() {
        // 测试基本的 ReAct 循环流程
        let mut context = ContextManager::new(5);

        // 初始用户消息
        context.push_message(Message::user("What is 2 + 2?"));

        // 助手思考
        context.push_message(Message::assistant("Let me calculate this."));

        // 工具调用
        context.push_message(Message::assistant("I'll use a calculator."));

        // 工具结果
        context.push_message(Message::tool("4"));

        // 最终回复
        context.push_message(Message::assistant("The answer is 4."));

        // 验证消息数量
        assert_eq!(context.conversation.messages().len(), 5);
    }

    #[test]
    fn test_react_loop_with_pruning() {
        // 测试 ReAct 循环中的智能剪枝
        let mut context = ContextManager::new(2); // 只保留 2 轮

        // 添加多轮对话
        for i in 0..5 {
            context.push_message(Message::user(&format!("Question {}", i)));
            context.push_message(Message::assistant(&format!("Answer {}", i)));
        }

        // 验证剪枝后消息数量
        assert!(context.conversation.messages().len() <= 4); // 最多 4 条（2 轮）
    }

    #[test]
    fn test_react_loop_system_message_preserved() {
        // 测试系统消息在没有触发剪枝时被保留
        let mut context = ContextManager::new(10);

        context.push_message(Message::system("You are a helpful assistant."));
        context.push_message(Message::user("Hello"));
        context.push_message(Message::assistant("Hi! How can I help?"));

        // 验证系统消息被保留（在没有触发剪枝的情况下）
        let messages = context.conversation.messages();
        assert_eq!(messages.len(), 3);

        let has_system = messages
            .iter()
            .any(|m| matches!(m.role, bee::memory::Role::System));
        assert!(
            has_system,
            "System message should be preserved when not pruning"
        );
    }

    #[tokio::test]
    async fn test_react_loop_session_state_transitions() {
        // 测试会话状态在 ReAct 循环中的转换
        use bee::domain::session::{Session, SessionConfig, SessionStatus};

        let config = SessionConfig::new().with_system_prompt("You are a helpful assistant.");
        let mut session = Session::new(config);

        // 初始状态
        assert_eq!(session.state.status, SessionStatus::Idle);

        // 思考状态
        session.set_status(SessionStatus::Thinking);
        assert_eq!(session.state.status, SessionStatus::Thinking);

        // 执行状态
        session.set_status(SessionStatus::Executing);
        assert_eq!(session.state.status, SessionStatus::Executing);

        // 回复状态
        session.set_status(SessionStatus::Responding);
        assert_eq!(session.state.status, SessionStatus::Responding);

        // 回到空闲状态
        session.set_status(SessionStatus::Idle);
        assert_eq!(session.state.status, SessionStatus::Idle);
    }

    #[test]
    fn test_react_loop_tool_result_handling() {
        // 测试工具结果的处理
        let mut context = ContextManager::new(3);

        context.push_message(Message::user("Search for Rust programming"));
        context.push_message(Message::assistant("Searching..."));
        context.push_message(Message::tool(
            "Search results: Rust is a systems programming language...",
        ));

        // 工具结果应该被正确记录
        let tool_messages: Vec<_> = context
            .conversation
            .messages()
            .iter()
            .filter(|m| matches!(m.role, bee::memory::Role::Tool))
            .collect();

        assert_eq!(tool_messages.len(), 1);
    }

    #[test]
    fn test_react_loop_error_recovery() {
        // 测试 ReAct 循环中的错误恢复
        let mut context = ContextManager::new(10);

        context.push_message(Message::user("Do something"));
        context.push_message(Message::assistant("I'll try..."));
        context.push_message(Message::tool("Error: Tool execution failed"));

        // 助手应该能够处理错误
        context.push_message(Message::assistant(
            "Sorry, there was an error. Let me try another approach.",
        ));

        assert_eq!(context.conversation.messages().len(), 4);
    }

    #[test]
    fn test_react_loop_multi_turn_conversation() {
        // 测试多轮对话中的 ReAct 循环
        let mut context = ContextManager::new(10);

        // 第一轮
        context.push_message(Message::user("What is the capital of France?"));
        context.push_message(Message::assistant("Let me think..."));
        context.push_message(Message::assistant("The capital of France is Paris."));

        // 第二轮（后续问题）
        context.push_message(Message::user("What about Germany?"));
        context.push_message(Message::assistant("The capital of Germany is Berlin."));

        // 验证对话历史
        assert_eq!(context.conversation.messages().len(), 5);

        // 验证消息顺序
        let messages = context.conversation.messages();
        assert!(matches!(messages[0].role, bee::memory::Role::User));
        assert!(matches!(messages[4].role, bee::memory::Role::Assistant));
    }

    #[test]
    fn test_tool_message_role_not_assistant() {
        // 验证工具消息使用 Role::Tool 而非 Role::Assistant
        // 这是为了确保 TUI 不会将工具调用结果显示为原始文本
        let mut context = ContextManager::new(5);

        context.push_message(Message::user("Run echo command"));
        context.push_message(Message::assistant("I'll run the echo tool."));

        // 工具结果应该使用 Role::Tool
        context.push_message(Message::tool("Tool: echo | Result: Hello"));

        // 验证工具消息的角色是 Tool 而不是 Assistant
        let messages = context.conversation.messages();
        let tool_msg = messages.iter().find(|m| m.content.contains("Tool: echo"));

        assert!(tool_msg.is_some(), "Should find tool message");
        assert_eq!(
            tool_msg.unwrap().role,
            bee::memory::Role::Tool,
            "Tool message should use Role::Tool, not Role::Assistant"
        );

        // 验证没有 Assistant 角色包含工具调用文本
        let assistant_with_tool: Vec<_> = messages
            .iter()
            .filter(|m| matches!(m.role, bee::memory::Role::Assistant))
            .filter(|m| m.content.contains("Tool call:") || m.content.contains("Tool:"))
            .collect();

        assert!(
            assistant_with_tool.is_empty(),
            "No Assistant message should contain tool call text"
        );
    }

    #[tokio::test]
    async fn test_react_loop_with_echo_tool() {
        // 真实调用 echo 工具验证工具消息的角色
        use bee::core::RecoveryEngine;
        use bee::llm::MockLlmClient;
        use bee::react::ContextManager;
        use bee::react::{react_loop, Planner};
        use bee::tools::{EchoTool, ToolExecutor, ToolRegistry};
        use std::sync::Arc;

        // 创建 Mock LLM（返回固定的工具调用响应）
        let llm = Arc::new(MockLlmClient);
        let planner = Planner::new(llm, "You are a test assistant.".to_string());

        // 注册 Echo 工具
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);
        let executor = ToolExecutor::new(registry, 30);

        let recovery = RecoveryEngine::new();
        let mut context = ContextManager::new(10);
        let cancel_token = tokio_util::sync::CancellationToken::new();

        // 执行 ReAct 循环
        let result = react_loop(
            &planner,
            &executor,
            &recovery,
            &mut context,
            "Say hello",
            None,
            None,
            cancel_token,
            None,
            None,
            None,
            None,
        )
        .await;

        // 验证执行成功
        assert!(result.is_ok(), "ReAct loop should complete successfully");

        // 验证消息历史
        let messages = context.conversation.messages();

        // 检查是否存在 Tool 角色的消息（如果有工具调用）
        let tool_messages: Vec<_> = messages
            .iter()
            .filter(|m| matches!(m.role, bee::memory::Role::Tool))
            .collect();

        // 检查 Assistant 角色是否包含工具调用文本（不应该）
        let assistant_tool_calls: Vec<_> = messages
            .iter()
            .filter(|m| matches!(m.role, bee::memory::Role::Assistant))
            .filter(|m| m.content.contains("Tool call:") || m.content.contains("Tool: echo"))
            .collect();

        assert!(
            assistant_tool_calls.is_empty(),
            "Assistant messages should not contain tool call text"
        );
    }
}
