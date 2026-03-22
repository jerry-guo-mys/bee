//! Simplified TUI main loop

use std::io;
use std::ops::Range;
use std::path::Path;
use std::time::{Duration, Instant};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::{mpsc, watch};
use tokio::time::sleep;
use walkdir::WalkDir;

use crate::core::UiState;
use crate::react::ReactEvent;
use crate::ui::render::{draw, RenderContext};
use crate::ui::widgets::{CommandPopup, FilePopup, InputFocus, InputHistory, InputState};

const DEFAULT_AGENTS: &[&str] = &["默认", "自动分派"];
const DEFAULT_MODELS: &[&str] = &["默认", "DeepSeek", "GPT-4o", "Claude"];
const MAX_PROCESS_LINES: usize = 8;
const STREAM_REVEAL_INTERVAL: Duration = Duration::from_millis(14);
const PROCESS_PREVIEW_CHARS: usize = 120;

#[derive(Debug, Default, Clone)]
pub struct LiveConversationState {
    pub process_lines: Vec<String>,
    pub live_response: String,
    pub revealed_chars: usize,
    pub last_reveal_at: Option<Instant>,
}

impl LiveConversationState {
    pub(crate) fn reset(&mut self) {
        self.process_lines.clear();
        self.live_response.clear();
        self.revealed_chars = 0;
        self.last_reveal_at = None;
    }

    fn push_process_line(&mut self, line: String) {
        let line = line.trim().to_string();
        if line.is_empty() {
            return;
        }
        if self.process_lines.last().is_some_and(|last| last == &line) {
            return;
        }
        self.process_lines.push(line);
        if self.process_lines.len() > MAX_PROCESS_LINES {
            let excess = self.process_lines.len() - MAX_PROCESS_LINES;
            self.process_lines.drain(..excess);
        }
    }

    pub(crate) fn set_live_response(&mut self, text: String) {
        self.live_response = text;
        self.revealed_chars = 0;
        self.last_reveal_at = Some(Instant::now());
    }

    pub(crate) fn reveal_tick(&mut self) {
        if self.live_response.is_empty() {
            return;
        }

        let total_chars = self.live_response.chars().count();
        if self.revealed_chars >= total_chars {
            return;
        }

        let now = Instant::now();
        let last = self.last_reveal_at.unwrap_or(now);
        let elapsed = now.saturating_duration_since(last);
        let steps = elapsed.as_millis() / STREAM_REVEAL_INTERVAL.as_millis().max(1);
        if steps == 0 {
            return;
        }

        self.revealed_chars = (self.revealed_chars + steps as usize).min(total_chars);
        self.last_reveal_at = Some(now);
    }

    pub(crate) fn visible_response(&self) -> String {
        self.live_response
            .chars()
            .take(self.revealed_chars)
            .collect()
    }

    pub(crate) fn is_revealing(&self) -> bool {
        !self.live_response.is_empty() && self.revealed_chars < self.live_response.chars().count()
    }

    pub(crate) fn sync_with_state(&mut self, state: &UiState) {
        let last_assistant = state
            .history
            .iter()
            .rev()
            .find(|message| matches!(message.role, crate::memory::Role::Assistant))
            .map(|message| message.content.as_str());

        if !state.input_locked
            && last_assistant.is_some_and(|content| content == self.live_response)
        {
            self.live_response.clear();
            self.revealed_chars = 0;
            self.last_reveal_at = None;
        }

        if !state.input_locked && state.error_message.is_none() && self.live_response.is_empty() {
            self.process_lines.clear();
        }
    }
}

fn summarize_json(args: &serde_json::Value) -> String {
    let compact = args.to_string();
    let mut chars = compact.chars();
    let summary: String = chars.by_ref().take(64).collect();
    if chars.next().is_some() {
        format!("{summary}...")
    } else {
        summary
    }
}

fn summarize_text(text: &str) -> String {
    let single_line = text.replace('\n', " ");
    let trimmed = single_line.trim();
    let mut chars = trimmed.chars();
    let summary: String = chars.by_ref().take(PROCESS_PREVIEW_CHARS).collect();
    if chars.next().is_some() {
        format!("{summary}...")
    } else {
        summary
    }
}

