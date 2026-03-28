//! 事件总线基础设施实现

use crate::domain::event::EventEnvelope;
use std::error::Error;

/// 事件总线 trait
#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    type Error: Error + Send + Sync;

    /// 发布事件
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), Self::Error>;

    /// 批量发布事件
    async fn publish_batch(
        &self,
        envelopes: Vec<EventEnvelope>,
    ) -> Result<(), Self::Error> {
        for envelope in envelopes {
            self.publish(envelope).await?;
        }
        Ok(())
    }

    /// 关闭连接
    async fn close(&self) -> Result<(), Self::Error>;
}

pub mod in_memory;
pub mod kafka;
