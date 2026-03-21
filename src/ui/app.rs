//! Simplified TUI main loop

use std::io::{self, Stdout};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::watch;

use crate::core::UiState;
use crate::ui::render::{draw, RenderContext};
use crate::ui::widgets::{CommandPopup, InputFocus, InputHistory, InputState};

const DEFAULT_AGENTS: &[&str] = &["默认", "自动分派"];
const DEFAULT_MODELS: &[&str] = &["默认", "DeepSeek", "GPT-4o", "Claude"];

pub async fn run_app(
    state_rx: watch::Receiver<UiState>,
    mut stream_rx: tokio::sync::broadcast::Receiver<String>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<crate::core::Command>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let event_handler = super::event::EventHandler::new(cmd_tx);
    let mut input_history = InputHistory::new(100);
    let mut command_popup = CommandPopup::new();
    let mut input_buffer = String::new();
    let mut conversation_scroll = 0usize;
    let mut last_history_len = 0usize;
    let mut input_state = InputState::default();
    let agents: Vec<&str> = DEFAULT_AGENTS.to_vec();
    let models: Vec<&str> = DEFAULT_MODELS.to_vec();
    let mut render_ctx = RenderContext::new();

    loop {
        input_state.update_cursor();
        let state = state_rx.borrow().clone();

        while let Ok(token) = stream_rx.try_recv() {
            tracing::debug!("Stream: {}", token);
        }

        if state.history.len() != last_history_len {
            last_history_len = state.history.len();
            conversation_scroll = usize::MAX;
        }

        if let Ok(Some(ev)) = event_handler.poll() {
            match ev {
                super::event::AppEvent::Command(cmd) => {
                    if matches!(cmd, crate::core::Command::Quit) {
                        break;
                    }
                }
                super::event::AppEvent::Key(key) if !state.input_locked => {
                    if command_popup.is_visible() {
                        match key.code {
                            KeyCode::Enter => {
                                if let Some(cmd) = command_popup.confirm() {
                                    input_buffer = cmd.to_string();
                                    command_popup.hide();
                                }
                            }
                            KeyCode::Up => command_popup.select_previous(),
                            KeyCode::Down => command_popup.select_next(),
                            KeyCode::Esc | KeyCode::Tab => command_popup.hide(),
                            KeyCode::Char(c) => {
                                input_buffer.push(c);
                                if input_buffer.starts_with('/') {
                                    command_popup.filter(&input_buffer[1..]);
                                } else {
                                    command_popup.hide();
                                }
                            }
                            KeyCode::Backspace => {
                                input_buffer.pop();
                                if input_buffer.starts_with('/') {
                                    command_popup.filter(&input_buffer[1..]);
                                } else {
                                    command_popup.hide();
                                }
                            }
                            _ => {}
                        }
                        terminal.draw(|f| {
                            draw(
                                f,
                                &state,
                                &input_buffer,
                                conversation_scroll,
                                &mut (0, 0),
                                &input_state,
                                &agents,
                                &models,
                                &mut render_ctx,
                                &mut command_popup,
                            );
                        })?;
                        continue;
                    }

                    match key.code {
                        KeyCode::Enter => {
                            if input_state.focus == InputFocus::Input
                                || input_state.focus == InputFocus::Send
                            {
                                let input = input_buffer.trim().to_string();
                                if !input.is_empty() {
                                    if matches!(input.to_lowercase().as_str(), "/exit" | "quit") {
                                        break;
                                    }
                                    input_history.push(input.clone());
                                    event_handler.send_submit(input);
                                    input_buffer.clear();
                                }
                            }
                        }
                        KeyCode::Up => {
                            if input_state.focus == InputFocus::Input {
                                if let Some(prev) = input_history.previous(&input_buffer) {
                                    input_buffer = prev.clone();
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
                            if input_state.focus == InputFocus::Input {
                                if let Some(next) = input_history.next() {
                                    input_buffer = next.clone();
                                } else if let Some(cached) = input_history.cancel() {
                                    input_buffer = cached;
                                }
                            } else if input_state.focus == InputFocus::Agent {
                                input_state.agent_index =
                                    (input_state.agent_index + 1).min(agents.len().saturating_sub(1));
                            } else if input_state.focus == InputFocus::Model {
                                input_state.model_index =
                                    (input_state.model_index + 1).min(models.len().saturating_sub(1));
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
                        KeyCode::Backspace if input_state.focus == InputFocus::Input => {
                            input_buffer.pop();
                        }
                        KeyCode::Char(c) if input_state.focus == InputFocus::Input => {
                            input_buffer.push(c);
                            if input_buffer == "/" {
                                command_popup.filter("");
                            } else if input_buffer.starts_with('/') {
                                command_popup.filter(&input_buffer[1..]);
                            }
                        }
                        KeyCode::PageUp => {
                            conversation_scroll = conversation_scroll.saturating_sub(10);
                        }
                        KeyCode::PageDown => {
                            conversation_scroll = conversation_scroll.saturating_add(10);
                        }
                        KeyCode::Home => conversation_scroll = 0,
                        KeyCode::End => conversation_scroll = usize::MAX,
                        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            event_handler.send_clear();
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            event_handler.send_cancel();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
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
                &mut command_popup,
            );
        })?;
        let (total_lines, viewport_height) = scroll_info;
        conversation_scroll = conversation_scroll.min(total_lines.saturating_sub(viewport_height));
        tokio::task::yield_now().await;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
