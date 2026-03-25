//! 工具分组抽象

use crate::domain::tool::ToolRegistry;

/// 工具组 trait
pub trait ToolGroup: Send + Sync {
    /// 组名称
    fn name(&self) -> &'static str;

    /// 描述
    fn description(&self) -> &'static str;

    /// 注册工具到注册表
    fn register(&self, registry: &mut ToolRegistry);
}

/// 文件系统工具组
pub struct FilesystemToolGroup {
    #[allow(dead_code)]
    workspace_root: std::path::PathBuf,
}

impl FilesystemToolGroup {
    pub fn new(workspace_root: std::path::PathBuf) -> Self {
        Self { workspace_root }
    }
}

impl ToolGroup for FilesystemToolGroup {
    fn name(&self) -> &'static str {
        "filesystem"
    }

    fn description(&self) -> &'static str {
        "File system operations: read, write, list files"
    }

    fn register(&self, _registry: &mut ToolRegistry) {
        // 实际实现会在这里注册具体工具
        // registry.register(CatTool::new(&self.workspace_root));
        // registry.register(LsTool::new(&self.workspace_root));
    }
}

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
        "Code operations: read, edit, grep, review"
    }

    fn register(&self, _registry: &mut ToolRegistry) {
        // registry.register(CodeReadTool);
        // registry.register(CodeEditTool);
    }
}

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
        "Web operations: search, fetch pages"
    }

    fn register(&self, _registry: &mut ToolRegistry) {
        // registry.register(SearchTool);
        // registry.register(WeatherTool);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_group_names() {
        assert_eq!(CodeToolGroup.name(), "code");
        assert_eq!(WebToolGroup.name(), "web");
    }
}
