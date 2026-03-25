//! Markdown 渲染器
//!
//! 使用 pulldown-cmark 解析 Markdown，渲染为 Ratatui Line

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::ui::theme;

/// Markdown 渲染器
pub struct MarkdownRenderer {
    width: usize,
    style_stack: Vec<Style>,
    current_line: String,
    rendered_lines: Vec<Line<'static>>,
    code_block_lang: Option<String>,
    in_list_item: bool,
    list_depth: usize,
}

impl MarkdownRenderer {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            style_stack: vec![Style::default().fg(theme::TEXT_CREAM)],
            current_line: String::new(),
            rendered_lines: Vec::new(),
            code_block_lang: None,
            in_list_item: false,
            list_depth: 0,
        }
    }

    pub fn render(&mut self, markdown: &str) -> Vec<Line<'static>> {
        self.rendered_lines.clear();
        self.current_line.clear();
        self.style_stack = vec![Style::default().fg(theme::TEXT_CREAM)];
        self.in_list_item = false;
        self.list_depth = 0;
        let parser = Parser::new(markdown);
        for event in parser {
            self.handle_event(event);
        }
        self.flush_line();
        self.rendered_lines.clone()
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width.max(20);
    }

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn push_style(&mut self, style: Style) {
        let base = self.current_style();
        self.style_stack.push(apply_style(base, style));
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.handle_start_tag(tag),
            Event::End(tag) => self.handle_end_tag(tag),
            Event::Text(text) => self.current_line.push_str(&text),
            Event::Code(code) => {
                let style = Style::default()
                    .fg(theme::ACCENT_CYAN)
                    .bg(theme::PANEL_BG_SOFT);
                self.push_style(style);
                self.current_line.push_str(&code);
                self.pop_style();
            }
            Event::Html(_)
            | Event::FootnoteReference(_)
            | Event::InlineHtml(_)
            | Event::TaskListMarker(_) => {}
            Event::SoftBreak | Event::HardBreak => self.flush_line(),
            Event::Rule => {
                self.flush_line();
                self.rendered_lines.push(Line::from(Span::styled(
                    "─".repeat(self.width.min(80)),
                    Style::default().fg(theme::TEXT_SUBTLE),
                )));
            }
        }
    }

    fn handle_start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => {
                let style = match level {
                    HeadingLevel::H1 => Style::default()
                        .fg(theme::ACCENT_GREEN)
                        .add_modifier(Modifier::BOLD),
                    HeadingLevel::H2 => Style::default()
                        .fg(theme::ACCENT_BLUE)
                        .add_modifier(Modifier::BOLD),
                    HeadingLevel::H3 => Style::default()
                        .fg(theme::TEXT_CREAM)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(theme::TEXT_SOFT)
                        .add_modifier(Modifier::BOLD),
                };
                self.push_style(style);
            }
            Tag::BlockQuote => {
                let style = Style::default()
                    .fg(theme::ACCENT_PURPLE)
                    .add_modifier(Modifier::ITALIC);
                self.push_style(style);
                self.current_line.push_str("▍ ");
            }
            Tag::CodeBlock(kind) => {
                if let CodeBlockKind::Fenced(lang) = kind {
                    self.code_block_lang = Some(lang.to_string());
                    // 渲染代码块开始标记
                    self.flush_line();
                    self.rendered_lines.push(Line::from(Span::styled(
                        format!("┌─[ {} ]", lang),
                        Style::default().fg(theme::ACCENT_CYAN),
                    )));
                    self.current_line.push_str("│ ");
                }
                let style = Style::default()
                    .fg(theme::TEXT_SOFT)
                    .bg(theme::PANEL_BG_SOFT);
                self.push_style(style);
            }
            Tag::Emphasis => self.push_style(Style::default().add_modifier(Modifier::ITALIC)),
            Tag::Strong => self.push_style(Style::default().add_modifier(Modifier::BOLD)),
            Tag::Strikethrough => {
                self.push_style(Style::default().add_modifier(Modifier::CROSSED_OUT))
            }
            Tag::Link { .. } => {
                self.push_style(
                    Style::default()
                        .fg(theme::ACCENT_BLUE)
                        .add_modifier(Modifier::UNDERLINED),
                );
            }
            Tag::Image { .. } => self.push_style(Style::default().fg(theme::ACCENT_PURPLE)),
            Tag::List(_) => {
                self.list_depth += 1;
            }
            Tag::Item => {
                self.in_list_item = true;
                // 根据嵌套深度添加缩进
                let indent = "  ".repeat(self.list_depth - 1);
                self.current_line.push_str(&format!("{}• ", indent));
            }
            Tag::Table(_) | Tag::TableHead | Tag::TableRow | Tag::TableCell => {}
            Tag::FootnoteDefinition(_) | Tag::HtmlBlock | Tag::MetadataBlock(_) => {}
        }
    }

    fn handle_end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush_line();
                // 添加段间距（空行）
                self.rendered_lines.push(Line::from(""));
            }
            TagEnd::Heading { .. } => {
                self.pop_style();
                self.flush_line();
                // 添加段间距（空行）
                self.rendered_lines.push(Line::from(""));
            }
            TagEnd::BlockQuote => {
                self.pop_style();
                self.flush_line();
            }
            TagEnd::CodeBlock => {
                self.pop_style();
                // 渲染代码块结束标记
                self.flush_line();
                if let Some(ref _lang) = self.code_block_lang {
                    self.rendered_lines.push(Line::from(Span::styled(
                        "└──────────",
                        Style::default().fg(theme::ACCENT_CYAN),
                    )));
                }
                self.code_block_lang = None;
                // 添加段间距（空行）
                self.rendered_lines.push(Line::from(""));
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => self.pop_style(),
            TagEnd::Link | TagEnd::Image => self.pop_style(),
            TagEnd::List(_) => {
                self.list_depth -= 1;
                // 列表结束后添加空行
                if self.list_depth == 0 {
                    self.flush_line();
                    self.rendered_lines.push(Line::from(""));
                }
            }
            TagEnd::Item => {
                self.flush_line();
                self.in_list_item = false;
            }
            TagEnd::Table | TagEnd::TableHead | TagEnd::TableRow | TagEnd::TableCell => {}
            TagEnd::FootnoteDefinition | TagEnd::HtmlBlock | TagEnd::MetadataBlock(_) => {}
        }
    }

    fn flush_line(&mut self) {
        if !self.current_line.is_empty() {
            let line = Line::from(Span::styled(
                self.current_line.clone(),
                self.current_style(),
            ));
            self.rendered_lines.push(line);
            self.current_line.clear();
        }
    }
}

