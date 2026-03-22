//! 对话记忆

use crate::memory::Message;

/// 对话记忆
#[derive(Debug, Clone, Default)]
pub struct ConversationMemory {
    messages: Vec<Message>,
    max_turns: usize,
}

impl ConversationMemory {
    pub fn new(max_turns: usize) -> Self {
        Self {
            messages: vec![],
            max_turns,
        }
    }

    /// 推送消息
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
        // 如果超过最大轮数，移除最早的消息
        while self.messages.len() > self.max_turns {
            self.messages.remove(0);
        }
    }

    /// 获取消息列表
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// 设置消息列表
    pub fn set_messages(&mut self, messages: Vec<Message>) {
        self.messages = messages;
    }

    /// 清空消息
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}
