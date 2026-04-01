//! Kafka 事件总线实现

use super::EventBus;
use crate::domain::event::EventEnvelope;
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
};
use std::error::Error;
use std::fmt;
use std::time::Duration;

/// Kafka 事件总线错误
#[derive(Debug)]
pub enum KafkaEventBusError {
    KafkaConfigError(String),
    KafkaSendError(rdkafka::error::KafkaError),
    SerializationError(serde_json::Error),
}

impl fmt::Display for KafkaEventBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KafkaConfigError(msg) => write!(f, "Kafka config error: {}", msg),
            Self::KafkaSendError(e) => write!(f, "Kafka send error: {}", e),
            Self::SerializationError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl Error for KafkaEventBusError {}

impl From<rdkafka::error::KafkaError> for KafkaEventBusError {
    fn from(e: rdkafka::error::KafkaError) -> Self {
        Self::KafkaSendError(e)
    }
}

impl From<serde_json::Error> for KafkaEventBusError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e)
    }
}

/// Kafka 事件总线
pub struct KafkaEventBus {
    producer: FutureProducer,
    domain_events_topic: String,
    app_events_topic: String,
}

impl KafkaEventBus {
    /// 创建 Kafka 事件总线
    pub fn new(
        brokers: &str,
        domain_events_topic: &str,
        app_events_topic: &str,
    ) -> Result<Self, KafkaEventBusError> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| KafkaEventBusError::KafkaConfigError(e.to_string()))?;

        Ok(Self {
            producer,
            domain_events_topic: domain_events_topic.to_string(),
            app_events_topic: app_events_topic.to_string(),
        })
    }

    /// 从环境变量创建
    pub fn from_env() -> Result<Self, KafkaEventBusError> {
        let brokers =
            std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());
        let domain_topic = std::env::var("KAFKA_DOMAIN_EVENTS_TOPIC")
            .unwrap_or_else(|_| "bee.domain.events".to_string());
        let app_topic = std::env::var("KAFKA_APP_EVENTS_TOPIC")
            .unwrap_or_else(|_| "bee.app.events".to_string());

        Self::new(&brokers, &domain_topic, &app_topic)
    }

    /// 获取主题名称（根据事件类型路由）
    fn topic_for(&self, event_type: &str) -> &str {
        if event_type.starts_with("domain.") {
            &self.domain_events_topic
        } else {
            &self.app_events_topic
        }
    }
}

#[async_trait::async_trait]
impl EventBus for KafkaEventBus {
    type Error = KafkaEventBusError;

    async fn publish(&self, envelope: EventEnvelope) -> Result<(), Self::Error> {
        let topic = self.topic_for(&envelope.event_type);

        let key = envelope.aggregate_id.clone();
        let value = serde_json::to_string(&envelope)?;

        self.producer
            .send(
                FutureRecord::to(topic)
                    .key(&key)
                    .payload(&value)
                    .timestamp(chrono::Utc::now().timestamp_millis()),
                Timeout::After(Duration::from_secs(5)),
            )
            .await
            .map_err(|(e, _)| KafkaEventBusError::KafkaSendError(e))?;

        Ok(())
    }

    async fn publish_batch(&self, envelopes: Vec<EventEnvelope>) -> Result<(), Self::Error> {
        use futures_util::future::join_all;

        let futures = envelopes.into_iter().map(|envelope| self.publish(envelope));
        let results = join_all(futures).await;

        // 返回第一个错误
        for result in results {
            result?;
        }

        Ok(())
    }

    async fn close(&self) -> Result<(), Self::Error> {
        // rdskafka 的 FutureProducer 在 drop 时会自动关闭
        Ok(())
    }
}