fn handle_react_event(live: &mut LiveConversationState, event: ReactEvent) {
    match event {
        ReactEvent::StepUpdate { step, max_steps } => {
            live.push_process_line(format!("step {}/{}", step + 1, max_steps));
        }
        ReactEvent::Thinking => live.push_process_line("thinking".to_string()),
        ReactEvent::ThinkingContent { text } => {
            let preview = summarize_text(&text);
            if !preview.is_empty() {
                live.push_process_line(format!("plan {preview}"));
            }
        }
        ReactEvent::ToolCall { tool, args } => {
            let args = summarize_json(&args);
            live.push_process_line(format!("tool {tool} {args}"));
        }
        ReactEvent::Observation { tool, preview } => {
            let preview = summarize_text(&preview);
            live.push_process_line(format!("result {tool} {preview}"));
        }
        ReactEvent::ToolFailure { tool, reason } => {
            live.push_process_line(format!("tool failed {tool} {reason}"));
        }
        ReactEvent::Recovery { action, detail } => {
            let detail = summarize_text(&detail);
            live.push_process_line(format!("recovery {action} {detail}"));
        }
        ReactEvent::MemoryRecovery { preview } => {
            let preview = summarize_text(&preview);
            live.push_process_line(format!("memory {preview}"));
        }
        ReactEvent::MemoryConsolidation { preview } => {
            let preview = summarize_text(&preview);
            live.push_process_line(format!("saved {preview}"));
        }
        ReactEvent::TokenUsage { total_tokens, .. } => {
            live.push_process_line(format!("tokens {total_tokens}"));
        }
        ReactEvent::Error { text } => live.push_process_line(format!("error {text}")),
        ReactEvent::MessageChunk { .. } | ReactEvent::MessageDone => {}
    }
}

fn previous_boundary(text: &str, pos: usize) -> usize {
    text.char_indices()
        .map(|(idx, _)| idx)
        .filter(|idx| *idx < pos)
        .last()
        .unwrap_or(0)
}

fn mention_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut token_start = None;

    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = token_start.take() {
                if text[start..idx].starts_with('@') && idx.saturating_sub(start) > 1 {
                    ranges.push(start..idx);
                }
            }
            continue;
        }

        if token_start.is_none() {
            token_start = Some(idx);
        }
    }

    if let Some(start) = token_start {
        if text[start..].starts_with('@') && text.len().saturating_sub(start) > 1 {
            ranges.push(start..text.len());
        }
    }

    ranges
}

fn token_range_at_or_adjacent(text: &str, cursor: usize) -> Option<Range<usize>> {
    mention_ranges(text).into_iter().find(|range| {
        range.start < cursor && cursor <= range.end || cursor == range.start || cursor == range.end
    })
}

fn next_boundary(text: &str, pos: usize) -> usize {
    text.char_indices()
        .map(|(idx, _)| idx)
        .find(|idx| *idx > pos)
        .unwrap_or(text.len())
}

fn line_start(text: &str, pos: usize) -> usize {
    text[..pos.min(text.len())]
        .rfind('\n')
        .map(|idx| idx + 1)
        .unwrap_or(0)
}

fn line_end(text: &str, pos: usize) -> usize {
    let safe = pos.min(text.len());
    text[safe..]
        .find('\n')
        .map(|idx| safe + idx)
        .unwrap_or(text.len())
}

fn composer_text_width(total_width: u16) -> usize {
    total_width.saturating_sub(3).max(1) as usize
}

fn visual_line_ranges(text: &str, width: usize) -> Vec<Range<usize>> {
    if text.is_empty() {
        return vec![0..0];
    }

    let mut ranges = Vec::new();
    let mut line_start = 0usize;
    let mut line_width = 0usize;

    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            ranges.push(line_start..idx);
            line_start = idx + ch.len_utf8();
            line_width = 0;
            continue;
        }

        if line_width >= width {
            ranges.push(line_start..idx);
            line_start = idx;
            line_width = 0;
        }

        line_width += 1;
    }

    ranges.push(line_start..text.len());
    if text.ends_with('\n') {
        ranges.push(text.len()..text.len());
    }
    ranges
}

fn range_for_cursor(ranges: &[Range<usize>], cursor: usize) -> usize {
    ranges
        .iter()
        .enumerate()
        .find(|(_, range)| (range.start..=range.end).contains(&cursor))
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| ranges.len().saturating_sub(1))
}

fn byte_in_range_at_column(text: &str, range: &Range<usize>, column: usize) -> usize {
    text[range.clone()]
        .char_indices()
        .nth(column)
        .map(|(idx, _)| range.start + idx)
        .unwrap_or(range.end)
}

