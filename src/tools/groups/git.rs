//! Git 工具组

use crate::domain::tool::ToolGroup;
use crate::domain::tool::ToolRegistry;

/// Git 工具组
pub struct GitToolGroup {
    #[allow(dead_code)]
    repo_root: std::path::PathBuf,
}

impl GitToolGroup {
    pub fn new(repo_root: std::path::PathBuf) -> Self {
        Self { repo_root }
    }
}

impl ToolGroup for GitToolGroup {
    fn name(&self) -> &'static str {
        "git"
    }

    fn description(&self) -> &'static str {
        "Git operations: status, diff, add, commit, log"
    }

    fn register(&self, _registry: &mut ToolRegistry) {
        // 实际实现会在这里注册具体工具
        // registry.register(GitStatusTool::new(&self.repo_root));
        // registry.register(GitDiffTool::new(&self.repo_root));
        // registry.register(GitAddTool::new(&self.repo_root));
        // registry.register(GitCommitTool::new(&self.repo_root));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_tool_group_name() {
        let group = GitToolGroup::new(std::path::PathBuf::from("/tmp"));
        assert_eq!(group.name(), "git");
    }
}
