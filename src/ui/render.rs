//! Production TUI layout rendering

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Widget},
    Frame,
};

use crate::core::{AgentPhase, UiState};

use super::app::LiveConversationState;
use super::markdown::MarkdownRenderer;
use super::streaming::StreamController;
use super::theme;
use super::widgets::{
    ActivityRail, CommandPopup, ConversationView, FilePopup, InputArea, InputState, Renderable,
    StatusIndicator,
};

const WIDE_LAYOUT_WIDTH: u16 = 124;

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

fn render_conversation_scrollbar(
    area: Rect,
    buf: &mut Buffer,
    scroll_offset: usize,
    total_lines: usize,
    viewport_height: usize,
) {
    if area.width == 0 || area.height == 0 || total_lines <= viewport_height || viewport_height == 0
    {
        return;
    }

    let x = area.x + area.width.saturating_sub(1);
    let track_height = area.height as usize;
    let thumb_height = ((viewport_height * track_height) / total_lines)
        .max(1)
        .min(track_height);
    let max_scroll = total_lines.saturating_sub(viewport_height).max(1);
    let thumb_travel = track_height.saturating_sub(thumb_height);
    let thumb_top = (scroll_offset.min(max_scroll) * thumb_travel) / max_scroll;

    for offset in 0..track_height {
        let y = area.y + offset as u16;
        let is_thumb = offset >= thumb_top && offset < thumb_top + thumb_height;
        let (symbol, style) = if is_thumb {
            ("█", Style::default().fg(theme::TEXT_SOFT).bg(theme::APP_BG))
        } else {
            (
                "│",
                Style::default().fg(theme::TEXT_SUBTLE).bg(theme::APP_BG),
            )
        };
        buf[(x, y)].set_symbol(symbol).set_style(style);
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
    live_state: &LiveConversationState,
    command_popup: &mut CommandPopup,
    file_popup: &mut FilePopup,
) {
    let area = f.area();
    Block::default()
        .style(theme::fill_style())
        .render(area, f.buffer_mut());

    if state.input_locked {
        ctx.status_indicator.resume();
        ctx.status_indicator.header = "working".to_string();
    } else {
        ctx.status_indicator.pause();
        ctx.status_indicator.header = if state.error_message.is_some() {
            "error".to_string()
        } else {
            "ready".to_string()
        };
    }
    ctx.status_indicator.update_elapsed();
    ctx.status_indicator.inline_message = live_state.process_lines.last().cloned().or_else(|| {
        state
            .active_tool
            .as_ref()
            .map(|tool| format!("tool {tool}"))
    });
    ctx.status_indicator.details = None;
    ctx.status_indicator.details_max_lines = 0;

    let input = InputArea {
        state,
        input_buffer,
        input_state,
        agents,
        models,
    };
    let composer_height = input.desired_height(area.width).clamp(4, 7);

    let status_height = if state.input_locked { 1 } else { 0 };

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(status_height),
            Constraint::Length(composer_height),
        ])
        .margin(1)
        .split(area);

    let (conversation_container, rail_area) = if root[0].width >= WIDE_LAYOUT_WIDTH {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
            .spacing(1)
            .split(root[0]);
        (cols[0], Some(cols[1]))
    } else {
        (root[0], None)
    };

    let content_width = conversation_container.width.saturating_sub(2) as usize;
    let content_height = conversation_container.height as usize;
    let conversation_area = Rect::new(
        conversation_container.x + 1,
        conversation_container.y,
        conversation_container.width.saturating_sub(2),
        conversation_container.height,
    );

    let mut conv_view = ConversationView::new(
        state,
        conversation_scroll,
        content_width,
        content_height,
        &mut ctx.markdown_renderer,
        live_state,
    );
    conv_view.render(conversation_area, f.buffer_mut());
    let total_lines = conv_view.total_lines();
    render_conversation_scrollbar(
        conversation_area,
        f.buffer_mut(),
        conversation_scroll,
        total_lines,
        content_height,
    );

    if let Some(rail_area) = rail_area {
        let mut rail = ActivityRail::new(state);
        rail.render(rail_area, f.buffer_mut());
    }

    if status_height > 0 {
        ctx.status_indicator.render(root[1], f.buffer_mut());
    }

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

    if file_popup.is_visible() {
        let popup_height = file_popup.display_height() as u16 + 1;
        let popup_area = Rect::new(
            root[2].x + 2,
            root[2].y.saturating_sub(popup_height),
            root[2].width.min(64),
            popup_height,
        );
        file_popup.render(popup_area, f.buffer_mut());
    }

    out.0 = total_lines;
    out.1 = content_height;
}
