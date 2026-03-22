//! Slash command popup

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::ui::theme;
use crate::ui::Renderable;

pub struct CommandItem {
    pub name: &'static str,
    pub description: &'static str,
}

pub const AVAILABLE_COMMANDS: &[CommandItem] = &[
    CommandItem {
        name: "/help",
        description: "show help",
    },
    CommandItem {
        name: "/clear",
        description: "clear history",
    },
    CommandItem {
        name: "/exit",
        description: "exit app",
    },
    CommandItem {
        name: "/quit",
        description: "exit app",
    },
    CommandItem {
        name: "/config",
        description: "open config",
    },
    CommandItem {
        name: "/model",
        description: "switch model",
    },
    CommandItem {
        name: "/agent",
        description: "switch agent",
    },
    CommandItem {
        name: "/theme",
        description: "switch theme",
    },
];

pub struct CommandPopup {
    filtered: Vec<usize>,
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

    Some(score - candidate.len() as i32 / 8)
}

impl CommandPopup {
    pub fn new() -> Self {
        Self {
            filtered: Vec::new(),
            selected: 0,
            visible: false,
            max_display: 8,
        }
    }

    pub fn filter(&mut self, prefix: &str) {
        self.filtered.clear();
        self.selected = 0;

        let mut scored: Vec<(usize, i32)> = AVAILABLE_COMMANDS
            .iter()
            .enumerate()
            .filter_map(|(i, cmd)| fuzzy_score(prefix, cmd.name).map(|score| (i, score)))
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        self.filtered = scored.into_iter().map(|(i, _)| i).collect();

        self.visible = !self.filtered.is_empty();
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

    pub fn selected_command(&self) -> Option<&'static str> {
        self.filtered
            .get(self.selected)
            .map(|&i| AVAILABLE_COMMANDS[i].name)
    }

    pub fn confirm(&mut self) -> Option<&'static str> {
        self.selected_command()
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn display_height(&self) -> usize {
        self.filtered.len().min(self.max_display)
    }
}

impl Default for CommandPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderable for CommandPopup {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                buf[(x, y)].set_style(Style::default().bg(theme::PANEL_BG));
            }
        }

        let mut lines = Vec::new();
        for (i, &cmd_idx) in self.filtered.iter().take(self.max_display).enumerate() {
            let cmd = &AVAILABLE_COMMANDS[cmd_idx];
            let style = if i == self.selected {
                Style::default().fg(theme::UI_TEXT)
            } else {
                Style::default().fg(theme::UI_TEXT_DIM)
            };

            lines.push(Line::from(vec![
                Span::styled(if i == self.selected { "> " } else { "  " }, style),
                Span::styled(format!("{:<10}", cmd.name), style),
                Span::styled(cmd.description, Style::default().fg(theme::UI_TEXT_DIM)),
            ]));
        }

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
    fn test_filter_commands() {
        let mut popup = CommandPopup::new();
        popup.filter("/he");

        assert!(popup.is_visible());
        assert!(popup.filtered.len() >= 1);
        assert_eq!(popup.selected_command(), Some("/help"));
    }

    #[test]
    fn test_no_match() {
        let mut popup = CommandPopup::new();
        popup.filter("/xyz123");

        assert!(!popup.is_visible());
        assert!(popup.filtered.is_empty());
    }

    #[test]
    fn test_navigation() {
        let mut popup = CommandPopup::new();
        popup.filter("/");

        popup.select_next();
        popup.select_next();
        assert_eq!(popup.selected, 2);

        popup.select_previous();
        assert_eq!(popup.selected, 1);
    }

    #[test]
    fn test_case_insensitive() {
        let mut popup = CommandPopup::new();
        popup.filter("/HELP");

        assert!(popup.is_visible());
        assert_eq!(popup.selected_command(), Some("/help"));
    }
}
