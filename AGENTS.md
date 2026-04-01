# Agent Guidelines for Bee

Guidelines for AI coding agents working in this Rust personal AI agent system.

## Build Commands

```bash
cargo run                          # Run TUI (default)
cargo run --bin bee-web --features web
cargo run --bin bee-admin --features web
cargo run --bin bee-whatsapp --features whatsapp
cargo run --bin bee-lark --features lark
cargo run --bin bee-gateway --features gateway
cargo run --bin bee-evolution      # Evolution testing
cargo build --release              # Optimized release build
cargo check                        # Fast type check (no codegen)
cargo build --features browser     # Build with browser support
```

## Test Commands

```bash
cargo test                         # Run all tests
cargo test test_name               # Run single test by name
cargo test test_name -- --nocapture  # Run with output
cargo test module_name::           # Run by module pattern
cargo test -- --ignored            # Run ignored tests
cargo test -- --test-threads=1     # Run tests sequentially
```

**Running a single test:**
```bash
cargo test test_parse_input        # Run test by exact name
cargo test code_edit::tests::      # Run all tests in module
cargo test --test integration      # Run integration tests only
```

## Lint/Format

```bash
cargo clippy                       # Run linter
cargo clippy -- -D warnings        # Fail on warnings
cargo fmt                          # Format code
cargo fmt -- --check               # Check formatting (CI)
```

## Code Style

### Imports (grouped with blank lines)
```rust
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;
use anyhow::Context;

use crate::config::AppConfig;
use crate::core::AgentError;
```

### Naming Conventions
- **Types/Structs/Enums/Traits**: `PascalCase` (`AgentError`, `ToolExecutor`)
- **Functions/Methods/Variables**: `snake_case` (`create_agent`, `ctx_manager`)
- **Constants**: `SCREAMING_SNAKE_CASE` (`MAX_RETRIES`)
- **Modules**: `snake_case` (`memory`, `code_edit`)
- **Test functions**: `test_` prefix (`test_parse_input`)

### Formatting
- Max line length: ~100 chars (rustfmt default)
- Trailing commas in multi-line structs/enums
- Chain methods on separate lines when long

### Error Handling
- Use `thiserror` for error enums in `core/error.rs`
- Use `anyhow` for application errors
- Prefer `?` over `.unwrap()` in production code

```rust
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),
}
```

### Async Patterns & Testing
- Use `tokio` runtime with `#[tokio::main]` and `async-trait` for async traits
- Use `tokio::sync` primitives (mpsc, broadcast, CancellationToken)
- Tests are inline in `#[cfg(test)]` modules:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_feature() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { /* async test code */ });
    }
}
```

### Documentation & Logging
- Use `//!` for module-level docs (Chinese preferred), `///` for item-level docs
- Use `tracing` crate for logging:
```rust
tracing::info!("Processing: {}", value);
tracing::warn!("Condition met: {:?}", condition);
tracing::error!("Error: {:?}", err);
```

## Project Structure
```
src/
├── main.rs          # TUI entry point
├── lib.rs           # Library exports
├── bin/             # Additional binaries (web, whatsapp, etc.)
├── core/            # Orchestrator, state, recovery, error
├── llm/             # LLM clients (OpenAI, DeepSeek, Mock)
├── memory/          # Short/long-term memory, persistence
├── react/           # Planner, Critic, ReAct loop
├── tools/           # Tool implementations & executor
├── skills/          # Skills system
├── ui/              # Ratatui TUI components
├── config/          # Configuration loading
├── workflow/        # Workflow engine & graph
├── gateway/         # WebSocket gateway
├── evolution/       # Evolution/testing engine
├── integrations/    # WhatsApp, Lark integrations
└── observability/   # Tracing & logging
```

## Cargo Features
- `web` - Web server with streaming
- `whatsapp` - WhatsApp integration
- `lark` - Lark integration  
- `gateway` - WebSocket gateway
- `browser` - Headless Chrome control
- `async-sqlite` - Async SQLite (sqlx)

```rust
#[cfg(feature = "browser")]
use crate::tools::BrowserTool;
```

## Key Dependencies
- `tokio` - Async runtime
- `anyhow`/`thiserror` - Error handling
- `serde`/`serde_json` - Serialization
- `async-openai` - OpenAI/DeepSeek clients
- `ratatui`/`crossterm` - TUI
- `axum`/`tower` - Web server
- `rusqlite`/`sqlx` - SQLite persistence
- `tracing-subscriber` - Logging
