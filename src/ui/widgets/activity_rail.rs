//! Activity rail widget for wide layouts.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::core::{AgentPhase, UiState};
use crate::memory::Role;
use crate::ui::theme;

use super::Renderable;

pub struct ActivityRail<'a> {
    pub state: &'a UiState,
}

impl<'a> ActivityRail<'a> {
    pub fn new(state: &'a UiState) -> Self {
        Self { state }
    }

    fn counts(&self) -> (usize, usize, usize, usize) {
        let mut users = 0usize;
        let mut assistants = 0usize;
        let mut systems = 0usize;
        let mut tools = 0usize;

        for message in &self.state.history {
            match message.role {
                Role::User => users += 1,
                Role::Assistant => assistants += 1,
                Role::System => systems += 1,
                Role::Tool => tools += 1,
            }
        }

        (users, assistants, systems, tools)
    }

    fn phase_text(&self) -> &'static str {
        match self.state.phase {
            AgentPhase::Idle => "ready",
            AgentPhase::Thinking => "thinking",
            AgentPhase::Streaming => "streaming",
            AgentPhase::ToolExecuting => "running tool",
            AgentPhase::Responding => "finalizing",
            AgentPhase::Error => "error",
        }
    }

    fn workflow_lines(&self) -> Vec<Line<'static>> {
        let has_user = self.state.history.iter().any(|m| m.role == Role::User);
        let has_tool = self.state.history.iter().any(|m| m.role == Role::Tool);
        let has_answer = self.state.history.iter().any(|m| m.role == Role::Assistant);

        let items = [
            ("request captured", has_user),
            ("tool activity", has_tool),
            ("answer drafted", has_answer),
        ];

        items
            .into_iter()
            .map(|(label, done)| {
                Line::from(vec![
                    Span::styled(
                        if done { "• " } else { "· " },
                        Style::default().fg(theme::UI_TEXT_DIM),
                    ),
                    Span::styled(label, Style::default().fg(theme::UI_TEXT)),
                ])
            })
            .collect()
    }

    fn recent_activity_lines(&self) -> Vec<Line<'static>> {
        let mut rows = Vec::new();
        for message in self
            .state
            .history
            .iter()
            .rev()
            .filter(|message| message.role != Role::User && message.role != Role::Assistant)
            .take(4)
        {
            let label = match message.role {
                Role::System => "system",
                Role::Tool => "tool",
                Role::User | Role::Assistant => unreachable!(),
            };
            let snippet = message
                .content
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(34)
                .collect::<String>();
            rows.push(Line::from(vec![
                Span::styled(format!("{label} "), Style::default().fg(theme::UI_TEXT_DIM)),
                Span::styled(snippet, Style::default().fg(theme::UI_TEXT)),
            ]));
        }

        if rows.is_empty() {
            rows.push(Line::from(Span::styled(
                "no recent system activity",
                Style::default().fg(theme::UI_TEXT_DIM),
            )));
        }

        rows
    }

    fn section_title(title: &'static str) -> Line<'static> {
        Line::from(Span::styled(title, Style::default().fg(theme::UI_TEXT_DIM)))
    }
}

impl<'a> Renderable for ActivityRail<'a> {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                buf[(x, y)].set_style(Style::default().bg(theme::PANEL_BG));
            }
        }

        let (users, assistants, systems, tools) = self.counts();

        let mut lines = vec![
            Self::section_title("session"),
            Line::from(Span::styled(
                self.phase_text(),
                Style::default().fg(theme::UI_TEXT),
            )),
            Line::from(Span::styled(
                if self.state.input_locked {
                    "input locked"
                } else {
                    "input open"
                },
                Style::default().fg(theme::UI_TEXT_DIM),
            )),
        ];

        if let Some(tool) = &self.state.active_tool {
            lines.push(Line::from(Span::styled(
                format!("tool {tool}"),
                Style::default().fg(theme::UI_TEXT),
            )));
        }

        lines.push(Line::from(Span::raw("")));
        lines.push(Self::section_title("metrics"));
        lines.push(Line::from(Span::styled(
            format!("{users} user · {assistants} assistant"),
            Style::default().fg(theme::UI_TEXT),
        )));
        lines.push(Line::from(Span::styled(
            format!("{tools} tool · {systems} system"),
            Style::default().fg(theme::UI_TEXT_DIM),
        )));

        lines.push(Line::from(Span::raw("")));
        lines.push(Self::section_title("workflow"));
        lines.extend(self.workflow_lines());

        lines.push(Line::from(Span::raw("")));
        lines.push(Self::section_title("recent"));
        lines.extend(self.recent_activity_lines());

        lines.push(Line::from(Span::raw("")));
        lines.push(Self::section_title("keys"));
        lines.push(Line::from(Span::styled(
            "enter send",
            Style::default().fg(theme::UI_TEXT),
        )));
        lines.push(Line::from(Span::styled(
            "tab switch focus",
            Style::default().fg(theme::UI_TEXT_DIM),
        )));

        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(theme::PANEL_BG))
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
