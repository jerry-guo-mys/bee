//! 测试运行器
//!
//! 提供集成的测试环境设置和清理

use std::sync::Arc;

use crate::config::AppConfig;
use crate::core::{create_agent_builder, AgentComponents};
use crate::llm::MockLlmClient;

/// 测试运行器：提供完整的测试环境
pub struct TestHarness {
    pub config: AppConfig,
    pub components: AgentComponents,
    pub workspace: std::path::PathBuf,
}

impl TestHarness {
    /// 创建新的测试运行器
    pub fn new() -> Self {
        let config = AppConfig::default();
        let workspace = std::env::temp_dir().join(format!("bee-test-{}", uuid::Uuid::new_v4()));

        std::fs::create_dir_all(&workspace).ok();

        let builder = create_agent_builder(None)
            .with_system_prompt("You are a test assistant.")
            .with_critic(false);

        let components = builder.build_components();

        Self {
            config,
            components,
            workspace,
        }
    }

    /// 使用自定义配置创建测试运行器
    pub fn with_config(config: AppConfig) -> Self {
        let workspace = std::env::temp_dir().join(format!("bee-test-{}", uuid::Uuid::new_v4()));

        std::fs::create_dir_all(&workspace).ok();

        let builder = create_agent_builder(None).with_system_prompt("You are a test assistant.");

        let components = builder.build_components();

        Self {
            config,
            components,
            workspace,
        }
    }

    /// 获取 Mock LLM 客户端
    pub fn mock_llm(&self) -> Arc<MockLlmClient> {
        // 注意：这里返回的是通用的 MockLlmClient
        // 实际测试中应该使用 components 中的 LLM
        Arc::new(MockLlmClient)
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        // 清理测试工作空间
        if self.workspace.exists() {
            let _ = std::fs::remove_dir_all(&self.workspace);
        }
    }
}

/// 断言工具
pub mod assertions {
    /// 断言结果是否为 Ok
    #[macro_export]
    macro_rules! assert_ok {
        ($result:expr) => {
            assert!($result.is_ok(), "Expected Ok, got Err: {:?}", $result);
        };
    }

    /// 断言结果是否为 Err
    #[macro_export]
    macro_rules! assert_err {
        ($result:expr) => {
            assert!($result.is_err(), "Expected Err, got Ok");
        };
    }

    /// 断言两个值近似相等（用于浮点数）
    #[macro_export]
    macro_rules! assert_approx_eq {
        ($a:expr, $b:expr, $eps:expr) => {
            assert!(
                ($a - $b).abs() < $eps,
                "Expected {:?} ≈ {:?} (epsilon: {}), but diff = {}",
                $a,
                $b,
                $eps,
                ($a - $b).abs()
            );
        };
    }
}
