//! Web 工具组

use crate::domain::tool::ToolGroup;
use crate::domain::tool::ToolRegistry;

/// Web 工具组
pub struct WebToolGroup;

impl WebToolGroup {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebToolGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolGroup for WebToolGroup {
    fn name(&self) -> &'static str {
        "web"
    }

    fn description(&self) -> &'static str {
        "Web operations: search, fetch pages, deep search"
    }

    fn register(&self, _registry: &mut ToolRegistry) {
        // 实际实现会在这里注册具体工具
        // registry.register(SearchTool);
        // registry.register(FetchPageTool);
        // registry.register(DeepSearchTool);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_tool_group_name() {
        assert_eq!(WebToolGroup.name(), "web");
    }

    #[test]
    fn test_web_tool_group_description() {
        assert!(WebToolGroup.description().contains("Web"));
    }
}
