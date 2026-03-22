//! 测试夹具
//!
//! 提供常用的测试数据和配置

use std::sync::Arc;

use crate::config::AppConfig;
use crate::llm::MockLlmClient;
use crate::memory::Message;

/// 创建测试用的默认配置
pub fn test_config() -> AppConfig {
    AppConfig::default()
}

/// 创建 Mock LLM 客户端
pub fn mock_llm() -> Arc<MockLlmClient> {
    Arc::new(MockLlmClient)
}

/// 创建测试用的用户消息
pub fn user_message(content: &str) -> Message {
    Message {
        role: crate::memory::Role::User,
        content: content.to_string(),
    }
}

/// 创建测试用的助手消息
pub fn assistant_message(content: &str) -> Message {
    Message {
        role: crate::memory::Role::Assistant,
        content: content.to_string(),
    }
}

/// 测试用的工作空间路径
pub fn test_workspace() -> std::path::PathBuf {
    std::env::temp_dir().join("bee-test-workspace")
}

/// 清理测试文件
pub fn cleanup_test_files(path: &std::path::Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}
