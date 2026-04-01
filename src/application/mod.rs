//! 应用层：业务应用服务
//!
//! 负责用例编排，协调领域服务完成业务目标

pub mod agent_service;
pub mod commands;
pub mod event_bus;
pub mod events;
pub mod health;
pub mod orchestrator;
pub mod queries;
pub mod stream;
pub mod task_queue;

pub use agent_service::{AgentService, AgentServiceImpl};
pub use commands::{CommandBus, CommandHandler, CqrsCommand};
pub use event_bus::AppEventBus;
pub use events::{EventBusPublisher, EventHandler, EventPublisher};
pub use orchestrator::create_agent;
pub use queries::{CqrsQuery, QueryBus, QueryHandler};
pub use task_queue::{Priority, Task, TaskError, TaskQueue, TaskQueueBuilder};
