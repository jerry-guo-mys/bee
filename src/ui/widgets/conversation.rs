//! 对话历史渲染组件

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap, Widget},
};

use crate::core::UiState;
use crate::memory::Role;
use crate::ui::markdown::MarkdownRenderer;
use crate::ui::theme;
use crate::ui::Renderable;

const MAX_DISPLAY_CHARS: usize = 600;
const MAX_TOOL_DISPLAY_CHARS: usize = 280;

pub struct ConversationView<'a> {
    pub state: &'a UiState,
    pub scroll_offset: usize,
    pub content_width: usize,
    pub content_height: usize,
    pub markdown_renderer: &'a mut MarkdownRenderer,
}

impl<'a> ConversationView<'a> {
    pub fn new(
        state: &'a UiState,
        scroll_offset: usize,
        content_width: usize,
        content_height: usize,
        markdown_renderer: &'a mut MarkdownRenderer,
    ) -> Self {
        Self {
            state,
            scroll_offset,
            content_width,
            content_height,
            markdown_renderer,
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
            for ch in para.chars() {
                if line.chars().count() >= width {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(ch);
            }
            if !line.is_empty() {
                lines.push(line);
            }
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    fn separator(width: usize) -> Line<'static> {
        let rule = "─".repeat(width.max(8));
        Line::from(Span::styled(rule, Style::default().fg(theme::UI_TEXT_DIM)))
    }

    fn pad_markdown_line(line: &mut Line<'static>) {
        line.spans.insert(0, Span::raw("  "));
    }

    fn loosen_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
        let mut spaced = Vec::with_capacity(lines.len().saturating_mul(2));
        for (idx, line) in lines.into_iter().enumerate() {
            if idx > 0 {
                spaced.push(Line::from(Span::raw("")));
            }
            spaced.push(line);
        }
        spaced
    }

    pub fn total_lines(&mut self) -> usize {
        let mut count = 0usize;
        for message in &self.state.history {
            let display_text = Self::truncate_for_display(&message.content);
            let lines = if message.role == Role::Assistant {
                self.markdown_renderer.render(&display_text)
            } else {
                Self::wrap_text(&display_text, self.content_width.max(40))
                    .into_iter()
                    .map(|line| Line::from(Span::raw(line)))
                    .collect()
            };
            count += lines.len() + 3;
        }
        count.saturating_sub(1)
    }
}

impl<'a> Renderable for ConversationView<'a> {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                buf[(x, y)].set_style(theme::fill_style());
            }
        }

        let mut text_lines: Vec<Line<'static>> = Vec::new();

        for (idx, message) in self.state.history.iter().enumerate() {
            if idx > 0 {
                text_lines.push(Self::separator(self.content_width.saturating_sub(1)));
            }

            let (badge, badge_style) = theme::role_badge(&message.role);
            text_lines.push(Line::from(vec![
                Span::styled(badge, badge_style),
                Span::raw(" "),
                Span::styled(
                    format!("#{:02}", idx + 1),
                    Style::default().fg(theme::UI_TEXT_DIM),
                ),
            ]));

            let display_text = Self::truncate_for_display(&message.content);
            let lines = if message.role == Role::Assistant {
                let mut markdown_lines = self.markdown_renderer.render(&display_text);
                for line in &mut markdown_lines {
                    Self::pad_markdown_line(line);
                }
                Self::loosen_lines(markdown_lines)
            } else {
                let body_width = self.content_width.saturating_sub(2).max(24);
                Self::wrap_text(&display_text, body_width)
                    .into_iter()
                    .map(|line| {
                        Line::from(Span::styled(
                            format!("  {line}"),
                            Style::default().fg(theme::TEXT_PRIMARY),
                        ))
                    })
                    .collect()
            };
            text_lines.extend(lines);
            text_lines.push(Line::from(Span::raw("")));
        }

        let max_scroll = text_lines.len().saturating_sub(self.content_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        Paragraph::new(Text::from(text_lines))
            .style(theme::fill_style())
            .wrap(Wrap { trim: false })
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
