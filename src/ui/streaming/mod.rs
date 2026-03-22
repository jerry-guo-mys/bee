//! 流式输出管线
//!
//! Phase 2 实现目标：
//! - MarkdownStreamCollector: newline-gated Markdown 累积
//! - StreamState: 队列压力管理
//! - StreamController: 流生命周期控制

mod collector;
mod controller;
mod state;

pub use collector::MarkdownStreamCollector;
pub use controller::{StreamConfig, StreamController, StreamStatus};
pub use state::{QueuedLine, StreamState};
