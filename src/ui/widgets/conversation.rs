//! 对话历史渲染组件

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget},
};
use unicode_width::UnicodeWidthChar;

use crate::core::UiState;
use crate::memory::Role;
use crate::ui::app::LiveConversationState;
use crate::ui::markdown::MarkdownRenderer;
use crate::ui::theme;
use crate::ui::Renderable;

const MAX_DISPLAY_CHARS: usize = 600;
const MAX_TOOL_DISPLAY_CHARS: usize = 280;
const COMPACT_EVENT_LINES: usize = 2;

pub struct ConversationView<'a> {
    pub state: &'a UiState,
    pub scroll_offset: usize,
    pub content_width: usize,
    pub content_height: usize,
    pub markdown_renderer: &'a mut MarkdownRenderer,
    pub live_state: &'a LiveConversationState,
}

impl<'a> ConversationView<'a> {
    pub fn new(
        state: &'a UiState,
        scroll_offset: usize,
        content_width: usize,
        content_height: usize,
        markdown_renderer: &'a mut MarkdownRenderer,
        live_state: &'a LiveConversationState,
    ) -> Self {
        Self {
            state,
            scroll_offset,
            content_width,
            content_height,
            markdown_renderer,
            live_state,
        }
    }

    fn is_tool_result(content: &str) -> bool {
        content.starts_with("Tool call:") || content.starts_with("Observation from ")
    }

    fn truncate_for_display(content: &str) -> String {
        let limit = if Self::is_tool_result(content) {
            MAX_TOOL_DISPLAY_CHARS
        } else {
            MAX_DISPLAY_CHARS
        };
        let chars: Vec<char> = content.chars().collect();
        if chars.len() <= limit {
            return content.to_string();
        }
        let head: String = chars.iter().take(limit).collect();
        format!("{}\n... [truncated, {} chars total]", head, chars.len())
    }

    fn wrap_text(s: &str, width: usize) -> Vec<String> {
        if width == 0 {
            return vec![s.to_string()];
        }

        let mut lines = Vec::new();
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split_whitespace() {
                let word_len = word.chars().count();
                let line_len = line.chars().count();

                if line.is_empty() {
                    if word_len <= width {
                        line.push_str(word);
                    } else {
                        let mut chunk = String::new();
                        for ch in word.chars() {
                            if chunk.chars().count() >= width {
                                lines.push(std::mem::take(&mut chunk));
                            }
                            chunk.push(ch);
                        }
                        if !chunk.is_empty() {
                            line = chunk;
                        }
                    }
                    continue;
                }

                if line_len + 1 + word_len <= width {
                    line.push(' ');
                    line.push_str(word);
                } else {
                    lines.push(std::mem::take(&mut line));
                    if word_len <= width {
                        line.push_str(word);
                    } else {
                        let mut chunk = String::new();
                        for ch in word.chars() {
                            if chunk.chars().count() >= width {
                                lines.push(std::mem::take(&mut chunk));
                            }
                            chunk.push(ch);
                        }
                        line = chunk;
                    }
                }
            }

            if !line.is_empty() {
                lines.push(line);
            } else if para.is_empty() {
                lines.push(String::new());
            }
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    fn meta_line(role: &Role, idx: usize, chars: usize) -> Line<'static> {
        let role_name = match role {
            Role::User => "you",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        };
        Line::from(vec![
            Span::styled(format!("{role_name} "), Style::default().fg(theme::UI_TEXT)),
            Span::styled(
                format!("#{idx:02}"),
                Style::default().fg(theme::UI_TEXT_DIM),
            ),
            Span::styled(
                format!(" · {chars} chars"),
                Style::default().fg(theme::TEXT_SUBTLE),
            ),
        ])
    }

    fn assistant_line(line: Line<'static>) -> Line<'static> {
        let mut spans = vec![Span::styled("  ", Style::default().fg(theme::TEXT_SUBTLE))];
        if line.spans.is_empty() {
            spans.push(Span::raw(""));
        } else {
            spans.extend(line.spans);
        }
        Line::from(spans)
    }

    fn event_line(line: String) -> Line<'static> {
        Line::from(vec![
            Span::styled("  ", Style::default().fg(theme::TEXT_SUBTLE)),
            Span::styled(line, Style::default().fg(theme::TEXT_SOFT)),
        ])
    }

