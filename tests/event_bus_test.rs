//! 事件总线集成测试

use bee::domain::event::{DomainEvent, EventEnvelope};
use bee::infrastructure::event_bus::in_memory::InMemoryEventBus;
use bee::infrastructure::event_bus::EventBus;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct TestEvent {
    aggregate_id: Uuid,
    message: String,
}

impl DomainEvent for TestEvent {
    fn event_type(&self) -> &'static str {
        "domain.test"
    }

    fn aggregate_type(&self) -> &'static str {
        "TestAggregate"
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }
}

#[tokio::test]
async fn test_in_memory_event_bus_publish() {
    let bus = InMemoryEventBus::new();
    let test_id = Uuid::new_v4();

    let event = TestEvent {
        aggregate_id: test_id,
        message: "test message".to_string(),
    };

    let envelope = EventEnvelope::new(&event).unwrap();
    bus.publish(envelope).await.unwrap();

    let events = bus.get_events(&test_id.to_string());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "domain.test");
}

#[tokio::test]
async fn test_in_memory_event_bus_publish_batch() {
    let bus = InMemoryEventBus::new();
    let test_id = Uuid::new_v4();

    let events: Vec<EventEnvelope> = (0..3)
        .map(|i| {
            let event = TestEvent {
                aggregate_id: test_id,
                message: format!("message {}", i),
            };
            EventEnvelope::new(&event).unwrap()
        })
        .collect();

    bus.publish_batch(events).await.unwrap();

    let retrieved_events = bus.get_events(&test_id.to_string());
    assert_eq!(retrieved_events.len(), 3);
}

#[tokio::test]
async fn test_in_memory_event_bus_subscribe() {
    let bus = InMemoryEventBus::new();
    let test_id = Uuid::new_v4();
    let mut receiver = bus.subscribe();

    let event = TestEvent {
        aggregate_id: test_id,
        message: "test message".to_string(),
    };

    let envelope = EventEnvelope::new(&event).unwrap();
    bus.publish(envelope.clone()).await.unwrap();

    // 接收广播事件
    let received = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
        .await
        .expect("timeout")
        .expect("channel closed");

    assert_eq!(received.event_type, "domain.test");
    assert_eq!(received.aggregate_id, test_id.to_string());
}

#[tokio::test]
async fn test_event_envelope_metadata() {
    let bus = InMemoryEventBus::new();
    let test_id = Uuid::new_v4();

    let event = TestEvent {
        aggregate_id: test_id,
        message: "test message".to_string(),
    };

    let envelope = EventEnvelope::new(&event)
        .unwrap()
        .with_correlation_id("corr-123".to_string())
        .with_causation_id("cause-456".to_string())
        .with_user_id("user-789".to_string())
        .with_tenant_id("tenant-abc".to_string());

    bus.publish(envelope.clone()).await.unwrap();

    assert_eq!(
        envelope.metadata.correlation_id,
        Some("corr-123".to_string())
    );
    assert_eq!(
        envelope.metadata.causation_id,
        Some("cause-456".to_string())
    );
    assert_eq!(envelope.metadata.user_id, Some("user-789".to_string()));
    assert_eq!(envelope.metadata.tenant_id, Some("tenant-abc".to_string()));
}

#[tokio::test]
async fn test_in_memory_event_bus_clear() {
    let bus = InMemoryEventBus::new();
    let test_id = Uuid::new_v4();

    let event = TestEvent {
        aggregate_id: test_id,
        message: "test message".to_string(),
    };

    let envelope = EventEnvelope::new(&event).unwrap();
    bus.publish(envelope).await.unwrap();

    assert_eq!(bus.event_count(), 1);

    bus.clear();

    assert_eq!(bus.event_count(), 0);
}

#[tokio::test]
async fn test_event_envelope_creation() {
    let test_id = Uuid::new_v4();
    let event = TestEvent {
        aggregate_id: test_id,
        message: "test".to_string(),
    };

    let envelope = EventEnvelope::new(&event).unwrap();

    assert_eq!(envelope.aggregate_id, test_id.to_string());
    assert_eq!(envelope.event_type, "domain.test");
    assert_eq!(envelope.aggregate_type, "TestAggregate");
    assert!(!envelope.id.is_empty());
    assert!(envelope.occurred_at > Utc::now() - chrono::Duration::seconds(1));
}
