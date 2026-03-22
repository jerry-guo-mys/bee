//! 状态指示器组件

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::ui::theme;

use super::Renderable;

pub struct StatusIndicator {
    pub header: String,
    pub details: Option<String>,
    pub inline_message: Option<String>,
    pub elapsed_running: std::time::Duration,
    pub last_resume_at: std::time::Instant,
    pub is_paused: bool,
    pub show_interrupt_hint: bool,
    pub details_max_lines: usize,
}

impl Default for StatusIndicator {
    fn default() -> Self {
        Self {
            header: String::from("ready"),
            details: None,
            inline_message: None,
            elapsed_running: std::time::Duration::ZERO,
            last_resume_at: std::time::Instant::now(),
            is_paused: false,
            show_interrupt_hint: true,
            details_max_lines: 1,
        }
    }
}

impl StatusIndicator {
    pub fn format_elapsed(&self) -> String {
        let secs = self.elapsed_running.as_secs();
        if secs < 60 {
            format!("{secs}s")
        } else if secs < 3600 {
            let mins = secs / 60;
            let secs = secs % 60;
            format!("{mins}m {secs}s")
        } else {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            let secs = secs % 60;
            format!("{hours}h {mins:02}m {secs:02}s")
        }
    }

    pub fn spinner_frame(&self) -> &'static str {
        const FRAMES: &[&str] = &["◜", "◠", "◝", "◞", "◡", "◟"];
        let frame_idx = (self.elapsed_running.as_millis() / 120) as usize % FRAMES.len();
        FRAMES[frame_idx]
    }

    pub fn update_elapsed(&mut self) {
        if !self.is_paused {
            let now = std::time::Instant::now();
            let delta = now.duration_since(self.last_resume_at);
            self.elapsed_running += delta;
            self.last_resume_at = now;
        }
    }

    pub fn pause(&mut self) {
        if !self.is_paused {
            self.is_paused = true;
        }
    }

    pub fn resume(&mut self) {
        if self.is_paused {
            self.is_paused = false;
            self.last_resume_at = std::time::Instant::now();
        }
    }
}

impl Renderable for StatusIndicator {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let mut primary = vec![
            Span::styled(
                format!(" {} ", self.spinner_frame()),
                Style::default().fg(theme::UI_TEXT).bg(theme::PANEL_BG_SOFT),
            ),
            Span::raw(" "),
            Span::styled(&self.header, Style::default().fg(theme::UI_TEXT)),
            Span::raw("  "),
            Span::styled(
                format!("runtime {}", self.format_elapsed()),
                Style::default().fg(theme::UI_TEXT_DIM),
            ),
        ];

        if let Some(ref inline) = self.inline_message {
            primary.push(Span::raw("  "));
            primary.push(Span::styled(
                inline,
                Style::default().fg(theme::UI_TEXT_DIM),
            ));
        }

        if self.show_interrupt_hint {
            primary.push(Span::raw("  "));
            primary.push(Span::styled(
                "esc interrupt",
                Style::default().fg(theme::UI_TEXT_DIM),
            ));
        }

        let mut lines = vec![Line::from(primary)];
        if let Some(ref details) = self.details {
            for line in details.lines().take(self.details_max_lines) {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(theme::UI_TEXT_DIM),
                )));
            }
        }

        Paragraph::new(lines)
            .style(theme::fill_style())
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        let detail_lines = self
            .details
            .as_ref()
            .map(|details| details.lines().take(self.details_max_lines).count() as u16)
            .unwrap_or(0);
        1 + detail_lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_elapsed_seconds() {
        let indicator = StatusIndicator {
            elapsed_running: std::time::Duration::from_secs(30),
            ..Default::default()
        };
        assert_eq!(indicator.format_elapsed(), "30s");
    }

    #[test]
    fn test_format_elapsed_minutes() {
        let indicator = StatusIndicator {
            elapsed_running: std::time::Duration::from_secs(90),
            ..Default::default()
        };
        assert_eq!(indicator.format_elapsed(), "1m 30s");
    }

    #[test]
    fn test_format_elapsed_hours() {
        let indicator = StatusIndicator {
            elapsed_running: std::time::Duration::from_secs(7384),
            ..Default::default()
        };
        assert_eq!(indicator.format_elapsed(), "2h 03m 04s");
    }
}
