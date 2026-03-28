//! 应用层：业务应用服务
//!
//! 负责用例编排，协调领域服务完成业务目标

pub mod agent_service;
pub mod event_bus;
pub mod events;
pub mod commands;
pub mod health;
pub mod orchestrator;
pub mod queries;
pub mod stream;
pub mod task_queue;

pub use agent_service::{AgentService, AgentServiceImpl};
pub use event_bus::AppEventBus;
pub use commands::{CqrsCommand, CommandBus, CommandHandler};
pub use events::{EventBusPublisher, EventPublisher, EventHandler};
pub use orchestrator::create_agent;
pub use queries::{CqrsQuery, QueryBus, QueryHandler};
pub use task_queue::{Priority, Task, TaskError, TaskQueue, TaskQueueBuilder};
