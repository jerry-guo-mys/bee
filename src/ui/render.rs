//! 界面渲染
//!
//! 根据 UiState（phase、history、error）与 input_buffer 绘制：标题栏显示 phase，
//! 主体为对话历史（按角色着色、工具结果折叠、按宽度换行），底部为现代化输入框（占位符、圆角、
//! 智能体/模型选择器、发送按钮）。

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Wrap,
    },
    Frame,
};

use crate::core::{AgentPhase, UiState};
use crate::memory::Role;

/// 输入区状态：焦点、当前选中的智能体/模型/模式
#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub focus: InputFocus,
    pub agent_index: usize,
    pub model_index: usize,
    pub mode_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputFocus {
    #[default]
    Input,
    Agent,
    Model,
    Mode,
    Image,
    Send,
}

/// 单条消息在 UI 中显示的最大字符数；工具返回的整页内容超过此值会折叠，避免刷屏
const MAX_DISPLAY_CHARS: usize = 600;
/// 工具调用/观察类消息的显示上限（更短，因为多为原始 HTML/JSON）
const MAX_TOOL_DISPLAY_CHARS: usize = 280;

/// 是否为「工具调用结果」类消息（包含整段原始数据，需要折叠显示）
fn is_tool_result(content: &str) -> bool {
    content.starts_with("Tool call:") || content.starts_with("Observation from ")
}

/// 对过长内容做折叠：保留前 N 字 + 省略提示，便于阅读
fn truncate_for_display(content: &str) -> String {
    let limit = if is_tool_result(content) {
        MAX_TOOL_DISPLAY_CHARS
    } else {
        MAX_DISPLAY_CHARS
    };
    let chars: Vec<char> = content.chars().collect();
    if chars.len() <= limit {
        return content.to_string();
    }
    let head: String = chars.iter().take(limit).collect();
    format!("{}\n... [结果已省略，共 {} 字]", head, chars.len())
}

/// 将内容按宽度换行，支持 UTF-8（按字符数，避免在 UTF-8 中间截断）
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

/// 绘制一帧：上方对话区（标题 + 历史 + 滚动条），下方输入区；将 (总行数, 可视高度) 写入 out 供外部 clamp 滚动
pub fn draw(
    f: &mut Frame,
    state: &UiState,
    input_buffer: &str,
    conversation_scroll: usize,
    out: &mut (usize, usize),
    input_state: &InputState,
    agents: &[&str],
    models: &[&str],
) {
    // 输入区：主输入 5 行 + 工具栏 1 行
    let input_height = 6u16;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(input_height)])
        .split(f.area());

    let conv_area = chunks[0];
    let content_width = conv_area.width.saturating_sub(2).saturating_sub(1) as usize; // 边框 + 滚动条

    let phase_str: String = match &state.phase {
        AgentPhase::Idle => "空闲".to_string(),
        AgentPhase::Thinking => "思考中…".to_string(),
        AgentPhase::Streaming => "输出中…".to_string(),
        AgentPhase::ToolExecuting => state
            .active_tool
            .as_deref()
            .map(|t| format!("执行: {}", t))
            .unwrap_or_else(|| "执行中…".to_string()),
        AgentPhase::Responding => "回复中".to_string(),
        AgentPhase::Error => "错误".to_string(),
    };

    let title = format!(" Bee │ {} ", phase_str);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    // 构建对话内容：每条消息先截断过长/工具结果，再按宽度换行；消息之间加空行分隔
    let mut text_lines: Vec<Line> = Vec::new();
    for (idx, m) in state.history.iter().enumerate() {
        if idx > 0 {
            text_lines.push(Line::from(Span::raw("")));
        }
        let (prefix, color) = match m.role {
            Role::User => ("You ", Color::Cyan),
            Role::Assistant => ("Bee ", Color::Green),
            Role::System => ("Sys ", Color::Gray),
            Role::Tool => ("🔧  ", Color::Yellow),
        };
        let display_text = truncate_for_display(&m.content);
        let wrapped = wrap_text(&display_text, content_width.max(40));
        for (i, line) in wrapped.into_iter().enumerate() {
            let pref = if i == 0 { prefix } else { "    " };
            text_lines.push(Line::from(vec![
                Span::styled(
                    pref,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(line),
            ]));
        }
    }

    let content_height = conv_area.height.saturating_sub(2) as usize; // 边框
    let total_lines = text_lines.len();
    let max_scroll = total_lines.saturating_sub(content_height);
    let scroll_offset = conversation_scroll.min(max_scroll);

    let inner = block.inner(conv_area);
    let paragraph = Paragraph::new(Text::from(text_lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset as u16, 0));
    f.render_widget(paragraph, inner);

    if total_lines > content_height {
        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(scroll_offset)
            .viewport_content_length(content_height);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("█")
            .track_symbol(Some("░"));
        f.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }

    let input_area = chunks[1];

    let border_color = if state.error_message.is_some() {
        Color::Red
    } else {
        Color::Rgb(100, 116, 139) // 浅灰
    };

    let hint = " Enter 发送 │ Tab 切换 │ ↑↓ 选择 │ Ctrl+Q 退出 ";
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title_bottom(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        )));

    let inner = input_block.inner(input_area);
    let (text_area, toolbar_area) = {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(inner);
        (chunks[0], chunks[1])
    };

    let display_text = if input_buffer.is_empty() && !state.input_locked {
        Span::styled("随便问点什么...", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(input_buffer)
    };

    let input = Paragraph::new(Text::from(display_text))
        .wrap(Wrap { trim: false })
        .style(if state.input_locked {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        });

    f.render_widget(input_block, input_area);
    f.render_widget(input, text_area);

    // 工具栏：智能体 ▼ | 模型 ◆▼ | 模式: 默认 | 🖼 | ↑
    let agent_idx = input_state.agent_index.min(agents.len().saturating_sub(1));
    let model_idx = input_state.model_index.min(models.len().saturating_sub(1));
    let agent_str = agents.get(agent_idx).copied().unwrap_or("默认");
    let model_str = models.get(model_idx).copied().unwrap_or("默认");

    let agent_style = if input_state.focus == InputFocus::Agent {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let model_style = if input_state.focus == InputFocus::Model {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let send_disabled = input_buffer.trim().is_empty() || state.input_locked;
    let send_style = if send_disabled {
        Style::default().fg(Color::DarkGray)
    } else if input_state.focus == InputFocus::Send {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let toolbar_text = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(agent_str, agent_style),
        Span::styled(" ▼ ", Style::default().fg(Color::DarkGray)),
        Span::styled(" ", Style::default()),
        Span::styled("◆ ", Style::default().fg(Color::Yellow)),
        Span::styled(model_str, model_style),
        Span::styled(" ▼ ", Style::default().fg(Color::DarkGray)),
        Span::styled("  模式: 默认 ", Style::default().fg(Color::DarkGray)),
        Span::styled(" 🖼 ", Style::default().fg(Color::DarkGray)),
        Span::styled(" ↑ ", send_style),
    ]);

    let toolbar = Paragraph::new(toolbar_text);
    f.render_widget(toolbar, toolbar_area);

    out.0 = total_lines;
    out.1 = content_height;
}
