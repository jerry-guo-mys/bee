//! File search popup

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::ui::theme;
use crate::ui::Renderable;

pub struct FilePopup {
    filtered: Vec<String>,
    selected: usize,
    visible: bool,
    max_display: usize,
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let query = query.to_lowercase();
    let candidate = candidate.to_lowercase();
    let mut score = 0i32;
    let mut search_from = 0usize;

    for ch in query.chars() {
        let found = candidate[search_from..].find(ch)?;
        score += if found == 0 { 4 } else { 1 };
        search_from += found + 1;
    }

    if candidate.starts_with(&query) {
        score += 8;
    } else if candidate.contains(&query) {
        score += 4;
    }

    Some(score - candidate.len() as i32 / 16)
}

impl FilePopup {
    pub fn new() -> Self {
        Self {
            filtered: Vec::new(),
            selected: 0,
            visible: false,
            max_display: 8,
        }
    }

    pub fn filter(&mut self, prefix: &str, files: &[String]) {
        self.filtered.clear();
        self.selected = 0;

        let mut scored: Vec<(&String, i32)> = files
            .iter()
            .filter_map(|path| fuzzy_score(prefix, path).map(|score| (path, score)))
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        self.filtered = scored
            .into_iter()
            .take(100)
            .map(|(path, _)| path.clone())
            .collect();

        self.visible = !self.filtered.is_empty();
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn select_previous(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered.len() - 1);
        }
    }

    pub fn confirm(&self) -> Option<&str> {
        self.filtered.get(self.selected).map(String::as_str)
    }

    pub fn display_height(&self) -> usize {
        self.filtered.len().min(self.max_display)
    }
}

impl Default for FilePopup {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for FilePopup {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                buf[(x, y)].set_style(Style::default().bg(theme::PANEL_BG));
            }
        }

        let lines: Vec<Line<'static>> = self
            .filtered
            .iter()
            .take(self.max_display)
            .enumerate()
            .map(|(idx, item)| {
                let style = if idx == self.selected {
                    Style::default().fg(theme::TEXT_PRIMARY)
                } else {
                    Style::default().fg(theme::UI_TEXT_DIM)
                };
                Line::from(vec![
                    Span::styled(if idx == self.selected { "> " } else { "  " }, style),
                    Span::styled(item.clone(), style),
                ])
            })
            .collect();

        Paragraph::new(lines)
            .style(Style::default().bg(theme::PANEL_BG))
            .render(area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        self.display_height() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_files() {
        let mut popup = FilePopup::new();
        let files = vec![
            "src/main.rs".to_string(),
            "src/ui/app.rs".to_string(),
            "Cargo.toml".to_string(),
        ];
        popup.filter("ui", &files);
        assert!(popup.is_visible());
        assert_eq!(popup.confirm(), Some("src/ui/app.rs"));
    }

    #[test]
    fn test_navigation() {
        let mut popup = FilePopup::new();
        let files = vec!["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()];
        popup.filter("", &files);
        popup.select_next();
        popup.select_next();
        assert_eq!(popup.confirm(), Some("c.rs"));
        popup.select_previous();
        assert_eq!(popup.confirm(), Some("b.rs"));
    }
}
