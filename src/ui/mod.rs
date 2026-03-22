//! TUI 层：Ratatui + crossterm，主循环（app）、事件（event）、渲染（render）
//!
//! 组件化架构：
//! - widgets: 可复用组件（ConversationView, InputArea, StatusIndicator）
//! - streaming: 流式输出管线（Phase 2）
//! - markdown: Markdown 渲染（Phase 3）

pub mod app;
pub mod event;
pub mod markdown;
pub mod render;
pub mod streaming;
pub mod theme;
pub mod widgets;

pub use app::run_app;
pub use event::EventHandler;
pub use render::draw;
pub use widgets::{ConversationView, InputArea, InputState, Renderable, StatusIndicator};
