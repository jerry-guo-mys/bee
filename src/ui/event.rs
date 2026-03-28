//! 事件处理

use std::thread;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent};
use tokio::sync::mpsc;

use crate::application::orchestrator::Command;

/// 应用事件
#[derive(Debug, Clone)]
pub enum AppEvent {
    Command(Command),
    Key(KeyEvent),
    Mouse(MouseEvent),
}

/// 事件处理器
pub struct EventHandler {
    cmd_tx: mpsc::UnboundedSender<Command>,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(cmd_tx: mpsc::UnboundedSender<Command>) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        thread::spawn(move || loop {
            match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    if event_tx.send(AppEvent::Key(key)).is_err() {
                        break;
                    }
                }
                Ok(Event::Mouse(mouse)) => {
                    if event_tx.send(AppEvent::Mouse(mouse)).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        });

        Self { cmd_tx, event_rx }
    }

    pub async fn next_event(&mut self) -> Option<AppEvent> {
        let event = self.event_rx.recv().await?;
        Some(match event {
            AppEvent::Key(key) => self.handle_key(key),
            other => other,
        })
    }

    fn handle_key(&self, key: KeyEvent) -> AppEvent {
        match key.code {
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.cmd_tx.send(Command::Cancel);
                AppEvent::Command(Command::Cancel)
            }
            KeyCode::Char('l')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                let _ = self.cmd_tx.send(Command::Clear);
                AppEvent::Command(Command::Clear)
            }
            KeyCode::Esc => AppEvent::Command(Command::Cancel),
            KeyCode::Char('q')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                AppEvent::Command(Command::Quit)
            }
            _ => AppEvent::Key(key),
        }
    }

    pub fn send_submit(&self, input: String) {
        let _ = self.cmd_tx.send(Command::Submit(input));
    }

    pub fn send_clear(&self) {
        let _ = self.cmd_tx.send(Command::Clear);
    }

    pub fn send_cancel(&self) {
        let _ = self.cmd_tx.send(Command::Cancel);
    }
}
