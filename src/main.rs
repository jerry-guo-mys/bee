//! Bee - Rust personal agent system

use anyhow::Context;
use bee::{application::create_agent, ui::run_app};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn init_logging() -> WorkerGuard {
    let _ = std::fs::create_dir_all("logs");
    let file_appender = tracing_appender::rolling::never("logs", "bee-tui.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
        .init();

    guard
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _log_guard = init_logging();

    let _ = std::fs::create_dir_all("workspace");
    let _ = std::fs::create_dir_all("config/prompts");

    let (cmd_tx, state_rx, stream_rx, event_rx) =
        create_agent(None).await.context("Failed to create agent")?;

    run_app(state_rx, stream_rx, event_rx, cmd_tx)
        .await
        .context("App run failed")?;

    Ok(())
}
