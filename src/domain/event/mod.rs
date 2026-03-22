//! 领域事件：业务事件定义与事件总线

pub mod bus;
pub mod events;

pub use bus::EventBus;
pub use events::DomainEvent;