fn move_cursor_visual_vertical(
    text: &str,
    pos: usize,
    width: usize,
    direction: i32,
    preferred: &mut Option<usize>,
) -> usize {
    let ranges = visual_line_ranges(text, width.max(1));
    let current_idx = range_for_cursor(&ranges, pos.min(text.len()));
    let current_range = &ranges[current_idx];
    let current_column = preferred.unwrap_or_else(|| {
        text[current_range.start..pos.min(current_range.end)]
            .chars()
            .count()
    });
    *preferred = Some(current_column);

    if direction < 0 {
        if current_idx == 0 {
            return pos;
        }
        byte_in_range_at_column(text, &ranges[current_idx - 1], current_column)
    } else {
        if current_idx + 1 >= ranges.len() {
            return pos;
        }
        byte_in_range_at_column(text, &ranges[current_idx + 1], current_column)
    }
}

fn current_mention_range(text: &str, cursor: usize) -> Option<(usize, usize)> {
    let safe_cursor = cursor.min(text.len());
    let left = &text[..safe_cursor];
    let start = left.rfind('@')?;
    let candidate = &text[start..safe_cursor];
    if candidate.contains(char::is_whitespace) {
        return None;
    }
    Some((start, safe_cursor))
}

fn replace_range(text: &mut String, range: std::ops::Range<usize>, replacement: &str) -> usize {
    text.replace_range(range.clone(), replacement);
    range.start + replacement.len()
}

fn clamp_cursor(input_state: &mut InputState, text: &str) {
    input_state.cursor_byte = input_state.cursor_byte.min(text.len());
    while input_state.cursor_byte > 0 && !text.is_char_boundary(input_state.cursor_byte) {
        input_state.cursor_byte -= 1;
    }
}

fn refresh_command_popup(command_popup: &mut CommandPopup, input_buffer: &str) {
    if input_buffer == "/" {
        command_popup.filter("");
    } else if input_buffer.starts_with('/') {
        command_popup.filter(&input_buffer[1..]);
    } else {
        command_popup.hide();
    }
}

fn refresh_file_popup(
    file_popup: &mut FilePopup,
    input_buffer: &str,
    cursor_byte: usize,
    file_index: &[String],
) {
    if let Some((start, end)) = current_mention_range(input_buffer, cursor_byte) {
        let query = &input_buffer[start + 1..end];
        file_popup.filter(query, file_index);
    } else {
        file_popup.hide();
    }
}

fn collect_file_index(root: &Path) -> Vec<String> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.path();
            let relative = path.strip_prefix(root).ok()?;
            let display = relative.to_string_lossy().replace('\\', "/");
            if display.starts_with(".git/")
                || display.starts_with("target/")
                || display.starts_with("node_modules/")
            {
                return None;
            }
            Some(display)
        })
        .collect()
}

async fn sleep_until_reveal_tick() {
    sleep(STREAM_REVEAL_INTERVAL).await;
}

async fn sleep_until_status_tick() {
    sleep(Duration::from_millis(250)).await;
}

