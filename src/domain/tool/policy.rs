//! 工具策略

/// 工具使用策略
#[derive(Debug, Clone, Default)]
pub struct ToolPolicy {
    /// 允许的工具列表
    pub allowed_tools: Vec<String>,
    /// 禁止的工具列表
    pub disallowed_tools: Vec<String>,
    /// 需要审批的工具列表
    pub require_approval: Vec<String>,
}

impl ToolPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查工具是否允许
    pub fn is_allowed(&self, tool_name: &str) -> bool {
        if self.disallowed_tools.contains(&tool_name.to_string()) {
            return false;
        }

        if self.allowed_tools.is_empty() {
            return true;
        }

        self.allowed_tools.contains(&tool_name.to_string())
    }

    /// 检查工具是否需要审批
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        self.require_approval.contains(&tool_name.to_string())
    }

    /// 添加工具到允许列表
    pub fn allow(&mut self, tool_name: &str) {
        if !self.allowed_tools.contains(&tool_name.to_string()) {
            self.allowed_tools.push(tool_name.to_string());
        }
    }

    /// 添加工具到禁止列表
    pub fn disallow(&mut self, tool_name: &str) {
        if !self.disallowed_tools.contains(&tool_name.to_string()) {
            self.disallowed_tools.push(tool_name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_policy_is_allowed() {
        let mut policy = ToolPolicy::new();
        policy.allow("cat");
        policy.allow("ls");
        policy.disallow("rm");

        assert!(policy.is_allowed("cat"));
        assert!(policy.is_allowed("ls"));
        assert!(!policy.is_allowed("rm"));
        assert!(!policy.is_allowed("unknown")); // 当 allowed_tools 非空时，默认拒绝
    }

    #[test]
    fn test_tool_policy_requires_approval() {
        let mut policy = ToolPolicy::new();
        policy.require_approval.push("shell".to_string());

        assert!(policy.requires_approval("shell"));
        assert!(!policy.requires_approval("cat"));
    }
}
