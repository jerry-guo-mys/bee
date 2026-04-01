//! 领域事件：业务事件定义与事件总线

pub mod bus;
pub mod events;

pub use bus::EventBus;
pub use events::{DomainEvent, EventEnvelope, EventMetadata, LegacyDomainEvent};
// 重新导出 DomainEventPublisher 和 InMemoryEventPublisher（从 tenant 模块）
pub use crate::domain::tenant::event::{DomainEventPublisher, InMemoryEventPublisher};
