//! 应用事件总线

use tokio::sync::broadcast;

/// 应用事件
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// 用户提交消息
    MessageSubmitted(String),
    /// Agent 开始思考
    ThinkingStarted,
    /// 工具执行
    ToolExecuted { name: String, success: bool },
    /// 响应完成
    ResponseCompleted(String),
    /// 错误发生
    Error(String),
}

/// 应用事件总线
pub struct AppEventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl AppEventBus {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(buffer_size);
        Self { sender }
    }

    /// 发布事件
    pub fn publish(&self, event: AppEvent) {
        if let Err(e) = self.sender.send(event) {
            tracing::debug!("Event bus publish failed (no subscribers): {:?}", e);
        }
    }

    /// 订阅事件
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }
}

impl Default for AppEventBus {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = AppEventBus::new(10);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::MessageSubmitted("test".to_string()));

        let event = rx.recv().await.unwrap();
        match event {
            AppEvent::MessageSubmitted(msg) => assert_eq!(msg, "test"),
            _ => panic!("Unexpected event"),
        }
    }
}
