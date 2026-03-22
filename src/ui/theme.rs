//! TUI 主题定义

use ratatui::style::{Color, Style};

use crate::core::AgentPhase;
use crate::memory::Role;

pub const APP_BG: Color = Color::Rgb(9, 13, 22);
pub const PANEL_BG: Color = Color::Rgb(16, 24, 38);
pub const PANEL_BG_SOFT: Color = Color::Rgb(22, 32, 50);
pub const PANEL_BORDER: Color = Color::Rgb(56, 78, 108);
pub const PANEL_BORDER_ACTIVE: Color = Color::Rgb(112, 164, 255);
pub const TEXT_PRIMARY: Color = Color::Rgb(232, 238, 247);
pub const TEXT_CREAM: Color = Color::Rgb(244, 240, 230);
pub const TEXT_SOFT: Color = Color::Rgb(188, 201, 219);
pub const TEXT_MUTED: Color = Color::Rgb(141, 156, 178);
pub const TEXT_SUBTLE: Color = Color::Rgb(96, 114, 138);
pub const UI_TEXT: Color = Color::Rgb(205, 205, 205);
pub const UI_TEXT_DIM: Color = Color::Rgb(172, 172, 172);
pub const ACCENT_BLUE: Color = Color::Rgb(112, 164, 255);
pub const ACCENT_CYAN: Color = Color::Rgb(88, 208, 214);
pub const ACCENT_GREEN: Color = Color::Rgb(116, 214, 155);
pub const ACCENT_GOLD: Color = Color::Rgb(244, 191, 94);
pub const ACCENT_RED: Color = Color::Rgb(255, 120, 120);
pub const ACCENT_PURPLE: Color = Color::Rgb(188, 142, 255);
pub const DIVIDER: Color = Color::Rgb(38, 54, 79);

pub fn fill_style() -> Style {
    Style::default().bg(APP_BG).fg(TEXT_PRIMARY)
}

pub fn panel_style() -> Style {
    Style::default().bg(PANEL_BG).fg(TEXT_PRIMARY)
}

pub fn muted_line(width: u16) -> String {
    "─".repeat(width.max(1) as usize)
}

pub fn phase_badge(phase: &AgentPhase) -> (&'static str, Style) {
    match phase {
        AgentPhase::Idle => ("idle", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
        AgentPhase::Thinking => ("think", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
        AgentPhase::Streaming => ("stream", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
        AgentPhase::ToolExecuting => ("tool", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
        AgentPhase::Responding => ("finalize", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
        AgentPhase::Error => ("error", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
    }
}

pub fn role_badge(role: &Role) -> (&'static str, Style) {
    match role {
        Role::User => (" user ", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
        Role::Assistant => (
            " assistant ",
            Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT),
        ),
        Role::System => (" system ", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
        Role::Tool => (" tool ", Style::default().fg(UI_TEXT).bg(PANEL_BG_SOFT)),
    }
}
