use crate::domain::event::{DomainEvent, EventEnvelope};
use crate::infrastructure::event_bus::EventBus;
use async_trait::async_trait;
use std::sync::Arc;

/// 事件发布器错误
#[derive(Debug)]
pub enum EventPublisherError {
    EnvelopeCreationError(String),
    PublishError(String),
}

impl std::fmt::Display for EventPublisherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventPublisherError::EnvelopeCreationError(e) => write!(f, "Envelope creation error: {}", e),
            EventPublisherError::PublishError(e) => write!(f, "Publish error: {}", e),
        }
    }
}

impl std::error::Error for EventPublisherError {}

/// 事件发布器
#[async_trait]
pub trait EventPublisher: Send + Sync {
    type Error: Send + Sync;

    async fn publish<E: DomainEvent>(&self, event: &E) -> Result<(), Self::Error>;

    async fn publish_batch<E: DomainEvent>(&self, events: &[E]) -> Result<(), Self::Error> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

/// 基于 EventBus 的实现
pub struct EventBusPublisher<EB: EventBus> {
    event_bus: Arc<EB>,
}

impl<EB: EventBus + 'static> EventBusPublisher<EB> {
    pub fn new(event_bus: Arc<EB>) -> Self {
        Self { event_bus }
    }
}

#[async_trait]
impl<EB: EventBus + 'static> EventPublisher for EventBusPublisher<EB> {
    type Error = EventPublisherError;

    async fn publish<E: DomainEvent>(&self, event: &E) -> Result<(), Self::Error> {
        let envelope = EventEnvelope::new(event)
            .map_err(|e| EventPublisherError::EnvelopeCreationError(e.to_string()))?;

        self.event_bus.publish(envelope).await
            .map_err(|_| EventPublisherError::PublishError("Failed to publish event".to_string()))
    }
}
