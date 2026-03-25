//! 工具 trait 定义

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::tool::metadata::ToolMetadata;

/// 工具 trait
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称
    fn name(&self) -> &str;

    /// 工具描述
    fn description(&self) -> &str;

    /// 工具元数据
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }

    /// 工具超时（秒）
    fn timeout_secs(&self) -> Option<u64> {
        None
    }

    /// 执行工具
    async fn execute(&self, args: Value) -> Result<String, String>;
}
