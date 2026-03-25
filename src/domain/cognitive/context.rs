//! 上下文管理器：整合短期/中期/长期记忆
//!
//! 将短期（Conversation）、中期（Working）、长期（LongTerm）统一为 ContextManager，
//! 供 ReAct 循环拼 system prompt 与写入长期记忆。

use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::memory::{ConversationMemory, WorkingMemory};
use crate::memory::Message;
use crate::memory::{
    append_lesson, append_preference, append_procedural, load_lessons, load_preferences,
    load_procedural, LongTermMemory,
};

/// 上下文管理器：整合短期/中期/长期记忆
#[derive(Clone)]
pub struct ContextManager {
    pub conversation: ConversationMemory,
    pub working: WorkingMemory,
    pub long_term: Option<Arc<dyn LongTermMemory>>,
    /// 行为约束/教训文件路径
    pub lessons_path: Option<PathBuf>,
    /// 程序记忆文件路径
    pub procedural_path: Option<PathBuf>,
    /// 用户偏好文件路径
    pub preferences_path: Option<PathBuf>,
    /// HallucinatedTool 时是否自动追加教训
    pub auto_lesson_on_hallucination: bool,
    /// 是否将工具调用成功写入程序记忆
    pub record_tool_success: bool,
}

impl ContextManager {
    /// 创建新的上下文管理器
    pub fn new(max_turns: usize) -> Self {
        Self {
            conversation: ConversationMemory::new(max_turns),
            working: WorkingMemory::new(),
            long_term: None,
            lessons_path: None,
            procedural_path: None,
            preferences_path: None,
            auto_lesson_on_hallucination: true,
            record_tool_success: false,
        }
    }

    /// 设置长期记忆
    pub fn with_long_term(mut self, long_term: Arc<dyn LongTermMemory>) -> Self {
        self.long_term = Some(long_term);
        self
    }

    /// 设置教训文件路径
    pub fn with_lessons_path(mut self, path: PathBuf) -> Self {
        self.lessons_path = Some(path);
        self
    }

    /// 设置程序记忆文件路径
    pub fn with_procedural_path(mut self, path: PathBuf) -> Self {
        self.procedural_path = Some(path);
        self
    }

    /// 设置用户偏好文件路径
    pub fn with_preferences_path(mut self, path: PathBuf) -> Self {
        self.preferences_path = Some(path);
        self
    }

    /// 设置是否自动追加教训
    pub fn with_auto_lesson_on_hallucination(mut self, enabled: bool) -> Self {
        self.auto_lesson_on_hallucination = enabled;
        self
    }

    /// 设置是否记录工具成功
    pub fn with_record_tool_success(mut self, enabled: bool) -> Self {
        self.record_tool_success = enabled;
        self
    }

    /// 获取消息列表
    pub fn messages(&self) -> &[Message] {
        self.conversation.messages()
    }

    /// 推送消息
    pub fn push_message(&mut self, msg: Message) {
        self.conversation.push(msg);
    }

    /// 设置消息列表
    pub fn set_messages(&mut self, messages: Vec<Message>) {
        self.conversation.set_messages(messages);
    }

    /// 转换为 LLM 消息
    pub fn to_llm_messages(&self) -> Vec<Message> {
        self.conversation.messages().to_vec()
    }

    /// 构建工作记忆段落
    pub fn working_memory_section(&self) -> String {
        self.working.to_prompt_section()
    }

    /// 构建长期记忆段落
    pub fn long_term_section(&self, query: &str) -> String {
        let Some(ref lt) = self.long_term else {
            return String::new();
        };
        if !lt.enabled() {
            return String::new();
        }
        let hits = lt.search(query, 5);
        if hits.is_empty() {
            return String::new();
        }
        let block = hits.join("\n\n");
        format!("## Relevant Past Knowledge\n{block}")
    }

    /// 推送内容到长期记忆
    pub fn push_to_long_term(&self, text: &str) {
        if let Some(ref lt) = self.long_term {
            lt.add(text);
        }
    }

    /// 构建教训段落
    pub fn lessons_section(&self) -> String {
        let Some(ref p) = self.lessons_path else {
            return String::new();
        };
        let s = load_lessons(p);
        if s.is_empty() {
            return String::new();
        }
        format!("\n## 行为约束 / Lessons（请遵守）\n{}\n", s)
    }

    /// 构建程序记忆段落
    pub fn procedural_section(&self) -> String {
        let Some(ref p) = self.procedural_path else {
            return String::new();
        };
        let s = load_procedural(p);
        if s.is_empty() {
            return String::new();
        }
        format!(
            "\n## 程序记忆 / 工具使用经验（请参考，避免重复失败）\n{}\n",
            s
        )
    }

    /// 构建用户偏好段落
    pub fn preferences_section(&self) -> String {
        let Some(ref p) = self.preferences_path else {
            return String::new();
        };
        let s = load_preferences(p);
        if s.is_empty() {
            return String::new();
        }
        format!("\n## 用户偏好 / Preferences（请遵守）\n{}\n", s)
    }

    /// 记录用户偏好
    pub fn append_preference(&self, content: &str) {
        if let Some(ref p) = self.preferences_path {
            let _ = append_preference(p, content);
        }
    }

    /// 追加 Critic 教训
    pub fn append_critic_lesson(&self, suggestion: &str) {
        if suggestion.trim().is_empty() {
            return;
        }
        let Some(ref p) = self.lessons_path else {
            return;
        };
        let line = format!("Critic 建议：{}", suggestion.trim());
        let _ = append_lesson(p, &line);
    }

    /// 追加幻觉教训
    pub fn append_hallucination_lesson(&self, hallucinated_tool: &str, valid_tools: &[String]) {
        if !self.auto_lesson_on_hallucination {
            return;
        }
        let Some(ref p) = self.lessons_path else {
            return;
        };
        let tools_list = valid_tools.join(", ");
        let line = format!(
            "仅使用以下已注册工具：{}；不要编造不存在的工具名（例如曾误用「{}」）。",
            tools_list, hallucinated_tool
        );
        let _ = append_lesson(p, &line);
    }

    /// 记录工具调用结果
    pub fn append_procedural_record(&self, tool: &str, success: bool, detail: &str) {
        if let Some(ref p) = self.procedural_path {
            let _ = append_procedural(p, tool, success, detail);
        }
    }

    /// 推送会话策略到长期记忆
    pub fn push_session_strategy_to_long_term(&self, goal: &str, tool_names: &[String]) {
        if tool_names.is_empty() {
            return;
        }
        let tools = tool_names.join(", ");
        let line = format!(
            "Session strategy: goal \"{}\"; tools used: {}.",
            goal.trim(),
            tools
        );
        self.push_to_long_term(&line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_manager_new() {
        let ctx = ContextManager::new(10);
        assert!(ctx.messages().is_empty());
        assert!(ctx.long_term.is_none());
    }

    #[test]
    fn test_context_manager_push_message() {
        let mut ctx = ContextManager::new(10);
        ctx.push_message(Message::user("Hello"));
        ctx.push_message(Message::assistant("Hi there!"));
        assert_eq!(ctx.messages().len(), 2);
    }

    #[test]
    fn test_context_manager_working_memory() {
        let mut ctx = ContextManager::new(10);
        ctx.working.set_goal("Find files");
        ctx.working.add_attempt("ls -> directory listing");
        let section = ctx.working_memory_section();
        assert!(section.contains("Find files"));
        assert!(section.contains("ls"));
    }
}
