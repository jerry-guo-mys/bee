//! Input area widget

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
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
    pub cursor_byte: usize,
    pub preferred_column: Option<usize>,
    pub scroll_line: usize,
    pub cursor_visible: bool,
    pub cursor_timer: u16,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            focus: InputFocus::Input,
            agent_index: 0,
            model_index: 0,
            cursor_byte: 0,
            preferred_column: None,
            scroll_line: 0,
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

    fn label(label: String, active: bool) -> Span<'static> {
        let style = if active {
            Style::default().fg(theme::TEXT_PRIMARY)
        } else {
            Style::default().fg(theme::UI_TEXT_DIM)
        };
        Span::styled(label, style)
    }

    fn footer_hint(&self) -> Line<'static> {
        let agent_idx = self
            .input_state
            .agent_index
            .min(self.agents.len().saturating_sub(1));
        let model_idx = self
            .input_state
            .model_index
            .min(self.models.len().saturating_sub(1));
        let agent = self
            .agents
            .get(agent_idx)
            .copied()
            .or_else(|| self.agents.first().copied())
            .unwrap_or("");
        let model = self
            .models
            .get(model_idx)
            .copied()
            .or_else(|| self.models.first().copied())
            .unwrap_or("");

        let send_text = if self.state.input_locked {
            "task running"
        } else if self.is_send_disabled() {
            "send disabled"
        } else {
            "enter send"
        };

        Line::from(vec![
            Self::label(
                agent.to_string(),
                self.input_state.focus == InputFocus::Agent,
            ),
            Span::raw(" · "),
            Self::label(
                model.to_string(),
                self.input_state.focus == InputFocus::Model,
            ),
            Span::raw(" · "),
            Self::label(
                send_text.to_string(),
                self.input_state.focus == InputFocus::Send,
            ),
            Span::raw(" · "),
            Span::styled(
                "shift+enter newline",
                Style::default().fg(theme::UI_TEXT_DIM),
            ),
        ])
    }

    fn mention_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
        let mut ranges = Vec::new();
        let mut token_start = None;

        for (idx, ch) in text.char_indices() {
            if ch.is_whitespace() {
                if let Some(start) = token_start.take() {
                    if text[start..idx].starts_with('@') && idx.saturating_sub(start) > 1 {
                        ranges.push(start..idx);
                    }
                }
                continue;
            }

            if token_start.is_none() {
                token_start = Some(idx);
            }
        }

        if let Some(start) = token_start {
            if text[start..].starts_with('@') && text.len().saturating_sub(start) > 1 {
                ranges.push(start..text.len());
            }
        }

        ranges
    }

    fn display_lines(&self, width: usize) -> (Vec<Line<'static>>, usize) {
        let show_cursor =
            self.input_state.focus == InputFocus::Input && self.input_state.cursor_visible;

        if self.input_buffer.is_empty() {
            let placeholder = "Describe the task, code change, or debugging goal";
            let mut chars = placeholder.chars();
            let first = chars.next().unwrap_or(' ');
            let rest: String = chars.collect();

            return (
                vec![Line::from(vec![
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
                    Span::styled(rest, Style::default().fg(theme::TEXT_SUBTLE)),
                ])],
                0,
            );
        }

        let cursor = self.input_state.cursor_byte.min(self.input_buffer.len());
        let mention_ranges = Self::mention_ranges(self.input_buffer);
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut current_spans: Vec<Span<'static>> = Vec::new();
        let mut current_width = 0usize;
        let mut cursor_line = 0usize;

        let push_cursor = |spans: &mut Vec<Span<'static>>, width_count: &mut usize| {
            spans.push(Span::styled("█", Style::default().fg(theme::ACCENT_CYAN)));
            *width_count += 1;
        };

        let flush_line = |lines: &mut Vec<Line<'static>>,
                          spans: &mut Vec<Span<'static>>,
                          width_count: &mut usize| {
            lines.push(Line::from(std::mem::take(spans)));
            *width_count = 0;
        };

        for (idx, ch) in self.input_buffer.char_indices() {
            if show_cursor && idx == cursor {
                if current_width >= width.max(1) {
                    flush_line(&mut lines, &mut current_spans, &mut current_width);
                }
                cursor_line = lines.len();
                push_cursor(&mut current_spans, &mut current_width);
            }

            if ch == '\n' {
                flush_line(&mut lines, &mut current_spans, &mut current_width);
                continue;
            }

            if current_width >= width.max(1) {
                flush_line(&mut lines, &mut current_spans, &mut current_width);
            }

            let in_mention = mention_ranges
                .iter()
                .any(|range| range.start <= idx && idx < range.end);
            let style = if in_mention {
                Style::default().fg(theme::ACCENT_CYAN).bg(theme::APP_BG)
            } else {
                Style::default().fg(theme::TEXT_PRIMARY)
            };
            current_spans.push(Span::styled(ch.to_string(), style));
            current_width += 1;
        }

        if show_cursor && cursor == self.input_buffer.len() {
            if current_width >= width.max(1) {
                flush_line(&mut lines, &mut current_spans, &mut current_width);
            }
            cursor_line = lines.len();
            push_cursor(&mut current_spans, &mut current_width);
        }

        if !current_spans.is_empty() {
            lines.push(Line::from(current_spans));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::raw("")));
        }

        (lines, cursor_line)
    }
}

impl<'a> Renderable for InputArea<'a> {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let footer_rows = 1u16;
        let text_height = area.height.saturating_sub(footer_rows);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(text_height), Constraint::Length(1)])
            .split(area);

        let text_rect = chunks[0];
        let footer_rect = chunks[1];

        for y in text_rect.y..text_rect.y.saturating_add(text_rect.height) {
            for x in text_rect.x..text_rect.x.saturating_add(text_rect.width) {
                buf[(x, y)].set_style(Style::default().bg(theme::PANEL_BG_SOFT));
            }
        }

        let inner_width = text_rect.width.saturating_sub(1) as usize;
        let (all_lines, cursor_line) = self.display_lines(inner_width.max(1));
        let max_visible = text_rect.height as usize;
        let max_scroll = all_lines.len().saturating_sub(max_visible);
        let desired_scroll = self.input_state.scroll_line.min(max_scroll);
        let cursor_scroll = cursor_line.saturating_sub(max_visible.saturating_sub(1));
        let visible_start = desired_scroll.max(cursor_scroll).min(max_scroll);
        let visible_end = (visible_start + max_visible).min(all_lines.len());
        let visible_lines = all_lines[visible_start..visible_end].to_vec();
        let vertical_padding = text_rect.height.saturating_sub(visible_lines.len() as u16) / 2;

        let draw_rect = Rect::new(
            text_rect.x.saturating_add(1),
            text_rect.y.saturating_add(vertical_padding),
            text_rect.width.saturating_sub(1),
            visible_lines.len().min(text_rect.height as usize) as u16,
        );

        Paragraph::new(Text::from(visible_lines))
            .style(Style::default().bg(theme::PANEL_BG_SOFT))
            .wrap(Wrap { trim: false })
            .render(draw_rect, buf);

        Paragraph::new(self.footer_hint())
            .style(theme::fill_style())
            .render(footer_rect, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        let text_lines = self
            .display_lines(width.saturating_sub(2) as usize)
            .0
            .len()
            .clamp(1, 4) as u16;
        text_lines.max(3) + 1
    }
}