fn apply_style(base: Style, overlay: Style) -> Style {
    let mut result = base;
    if let Some(fg) = overlay.fg {
        result = result.fg(fg);
    }
    result = result.add_modifier(overlay.add_modifier);
    result = result.remove_modifier(overlay.sub_modifier);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_render_plain_text() {
        let mut renderer = MarkdownRenderer::new(80);
        let lines = renderer.render("Hello, World!");
        // 段落结束后会添加一个空行
        assert_eq!(lines.len(), 2);
    }
    #[test]
    fn test_render_bold() {
        let mut renderer = MarkdownRenderer::new(80);
        let lines = renderer.render("**bold**");
        assert_eq!(lines.len(), 2);
    }
    #[test]
    fn test_render_italic() {
        let mut renderer = MarkdownRenderer::new(80);
        let lines = renderer.render("*italic*");
        assert_eq!(lines.len(), 2);
    }
    #[test]
    fn test_render_heading() {
        let mut renderer = MarkdownRenderer::new(80);
        let lines = renderer.render("# Heading 1");
        assert!(!lines.is_empty());
    }
    #[test]
    fn test_render_code() {
        let mut renderer = MarkdownRenderer::new(80);
        let lines = renderer.render("`inline code`");
        assert_eq!(lines.len(), 2);
    }
}
