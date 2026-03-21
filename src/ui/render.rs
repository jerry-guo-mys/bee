//! 生产级 TUI 渲染布局

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Widget},
    Frame,
};

use crate::core::{AgentPhase, UiState};

use super::markdown::MarkdownRenderer;
use super::streaming::StreamController;
use super::theme;
use super::widgets::{
    CommandPopup, ConversationView, InputArea, InputState, Renderable, StatusIndicator,
};

pub struct RenderContext {
    pub markdown_renderer: MarkdownRenderer,
    pub stream_controller: Option<StreamController>,
    pub status_indicator: StatusIndicator,
}

impl RenderContext {
    pub fn new() -> Self {
        Self {
            markdown_renderer: MarkdownRenderer::new(80),
            stream_controller: None,
            status_indicator: StatusIndicator::default(),
        }
    }

    pub fn update_status(&mut self, phase: &AgentPhase, elapsed_secs: u64) {
        use std::time::Duration;

        self.status_indicator.header = match phase {
            AgentPhase::Idle => "ready".to_string(),
            AgentPhase::Thinking => "thinking".to_string(),
            AgentPhase::Streaming => "streaming".to_string(),
            AgentPhase::ToolExecuting => "running tool".to_string(),
            AgentPhase::Responding => "finalizing".to_string(),
            AgentPhase::Error => "error".to_string(),
        };
        self.status_indicator.elapsed_running = Duration::from_secs(elapsed_secs);
    }
}

impl Default for RenderContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn draw(
    f: &mut Frame,
    state: &UiState,
    input_buffer: &str,
    conversation_scroll: usize,
    out: &mut (usize, usize),
    input_state: &InputState,
    agents: &[&str],
    models: &[&str],
    ctx: &mut RenderContext,
    command_popup: &mut CommandPopup,
) {
    let area = f.area();
    Block::default()
        .style(theme::fill_style())
        .render(area, f.buffer_mut());

    ctx.status_indicator.inline_message = state.active_tool.as_ref().map(|tool| format!("tool: {tool}"));
    ctx.status_indicator.details = state
        .error_message
        .as_ref()
        .map(|message| format!("error: {message}"));
    let status_height = ctx.status_indicator.desired_height(area.width).max(2);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(status_height),
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .margin(1)
        .split(area);

    ctx.status_indicator.render(root[0], f.buffer_mut());

    let content_width = root[1].width.saturating_sub(2) as usize;
    let content_height = root[1].height.saturating_sub(1) as usize;

    let conversation_area = Rect::new(root[1].x + 1, root[1].y, root[1].width.saturating_sub(2), root[1].height);
    let mut conv_view = ConversationView::new(
        state,
        conversation_scroll,
        content_width,
        content_height,
        &mut ctx.markdown_renderer,
    );
    conv_view.render(conversation_area, f.buffer_mut());
    let total_lines = conv_view.total_lines();

    let mut input = InputArea {
        state,
        input_buffer,
        input_state,
        agents,
        models,
    };
    input.render(root[2], f.buffer_mut());

    if command_popup.is_visible() {
        let popup_height = command_popup.display_height() as u16 + 1;
        let popup_area = Rect::new(
            root[2].x + 2,
            root[2].y.saturating_sub(popup_height),
            root[2].width.min(52),
            popup_height,
        );
        command_popup.render(popup_area, f.buffer_mut());
    }

    out.0 = total_lines;
    out.1 = content_height;
}
