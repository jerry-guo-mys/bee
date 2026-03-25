//! 对话历史渲染组件 - 简洁模式

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

    fn assistant_line(line: Line<'static>) -> Line<'static> {
        let mut spans = vec![Span::styled("  ", Style::default().fg(theme::TEXT_SUBTLE))];
        if line.spans.is_empty() {
            spans.push(Span::raw(""));
        } else {
            spans.extend(line.spans);
        }
        Line::from(spans)
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
        let mut needs_spacing = false;

        for message in self.state.history.iter() {
            // 只显示 User 和 Assistant 的消息，隐藏 System 和 Tool
            match message.role {
                Role::Assistant => {
                    // 添加间隔
                    if needs_spacing {
                        text_lines.push(Line::from(Span::raw("")));
                    }
                    needs_spacing = true;

                    // 助手消息用 Markdown 渲染，带左侧标识
                    let markdown_lines = self.markdown_renderer.render(&message.content);
                    for line in markdown_lines {
                        let mut spans = vec![
                            Span::styled("▌ ", Style::default().fg(theme::ACCENT_BLUE)),
                        ];
                        spans.extend(line.spans);
                        text_lines.push(Line::from(spans));
                    }
                }
                Role::User => {
                    // 添加间隔
                    if needs_spacing {
                        text_lines.push(Line::from(Span::raw("")));
                    }
                    needs_spacing = true;

                    // 用户消息带前缀标识
                    let wrapped = Self::wrap_text(&message.content, self.content_width.saturating_sub(4).max(24));
                    for (line_idx, line) in wrapped.into_iter().enumerate() {
                        let prefix = if line_idx == 0 {
                            Span::styled("❯ ", Style::default().fg(theme::ACCENT_GREEN))
                        } else {
                            Span::raw("  ")
                        };
                        text_lines.push(Line::from(vec![
                            prefix,
                            Span::styled(line, Style::default().fg(theme::TEXT_PRIMARY)),
                        ]));
                    }
                }
                Role::System | Role::Tool => {
                    // 隐藏系统和工具消息
                    continue;
                }
            }
        }

        // 添加实时响应（如果有）
        let live_lines = self.live_response_lines();
        if !live_lines.is_empty() {
            if !text_lines.is_empty() {
                text_lines.push(Line::from(Span::raw("")));
            }
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
