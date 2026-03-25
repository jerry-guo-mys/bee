//! 文件系统工具组

use crate::domain::tool::ToolGroup;
use crate::domain::tool::ToolRegistry;
use std::path::PathBuf;

/// 文件系统工具组
pub struct FilesystemToolGroup {
    workspace_root: PathBuf,
}

impl FilesystemToolGroup {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }
}

impl ToolGroup for FilesystemToolGroup {
    fn name(&self) -> &'static str {
        "filesystem"
    }

    fn description(&self) -> &'static str {
        "File system operations: read, write, list files, search"
    }

    fn register(&self, _registry: &mut ToolRegistry) {
        // 实际实现会在这里注册具体工具
        // registry.register(ReadFileTool::new(&self.workspace_root));
        // registry.register(WriteFileTool::new(&self.workspace_root));
        // registry.register(ListFilesTool::new(&self.workspace_root));
        // registry.register(SearchInFilesTool::new(&self.workspace_root));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_tool_group_name() {
        let group = FilesystemToolGroup::new(PathBuf::from("/tmp"));
        assert_eq!(group.name(), "filesystem");
    }

    #[test]
    fn test_filesystem_tool_group_description() {
        let group = FilesystemToolGroup::new(PathBuf::from("/tmp"));
        assert!(group.description().contains("File system"));
    }
}
