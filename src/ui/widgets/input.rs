//! Input area widget

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::core::UiState;
use crate::ui::theme;

use super::Renderable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputFocus {
    #[default]
    Input,
    Agent,
    Model,
    Send,
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub focus: InputFocus,
    pub agent_index: usize,
    pub model_index: usize,
    pub cursor_visible: bool,
    pub cursor_timer: u16,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            focus: InputFocus::Input,
            agent_index: 0,
            model_index: 0,
            cursor_visible: true,
            cursor_timer: 0,
        }
    }
}

impl InputState {
    pub fn update_cursor(&mut self) {
        self.cursor_timer = self.cursor_timer.wrapping_add(1);
        if self.cursor_timer.is_multiple_of(10) {
            self.cursor_visible = !self.cursor_visible;
        }
    }
}

pub struct InputArea<'a> {
    pub state: &'a UiState,
    pub input_buffer: &'a str,
    pub input_state: &'a InputState,
    pub agents: &'a [&'a str],
    pub models: &'a [&'a str],
}

impl<'a> InputArea<'a> {
    fn is_send_disabled(&self) -> bool {
        self.input_buffer.trim().is_empty() || self.state.input_locked
    }

    fn pill(label: String, active: bool, accent: ratatui::style::Color) -> Span<'static> {
        let _ = accent;
        let style = if active {
            Style::default().fg(theme::UI_TEXT)
        } else {
            Style::default().fg(theme::UI_TEXT_DIM)
        };
        Span::styled(format!("{label}"), style)
    }
}

impl<'a> Renderable for InputArea<'a> {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        let text_rect = chunks[0];
        let meta_rect = chunks[1];
        let divider_rect = chunks[2];

        for y in text_rect.y..text_rect.y.saturating_add(text_rect.height) {
            for x in text_rect.x..text_rect.x.saturating_add(text_rect.width) {
                buf[(x, y)].set_style(Style::default().bg(theme::PANEL_BG_SOFT));
            }
        }

        let show_cursor =
            self.input_state.focus == InputFocus::Input && self.input_state.cursor_visible;
        let spans = if self.input_buffer.is_empty() {
            let placeholder = "Describe the task, code change, or debugging goal";
            let mut chars = placeholder.chars();
            let first = chars.next().unwrap_or(' ');
            let rest: String = chars.collect();
            vec![
                if show_cursor {
                    Span::styled(
                        first.to_string(),
                        Style::default()
                            .fg(theme::APP_BG)
                            .bg(theme::ACCENT_CYAN)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(first.to_string(), Style::default().fg(theme::TEXT_SUBTLE))
                },
                Span::styled(
                    rest,
                    Style::default().fg(theme::TEXT_SUBTLE),
                ),
            ]
        } else {
            let mut spans = vec![Span::styled(
                self.input_buffer.to_string(),
                Style::default().fg(theme::TEXT_PRIMARY),
            )];
            if show_cursor {
                spans.push(Span::styled("█", Style::default().fg(theme::ACCENT_CYAN)));
            }
            spans
        };

        let centered_text_rect = Rect::new(
            text_rect.x.saturating_add(1),
            text_rect.y.saturating_add(1),
            text_rect.width.saturating_sub(1),
            1,
        );

        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(theme::PANEL_BG_SOFT))
            .wrap(Wrap { trim: false })
            .render(centered_text_rect, buf);

        let agent_idx = self
            .input_state
            .agent_index
            .min(self.agents.len().saturating_sub(1));
        let model_idx = self
            .input_state
            .model_index
            .min(self.models.len().saturating_sub(1));
        let agent = self.agents.get(agent_idx).copied().unwrap_or("default");
        let model = self.models.get(model_idx).copied().unwrap_or("default");

        let send_style = if self.is_send_disabled() {
            Style::default().fg(theme::UI_TEXT_DIM)
        } else if self.input_state.focus == InputFocus::Send {
            Style::default().fg(theme::UI_TEXT)
        } else {
            Style::default().fg(theme::UI_TEXT_DIM)
        };

        let meta = Line::from(vec![
            Self::pill(
                format!("agent {agent}"),
                self.input_state.focus == InputFocus::Agent,
                theme::ACCENT_BLUE,
            ),
            Span::raw("  "),
            Self::pill(
                format!("model {model}"),
                self.input_state.focus == InputFocus::Model,
                theme::ACCENT_PURPLE,
            ),
            Span::raw("  "),
            Span::styled("enter send", send_style),
            Span::raw("  "),
            Span::styled("tab focus", Style::default().fg(theme::UI_TEXT_DIM)),
        ]);

        Paragraph::new(meta)
            .style(theme::fill_style())
            .render(meta_rect, buf);

        Paragraph::new(Line::from(Span::styled(
            theme::muted_line(divider_rect.width),
            Style::default().fg(theme::DIVIDER),
        )))
        .style(theme::fill_style())
        .render(divider_rect, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        5
    }
}