    fn user_line(line: String) -> Line<'static> {
        Line::from(vec![
            Span::styled("  ", Style::default().fg(theme::TEXT_SUBTLE)),
            Span::styled(line, Style::default().fg(theme::TEXT_PRIMARY)),
        ])
    }

    fn loosen_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
        let mut spaced = Vec::with_capacity(lines.len().saturating_mul(2).saturating_add(1));
        for (idx, line) in lines.into_iter().enumerate() {
            if idx > 0 {
                spaced.push(Line::from(Span::raw("")));
            }
            spaced.push(Self::assistant_line(line));
        }
        spaced
    }

    fn compact_event_lines(content: &str, width: usize) -> Vec<Line<'static>> {
        let mut wrapped = Self::wrap_text(content, width.max(20));
        if wrapped.len() > COMPACT_EVENT_LINES {
            wrapped.truncate(COMPACT_EVENT_LINES);
            if let Some(last) = wrapped.last_mut() {
                last.push_str(" …");
            }
        }
        wrapped.into_iter().map(Self::event_line).collect()
    }

    fn process_meta_line() -> Line<'static> {
        Line::from(vec![
            Span::styled("process ", Style::default().fg(theme::UI_TEXT)),
            Span::styled("live", Style::default().fg(theme::TEXT_SUBTLE)),
        ])
    }

    fn process_lines(&self) -> Vec<Line<'static>> {
        self.live_state
            .process_lines
            .iter()
            .map(|line| {
                Line::from(vec![
                    Span::styled("  · ", Style::default().fg(theme::TEXT_SUBTLE)),
                    Span::styled(line.clone(), Style::default().fg(theme::TEXT_SOFT)),
                ])
            })
            .collect()
    }

    fn live_response_lines(&mut self) -> Vec<Line<'static>> {
        let response = self.live_state.visible_response();
        if response.is_empty() {
            return Vec::new();
        }

        let markdown_lines = self.markdown_renderer.render(&response);
        Self::loosen_lines(markdown_lines)
    }

    fn build_text_lines(&mut self) -> Vec<Line<'static>> {
        let mut text_lines: Vec<Line<'static>> = Vec::new();

        for (idx, message) in self.state.history.iter().enumerate() {
            if idx > 0 {
                text_lines.push(Line::from(Span::raw("")));
            }
            let display_text = Self::truncate_for_display(&message.content);
            text_lines.push(Self::meta_line(
                &message.role,
                idx + 1,
                message.content.chars().count(),
            ));
            let lines = match message.role {
                Role::Assistant => {
                    let markdown_lines = self.markdown_renderer.render(&display_text);
                    Self::loosen_lines(markdown_lines)
                }
                Role::User => {
                    Self::wrap_text(&display_text, self.content_width.saturating_sub(2).max(24))
                        .into_iter()
                        .map(Self::user_line)
                        .collect()
                }
                Role::System | Role::Tool => {
                    Self::compact_event_lines(&display_text, self.content_width.saturating_sub(2))
                }
            };
            text_lines.extend(lines);
        }

        if !self.live_state.process_lines.is_empty() {
            if !text_lines.is_empty() {
                text_lines.push(Line::from(Span::raw("")));
            }
            text_lines.push(Self::process_meta_line());
            text_lines.extend(self.process_lines());
        }

        let live_lines = self.live_response_lines();
        if !live_lines.is_empty() {
            if !text_lines.is_empty() {
                text_lines.push(Line::from(Span::raw("")));
            }
            text_lines.push(Self::meta_line(
                &Role::Assistant,
                self.state.history.len() + 1,
                self.live_state.revealed_chars,
            ));
            text_lines.extend(live_lines);
        }

        text_lines
    }

    fn wrap_line(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
        if width == 0 {
            return vec![line.clone()];
        }
        if line.spans.is_empty() {
            return vec![Line::from(Span::raw(""))];
        }

        let mut wrapped = Vec::new();
        let mut current_spans: Vec<Span<'static>> = Vec::new();
        let mut current_width = 0usize;

        for span in &line.spans {
            let mut segment = String::new();
            let style = span.style;

            for ch in span.content.chars() {
                let ch_width = ch.width().unwrap_or(0);
                if current_width + ch_width > width
                    && (!segment.is_empty() || !current_spans.is_empty())
                {
                    if !segment.is_empty() {
                        current_spans.push(Span::styled(std::mem::take(&mut segment), style));
                    }
                    wrapped.push(Line::from(std::mem::take(&mut current_spans)));
                    current_width = 0;
                }

                segment.push(ch);
                current_width += ch_width;
            }

            if !segment.is_empty() {
                current_spans.push(Span::styled(segment, style));
            }
        }

        if current_spans.is_empty() {
            wrapped.push(Line::from(Span::raw("")));
        } else {
            wrapped.push(Line::from(current_spans));
        }

        wrapped
    }

    fn wrapped_text_lines(&mut self) -> Vec<Line<'static>> {
        let width = self.content_width.max(1);
        self.build_text_lines()
            .into_iter()
            .flat_map(|line| Self::wrap_line(&line, width))
            .collect()
    }

    pub fn total_lines(&mut self) -> usize {
        self.markdown_renderer
            .set_width(self.content_width.saturating_sub(2).max(24));
        self.wrapped_text_lines().len()
    }
}

impl<'a> Renderable for ConversationView<'a> {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                buf[(x, y)].set_style(theme::fill_style());
            }
        }

        self.markdown_renderer
            .set_width(self.content_width.saturating_sub(2).max(24));
        let text_lines = self.wrapped_text_lines();

        let max_scroll = text_lines.len().saturating_sub(self.content_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        Paragraph::new(Text::from(text_lines))
            .style(theme::fill_style())
            .scroll((scroll_offset as u16, 0))
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_empty() {
        assert_eq!(ConversationView::wrap_text("", 40).len(), 1);
    }

    #[test]
    fn test_wrap_text_single() {
        assert_eq!(ConversationView::wrap_text("Hi", 40)[0], "Hi");
    }

    #[test]
    fn test_wrap_text_wraps() {
        assert!(ConversationView::wrap_text("long text", 5).len() > 1);
    }

    #[test]
    fn test_wrap_text_para() {
        assert!(ConversationView::wrap_text("A\nB", 40).len() >= 2);
    }
}
