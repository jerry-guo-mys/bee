//! 事件总线实现（向后兼容）

use super::events::LegacyDomainEvent;
use tokio::sync::broadcast;

/// 事件总线（向后兼容）
pub struct EventBus {
    sender: broadcast::Sender<LegacyDomainEvent>,
}

impl EventBus {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(buffer_size);
        Self { sender }
    }

    /// 发布事件
    pub fn publish(&self, event: LegacyDomainEvent) {
        let _ = self.sender.send(event);
    }

    /// 订阅事件
    pub fn subscribe(&self) -> broadcast::Receiver<LegacyDomainEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(100)
    }
}
