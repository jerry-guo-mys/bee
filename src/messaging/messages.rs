//! 应用消息类型

use crate::application::event_bus::AppEvent;
use crate::react::ReactEvent;

/// 应用层消息 - 统一内部通信
#[derive(Clone)]
pub enum AppMessage {
    /// 用户提交消息
    SubmitMessage {
        conversation_id: String,
        content: String,
    },
    /// 取消当前操作
    Cancel,
    /// 清空对话
    Clear { conversation_id: String },
    /// 切换助手
    SwitchAssistant { assistant_id: String },
    /// 切换模型
    SwitchModel { model_id: String },
    /// 配置变更
    ConfigChange { key: String, value: String },
    /// 事件通知（来自事件总线）
    Event(AppEvent),
    /// ReAct 事件（来自认知循环）
    ReactEvent(ReactEvent),
    /// Token 流式片段
    TokenChunk { token: String },
    /// 错误通知
    Error { message: String },
}
