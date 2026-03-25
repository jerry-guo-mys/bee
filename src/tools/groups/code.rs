//! 代码工具组

use crate::domain::tool::ToolGroup;
use crate::domain::tool::ToolRegistry;

/// 代码工具组
pub struct CodeToolGroup;

impl CodeToolGroup {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodeToolGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolGroup for CodeToolGroup {
    fn name(&self) -> &'static str {
        "code"
    }

    fn description(&self) -> &'static str {
        "Code operations: read, edit, grep, review, analyze"
    }

    fn register(&self, _registry: &mut ToolRegistry) {
        // 实际实现会在这里注册具体工具
        // registry.register(CodeReadTool);
        // registry.register(CodeEditTool);
        // registry.register(CodeGrepTool);
        // registry.register(CodeReviewTool);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_tool_group_name() {
        assert_eq!(CodeToolGroup.name(), "code");
    }

    #[test]
    fn test_code_tool_group_description() {
        assert!(CodeToolGroup.description().contains("Code"));
    }
}
