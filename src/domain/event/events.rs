//! 领域事件定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 领域事件 trait
pub trait DomainEvent: Send + Sync + Serialize {
    /// 事件类型名称
    fn event_type(&self) -> &'static str;

    /// 聚合根类型
    fn aggregate_type(&self) -> &'static str;

    /// 聚合根 ID
    fn aggregate_id(&self) -> Uuid;

    /// 事件发生时间
    fn occurred_at(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// 事件包用于 Kafka 传输
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: String,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
    pub metadata: EventMetadata,
}

/// 事件元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventMetadata {
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
}

impl EventEnvelope {
    /// 从领域事件创建事件包
    pub fn new<E: DomainEvent>(event: &E) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: event.event_type().to_string(),
            aggregate_type: event.aggregate_type().to_string(),
            aggregate_id: event.aggregate_id().to_string(),
            payload: serde_json::to_value(event)?,
            occurred_at: event.occurred_at(),
            metadata: EventMetadata::default(),
        })
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: EventMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// 设置关联 ID
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.metadata.correlation_id = Some(correlation_id);
        self
    }

    /// 设置因果 ID
    pub fn with_causation_id(mut self, causation_id: String) -> Self {
        self.metadata.causation_id = Some(causation_id);
        self
    }

    /// 设置用户 ID
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.metadata.user_id = Some(user_id);
        self
    }

    /// 设置租户 ID
    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.metadata.tenant_id = Some(tenant_id);
        self
    }
}

/// 向后兼容的旧版领域事件 enum
/// 用于现有的 bus.rs 模块
#[derive(Debug, Clone)]
pub enum LegacyDomainEvent {
    /// 会话创建
    SessionCreated(String),
    /// 会话完成
    SessionCompleted(String),
    /// 工具执行
    ToolExecuted { name: String, success: bool },
    /// 记忆更新
    MemoryUpdated(String),
    /// 错误发生
    Error(String),
    /// 自定义事件
    Custom(String),
}
