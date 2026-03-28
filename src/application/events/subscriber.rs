use async_trait::async_trait;
use std::error::Error;

/// 事件处理器
#[async_trait]
pub trait EventHandler<E: Send + Sync>: Send + Sync {
    type Error: Error + Send + Sync;

    async fn handle(&self, event: E) -> Result<(), Self::Error>;
}

/// 事件订阅者 trait
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    type Error: Error + Send + Sync;

    async fn subscribe(&self, event_type: &str) -> Result<(), Self::Error>;
}
