//! 内存事件总线实现（用于测试和开发）

use super::EventBus;
use crate::domain::event::EventEnvelope;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// 内存事件总线
pub struct InMemoryEventBus {
    events: Arc<DashMap<String, Vec<EventEnvelope>>>,
    broadcaster: broadcast::Sender<EventEnvelope>,
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            events: Arc::new(DashMap::new()),
            broadcaster: tx,
        }
    }
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取指定聚合根的事件
    pub fn get_events(&self, aggregate_id: &str) -> Vec<EventEnvelope> {
        self.events
            .get(aggregate_id)
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    /// 订阅所有事件
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.broadcaster.subscribe()
    }

    /// 清空所有事件（用于测试）
    pub fn clear(&self) {
        self.events.clear();
    }
}

#[async_trait::async_trait]
impl EventBus for InMemoryEventBus {
    type Error = std::convert::Infallible;

    async fn publish(&self, envelope: EventEnvelope) -> Result<(), Self::Error> {
        let aggregate_id = envelope.aggregate_id.clone();

        self.events
            .entry(aggregate_id)
            .or_insert_with(Vec::new)
            .push(envelope.clone());

        let _ = self.broadcaster.send(envelope);
        Ok(())
    }

    async fn publish_batch(
        &self,
        envelopes: Vec<EventEnvelope>,
    ) -> Result<(), Self::Error> {
        for envelope in envelopes {
            let aggregate_id = envelope.aggregate_id.clone();

            self.events
                .entry(aggregate_id)
                .or_insert_with(Vec::new)
                .push(envelope.clone());

            let _ = self.broadcaster.send(envelope);
        }
        Ok(())
    }

    async fn close(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(any(test, feature = "gateway"))]
impl InMemoryEventBus {
    /// 获取所有事件（用于测试）
    pub fn all_events(&self) -> Vec<EventEnvelope> {
        self.events
            .iter()
            .flat_map(|e| e.value().clone())
            .collect()
    }
}

impl InMemoryEventBus {
    /// 获取事件数量（用于测试）
    pub fn event_count(&self) -> usize {
        self.events.iter().map(|e| e.value().len()).sum()
    }
}