pub async fn run_app(
    mut state_rx: watch::Receiver<UiState>,
    mut stream_rx: tokio::sync::broadcast::Receiver<String>,
    mut event_rx: mpsc::UnboundedReceiver<ReactEvent>,
    cmd_tx: mpsc::UnboundedSender<crate::application::Command>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut event_handler = super::event::EventHandler::new(cmd_tx);
    let mut input_history = InputHistory::new(100);
    let mut command_popup = CommandPopup::new();
    let mut file_popup = FilePopup::new();
    let file_index = collect_file_index(Path::new("."));
    let mut input_buffer = String::new();
    let mut conversation_scroll = 0usize;
    let mut last_history_len = 0usize;
    let mut input_state = InputState::default();
    let mut live_state = LiveConversationState::default();
    let agents: Vec<&str> = DEFAULT_AGENTS.to_vec();
    let models: Vec<&str> = DEFAULT_MODELS.to_vec();
    let mut render_ctx = RenderContext::new();
    let mut state = state_rx.borrow().clone();

    loop {
        clamp_cursor(&mut input_state, &input_buffer);
        live_state.sync_with_state(&state);

        if state.history.len() != last_history_len {
            last_history_len = state.history.len();
            conversation_scroll = usize::MAX;
        }

        let mut scroll_info = (0usize, 0usize);
        terminal.draw(|f| {
            draw(
                f,
                &state,
                &input_buffer,
                conversation_scroll,
                &mut scroll_info,
                &input_state,
                &agents,
                &models,
                &mut render_ctx,
                &live_state,
                &mut command_popup,
                &mut file_popup,
            );
        })?;
        let (total_lines, viewport_height) = scroll_info;
        conversation_scroll = conversation_scroll.min(total_lines.saturating_sub(viewport_height));

        let terminal_width = terminal.size()?.width;
        let input_width = composer_text_width(terminal_width);

        tokio::select! {
            maybe_ev = event_handler.next_event() => {
                let Some(ev) = maybe_ev else { break; };
                input_state.update_cursor();
                match ev {
                    super::event::AppEvent::Command(cmd) => {
                        if matches!(cmd, crate::application::Command::Quit) {
                            break;
                        }
                    }
                    super::event::AppEvent::Mouse(mouse) => {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => {
                                conversation_scroll = conversation_scroll.saturating_sub(3);
                            }
                            MouseEventKind::ScrollDown => {
                                conversation_scroll = conversation_scroll.saturating_add(3);
                            }
                            _ => {}
                        }
                    }
                    super::event::AppEvent::Key(key) if !state.input_locked => {
                        if file_popup.is_visible() {
                            match key.code {
                                KeyCode::Enter | KeyCode::Tab => {
                                    if let Some(path) = file_popup.confirm().map(str::to_string) {
                                        if let Some((start, end)) =
                                            current_mention_range(&input_buffer, input_state.cursor_byte)
                                        {
                                            input_state.cursor_byte = replace_range(
                                                &mut input_buffer,
                                                start..end,
                                                &format!("@{} ", path),
                                            );
                                            input_state.preferred_column = None;
                                        }
                                        file_popup.hide();
                                    }
                                }
                                KeyCode::Up => file_popup.select_previous(),
                                KeyCode::Down => file_popup.select_next(),
                                KeyCode::Esc => file_popup.hide(),
                                KeyCode::Char(c) if input_state.focus == InputFocus::Input => {
                                    input_buffer.insert(input_state.cursor_byte, c);
                                    input_state.cursor_byte += c.len_utf8();
                                    input_state.preferred_column = None;
                                    refresh_command_popup(&mut command_popup, &input_buffer);
                                    refresh_file_popup(&mut file_popup, &input_buffer, input_state.cursor_byte, &file_index);
                                }
                                KeyCode::Backspace if input_state.focus == InputFocus::Input => {
                                    if input_state.cursor_byte > 0 {
                                        let prev = previous_boundary(&input_buffer, input_state.cursor_byte);
                                        input_buffer.replace_range(prev..input_state.cursor_byte, "");
                                        input_state.cursor_byte = prev;
                                        input_state.preferred_column = None;
                                        refresh_command_popup(&mut command_popup, &input_buffer);
                                        refresh_file_popup(&mut file_popup, &input_buffer, input_state.cursor_byte, &file_index);
                                    }
                                }
                                KeyCode::Delete if input_state.focus == InputFocus::Input => {
                                    if input_state.cursor_byte < input_buffer.len() {
                                        let next = next_boundary(&input_buffer, input_state.cursor_byte);
                                        input_buffer.replace_range(input_state.cursor_byte..next, "");
                                        input_state.preferred_column = None;
                                        refresh_command_popup(&mut command_popup, &input_buffer);
                                        refresh_file_popup(&mut file_popup, &input_buffer, input_state.cursor_byte, &file_index);
                                    }
                                }
                                _ => {}
                            }
                        } else if command_popup.is_visible() {
                            match key.code {
                                KeyCode::Enter | KeyCode::Tab => {
                                    if let Some(cmd) = command_popup.confirm() {
                                        input_buffer = cmd.to_string();
                                        input_state.cursor_byte = input_buffer.len();
                                        command_popup.hide();
                                    }
                                }
                                KeyCode::Up => command_popup.select_previous(),
                                KeyCode::Down => command_popup.select_next(),
                                KeyCode::Esc => command_popup.hide(),
                                KeyCode::Char(c) if input_state.focus == InputFocus::Input => {
                                    input_buffer.insert(input_state.cursor_byte, c);
                                    input_state.cursor_byte += c.len_utf8();
                                    input_state.preferred_column = None;
                                    refresh_command_popup(&mut command_popup, &input_buffer);
                                    refresh_file_popup(&mut file_popup, &input_buffer, input_state.cursor_byte, &file_index);
                                }
                                KeyCode::Backspace if input_state.focus == InputFocus::Input => {
                                    if input_state.cursor_byte > 0 {
                                        let prev = previous_boundary(&input_buffer, input_state.cursor_byte);
                                        input_buffer.replace_range(prev..input_state.cursor_byte, "");
                                        input_state.cursor_byte = prev;
                                        input_state.preferred_column = None;
                                        refresh_command_popup(&mut command_popup, &input_buffer);
                                        refresh_file_popup(&mut file_popup, &input_buffer, input_state.cursor_byte, &file_index);
                                    }
                                }
                                KeyCode::Delete if input_state.focus == InputFocus::Input => {
                                    if input_state.cursor_byte < input_buffer.len() {
                                        let next = next_boundary(&input_buffer, input_state.cursor_byte);
                                        input_buffer.replace_range(input_state.cursor_byte..next, "");
                                        input_state.preferred_column = None;
                                        refresh_command_popup(&mut command_popup, &input_buffer);
                                        refresh_file_popup(&mut file_popup, &input_buffer, input_state.cursor_byte, &file_index);
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Enter => {
                                    if key.modifiers.contains(KeyModifiers::SHIFT) && input_state.focus == InputFocus::Input {
                                        input_buffer.insert(input_state.cursor_byte, '\n');
                                        input_state.cursor_byte += 1;
                                        input_state.preferred_column = None;
                                    } else if input_state.focus == InputFocus::Input || input_state.focus == InputFocus::Send {
                                        let input = input_buffer.trim().to_string();
                                        if !input.is_empty() {
                                            if matches!(input.to_lowercase().as_str(), "/exit" | "quit") {
                                                break;
                                            }
                                            live_state.reset();
                                            input_history.push(input.clone());
                                            event_handler.send_submit(input);
                                            input_buffer.clear();
                                            input_state.cursor_byte = 0;
                                            input_state.preferred_column = None;
                                        }
                                    }
                                }
                                KeyCode::Left if input_state.focus == InputFocus::Input => {
                                    if let Some(range) = token_range_at_or_adjacent(&input_buffer, input_state.cursor_byte) {
                                        input_state.cursor_byte = range.start;
                                    } else {
                                        input_state.cursor_byte = previous_boundary(&input_buffer, input_state.cursor_byte);
                                    }
                                    input_state.preferred_column = None;
                                }
                                KeyCode::Right if input_state.focus == InputFocus::Input => {
                                    if let Some(range) = token_range_at_or_adjacent(&input_buffer, input_state.cursor_byte) {
                                        input_state.cursor_byte = range.end;
                                    } else {
                                        input_state.cursor_byte = next_boundary(&input_buffer, input_state.cursor_byte);
                                    }
                                    input_state.preferred_column = None;
                                }
                                KeyCode::Home if input_state.focus == InputFocus::Input => {
                                    input_state.cursor_byte = line_start(&input_buffer, input_state.cursor_byte);
                                    input_state.preferred_column = None;
                                }
                                KeyCode::End if input_state.focus == InputFocus::Input => {
                                    input_state.cursor_byte = line_end(&input_buffer, input_state.cursor_byte);
                                    input_state.preferred_column = None;
                                }
                                KeyCode::Backspace if input_state.focus == InputFocus::Input => {
                                    if input_state.cursor_byte > 0 {
                                        if let Some(range) = token_range_at_or_adjacent(&input_buffer, input_state.cursor_byte) {
                                            input_buffer.replace_range(range.clone(), "");
                                            input_state.cursor_byte = range.start;
                                        } else {
                                            let prev = previous_boundary(&input_buffer, input_state.cursor_byte);
                                            input_buffer.replace_range(prev..input_state.cursor_byte, "");
                                            input_state.cursor_byte = prev;
                                        }
                                        input_state.preferred_column = None;
                                    }
                                }
                                KeyCode::Delete if input_state.focus == InputFocus::Input => {
                                    if input_state.cursor_byte < input_buffer.len() {
                                        if let Some(range) = token_range_at_or_adjacent(&input_buffer, input_state.cursor_byte) {
                                            input_buffer.replace_range(range, "");
                                        } else {
                                            let next = next_boundary(&input_buffer, input_state.cursor_byte);
                                            input_buffer.replace_range(input_state.cursor_byte..next, "");
                                        }
                                        input_state.preferred_column = None;
                                    }
                                }
                                KeyCode::Up => {
                                    if input_state.focus == InputFocus::Input
                                        && (input_buffer.contains('\n') || input_buffer.chars().count() > input_width)
                                    {
                                        input_state.cursor_byte = move_cursor_visual_vertical(
                                            &input_buffer,
                                            input_state.cursor_byte,
                                            input_width,
                                            -1,
                                            &mut input_state.preferred_column,
                                        );
                                    } else if input_state.focus == InputFocus::Input
                                        && !input_buffer.contains('\n')
                                        && input_state.cursor_byte == input_buffer.len()
                                    {
                                        if let Some(prev) = input_history.previous(&input_buffer) {
                                            input_buffer = prev.clone();
                                            input_state.cursor_byte = input_buffer.len();
                                            input_state.preferred_column = None;
                                        }
                                    } else if input_state.focus == InputFocus::Agent {
                                        input_state.agent_index = input_state.agent_index.saturating_sub(1);
                                    } else if input_state.focus == InputFocus::Model {
                                        input_state.model_index = input_state.model_index.saturating_sub(1);
                                    } else {
                                        conversation_scroll = conversation_scroll.saturating_sub(1);
                                    }
                                }
                                KeyCode::Down => {
                                    if input_state.focus == InputFocus::Input
                                        && (input_buffer.contains('\n') || input_buffer.chars().count() > input_width)
                                    {
                                        input_state.cursor_byte = move_cursor_visual_vertical(
                                            &input_buffer,
                                            input_state.cursor_byte,
                                            input_width,
                                            1,
                                            &mut input_state.preferred_column,
                                        );
                                    } else if input_state.focus == InputFocus::Input
                                        && !input_buffer.contains('\n')
                                        && input_state.cursor_byte == input_buffer.len()
                                    {
                                        if let Some(next) = input_history.next() {
                                            input_buffer = next.clone();
                                            input_state.cursor_byte = input_buffer.len();
                                            input_state.preferred_column = None;
                                        } else if let Some(cached) = input_history.cancel() {
                                            input_buffer = cached;
                                            input_state.cursor_byte = input_buffer.len();
                                            input_state.preferred_column = None;
                                        }
                                    } else if input_state.focus == InputFocus::Agent {
                                        input_state.agent_index = (input_state.agent_index + 1).min(agents.len().saturating_sub(1));
                                    } else if input_state.focus == InputFocus::Model {
                                        input_state.model_index = (input_state.model_index + 1).min(models.len().saturating_sub(1));
                                    } else {
                                        conversation_scroll = conversation_scroll.saturating_add(1);
                                    }
                                }
                                KeyCode::Tab => {
                                    input_state.focus = match input_state.focus {
                                        InputFocus::Input => InputFocus::Agent,
                                        InputFocus::Agent => InputFocus::Model,
                                        InputFocus::Model => InputFocus::Send,
                                        InputFocus::Send => InputFocus::Input,
                                    };
                                }
                                KeyCode::BackTab => {
                                    input_state.focus = match input_state.focus {
                                        InputFocus::Input => InputFocus::Send,
                                        InputFocus::Agent => InputFocus::Input,
                                        InputFocus::Model => InputFocus::Agent,
                                        InputFocus::Send => InputFocus::Model,
                                    };
                                }
                                KeyCode::Char(c) if input_state.focus == InputFocus::Input => {
                                    input_buffer.insert(input_state.cursor_byte, c);
                                    input_state.cursor_byte += c.len_utf8();
                                    input_state.preferred_column = None;
                                    refresh_command_popup(&mut command_popup, &input_buffer);
                                    refresh_file_popup(&mut file_popup, &input_buffer, input_state.cursor_byte, &file_index);
                                }
                                KeyCode::PageUp => {
                                    conversation_scroll = conversation_scroll.saturating_sub(10);
                                }
                                KeyCode::PageDown => {
                                    conversation_scroll = conversation_scroll.saturating_add(10);
                                }
                                KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    event_handler.send_clear();
                                }
                                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    event_handler.send_cancel();
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            changed = state_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                state = state_rx.borrow().clone();
            }
            maybe_token = stream_rx.recv() => {
                if let Ok(token) = maybe_token {
                    live_state.set_live_response(token);
                }
            }
            maybe_event = event_rx.recv() => {
                if let Some(event) = maybe_event {
                    handle_react_event(&mut live_state, event);
                }
            }
            _ = sleep_until_reveal_tick(), if live_state.is_revealing() => {
                live_state.reveal_tick();
            }
            _ = sleep_until_status_tick(), if state.input_locked => {}
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
