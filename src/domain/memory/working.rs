//! 工作记忆

/// 工作记忆：存储当前任务目标、尝试和失败
#[derive(Debug, Clone, Default)]
pub struct WorkingMemory {
    goal: Option<String>,
    attempts: Vec<String>,
    failures: Vec<String>,
}

impl WorkingMemory {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置目标
    pub fn set_goal(&mut self, goal: &str) {
        self.goal = Some(goal.to_string());
    }

    /// 添加尝试
    pub fn add_attempt(&mut self, attempt: &str) {
        self.attempts.push(attempt.to_string());
    }

    /// 添加失败
    pub fn add_failure(&mut self, failure: &str) {
        self.failures.push(failure.to_string());
    }

    /// 清空
    pub fn clear(&mut self) {
        self.goal = None;
        self.attempts.clear();
        self.failures.clear();
    }

    /// 转换为 prompt 段落
    pub fn to_prompt_section(&self) -> String {
        let mut section = String::new();

        if let Some(ref goal) = self.goal {
            section.push_str(&format!("\n## Current Goal\n{}\n", goal));
        }

        if !self.attempts.is_empty() {
            section.push_str("\n## Attempts\n");
            for attempt in &self.attempts {
                section.push_str(&format!("- {}\n", attempt));
            }
        }

        if !self.failures.is_empty() {
            section.push_str("\n## Failures\n");
            for failure in &self.failures {
                section.push_str(&format!("- {}\n", failure));
            }
        }

        section
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_working_memory_to_prompt() {
        let mut wm = WorkingMemory::new();
        wm.set_goal("Find files");
        wm.add_attempt("ls -> listing");
        wm.add_failure("Permission denied");

        let section = wm.to_prompt_section();
        assert!(section.contains("Find files"));
        assert!(section.contains("ls"));
        assert!(section.contains("Permission denied"));
    }
}
