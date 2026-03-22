//! 语法高亮器
//!
//! 使用 syntect 进行代码语法高亮

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

/// 语法高亮器
pub struct Highlighter {
    /// 语法集合
    syntax_set: SyntaxSet,
    /// 主题集合
    theme_set: ThemeSet,
    /// 当前主题名称
    theme_name: String,
}

impl Highlighter {
    /// 创建新的高亮器
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        Self {
            syntax_set,
            theme_set,
            theme_name: String::from("base16-ocean.dark"),
        }
    }

    /// 设置主题
    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        if self.theme_set.themes.contains_key(theme_name) {
            self.theme_name = theme_name.to_string();
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", theme_name))
        }
    }

    /// 获取可用主题列表
    pub fn available_themes(&self) -> Vec<&str> {
        self.theme_set.themes.keys().map(|s| s.as_str()).collect()
    }

    /// 高亮代码
    pub fn highlight(&self, code: &str, language: &str) -> Vec<Line<'static>> {
        // 查找语法
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // 创建高亮器
        let theme = &self.theme_set.themes[&self.theme_name];
        let mut highlighter = HighlightLines::new(syntax, theme);

        // 高亮每一行
        let mut lines = Vec::new();
        for line in code.lines() {
            let highlighted = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let spans: Vec<Span> = highlighted
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(text.to_string(), syntect_style_to_ratatui(style))
                })
                .collect();

            if !spans.is_empty() {
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(Span::raw(line.to_string())));
            }
        }

        lines
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// 将 syntect 样式转换为 Ratatui 样式
fn syntect_style_to_ratatui(style: syntect::highlighting::Style) -> Style {
    let mut ratatui_style = Style::default();

    // 转换前景色
    ratatui_style = ratatui_style.fg(syntect_color_to_ratatui(style.foreground));

    // 转换字体样式
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::BOLD)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::ITALIC)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::UNDERLINE)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }

    ratatui_style
}

/// 将 syntect 颜色转换为 Ratatui 颜色
fn syntect_color_to_ratatui(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_creation() {
        let highlighter = Highlighter::new();
        assert!(!highlighter.available_themes().is_empty());
    }

    #[test]
    fn test_highlight_rust_code() {
        let highlighter = Highlighter::new();
        let code = r#"fn main() {
    println!("Hello, World!");
}"#;
        let lines = highlighter.highlight(code, "rs");
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_highlight_plain_text() {
        let highlighter = Highlighter::new();
        let code = "Just plain text";
        let lines = highlighter.highlight(code, "txt");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_set_theme() {
        let mut highlighter = Highlighter::new();
        assert!(highlighter.set_theme("base16-ocean.dark").is_ok());
        assert!(highlighter.set_theme("nonexistent").is_err());
    }
}
