# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo run                          # Run TUI (default)
cargo run --bin bee-web --features web
cargo run --bin bee-admin --features web   # 仅管理 REST，默认 8081（BEE_ADMIN_PORT）
cargo run --bin bee-whatsapp --features whatsapp
cargo run --bin bee-lark --features lark
cargo run --bin bee-gateway --features gateway
cargo run --bin bee-evolution      # Evolution testing
cargo build --release              # Production build
cargo check                        # Fast type check
cargo build --features browser     # With browser support
cargo build --features async-sqlite  # Async SQLite persistence

cargo test                         # Run all tests
cargo test test_name               # Single test by name
cargo test test_name -- --nocapture  # With output
cargo test module_name::           # By module pattern
cargo test -- --ignored            # Ignored tests
cargo test -- --test-threads=1     # Sequential execution

cargo clippy                       # Linter
cargo clippy -- -D warnings        # Fail on warnings
cargo fmt                          # Format code
cargo fmt -- --check               # Check formatting (CI)
```

## Architecture Overview

**Bee** is a Rust-based personal AI agent system built on the ReAct architecture (planner/critic dual-core with 20-step loop limit) featuring hierarchical memory, RAG retrieval enhancement, multi-tool collaboration, skill plugins, and self-evolution capabilities.

### Layered Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                    Interface Layer                             │
│   TUI(Ratatui) │ Web(Axum SSE) │ WhatsApp │ Lark │ Gateway   │
└───────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                    Headless Agent Runtime                      │
│         create_agent → process_message                         │
└───────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                    Core Orchestrator                           │
│  AgentBuilder │ Session Supervisor │ Recovery │ TaskScheduler │
└───────────────────────────────────────────────────────────────┘
                              │
    ┌─────────────────────────┼─────────────────────────┐
    ▼                         ▼                         ▼
┌────────────────┐  ┌──────────────────┐  ┌────────────────────┐
│  Cognitive     │  │     Tool         │  │     Memory         │
│  Planner       │  │  Sandbox FS      │  │  Short-term(conv)  │
│  Critic        │  │  Shell whitelist │  │  Mid-term(workspace)│
│  ReAct Loop    │  │  Code R/W/Edit   │  │  Long-term(file+vector)│
│  (20 steps)    │  │  Git/Diff/Commit │  │  User memory        │
│                │  │  Web/DeepSearch  │  │  Learnings          │
│                │  │  Knowledge Graph │  │  RAG Pipeline       │
└────────────────┘  └──────────────────┘  └────────────────────┘
                              │
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                    Foundation Layer                            │
│  Multi-model routing (DeepSeek/OpenAI/Claude/Gemini/Qwen)     │
│  Skill system (TOML definitions, dynamic loading)             │
│  Self-evolution engine (Analyze→Plan→Execute→Commit)          │
│  SQLite (rusqlite/sqlx) + File persistence                    │
└───────────────────────────────────────────────────────────────┘
```

### Source Structure

- `src/main.rs` - TUI entry point
- `src/lib.rs` - Library exports
- `src/agent.rs` - Headless Agent runtime (for non-TUI frontends)
- `src/config.rs` - Configuration loading
- `src/bin/` - Additional binaries (web, whatsapp, lark, gateway, evolution)
- `src/core/` - Orchestrator, builder, session_supervisor, recovery, state, error
- `src/llm/` - LLM clients (deepseek, openai, router, embedding, mock)
- `src/memory/` - Conversation, working, long_term, user_memory, learnings, rag, tokenizer, token_budget
- `src/react/` - ReAct loop, planner, critic, events
- `src/skills/` - loader, selector (TOML-based skill system)
- `src/tools/` - 27 tools (filesystem, shell, search, deep_search, code_*, git_*, browser, etc.)
- `src/evolution/` - Self-evolution engine (analyzer, planner, executor, engine, loop)
- `src/gateway/` - WebSocket gateway (hub-spoke architecture, session store, task queue)
- `src/integrations/` - WhatsApp, Lark APIs
- `src/workflow/` - Workflow engine (DAG-based, sequential/parallel/conditional dependencies)
- `src/ui/` - TUI components (Ratatui)
- `src/observability/` - Metrics, tracing

### Configuration

| File | Description |
|------|-------------|
| `config/default.toml` | Main config (LLM, tools, memory, evolution, security) |
| `config/models.toml` | Multi-model registry |
| `config/assistants.toml` | Multi-assistant definitions |
| `config/prompts/` | System prompt templates |
| `config/skills/` | Skill definitions (TOML + capability.md + templates) |

### Key Design Patterns

- **AgentBuilder**: Unified agent construction with custom prompts and config injection
- **Multi-model routing**: Auto-select optimal model by task type (code/reasoning/summary)
- **RAG Pipeline**: Document chunking → Vector store → Hybrid search (vector+keyword RRF) → Context enhancement
- **Smart Pruning**: Preserve system messages, prefer removing tool outputs, Token Budget management
- **Skill System**: TOML-defined skills with SkillSelector for task-based selection
- **Self-Evolution**: Analyzer → Planner → Executor → Git commit cycle
- **Gateway Hub-Spoke**: WebSocket gateway with session persistence, task queue, user memory

### Code Style (from AGENTS.md)

- **Imports**: Grouped with blank lines (std → external → crate)
- **Naming**: `PascalCase` for types, `snake_case` for functions/variables, `SCREAMING_SNAKE_CASE` for constants
- **Error Handling**: `thiserror` for error enums, `anyhow` for application errors, prefer `?` over `.unwrap()`
- **Async**: `tokio` runtime, `#[tokio::main]`, `async-trait`, inline `#[cfg(test)]` modules
- **Logging**: `tracing` crate (`info!`, `warn!`, `error!`)
- **Line length**: ~100 chars (rustfmt default)

### Feature Flags

- `web` - Web server with streaming (Axum SSE)
- `whatsapp` - WhatsApp integration (Webhook)
- `lark` - Lark/飞书 integration (Webhook)
- `gateway` - WebSocket gateway (includes async-sqlite)
- `browser` - Headless Chrome control
- `async-sqlite` - Async SQLite with sqlx

### Environment Variables

| Variable | Description |
|----------|-------------|
| `DEEPSEEK_API_KEY` | DeepSeek API Key (recommended) |
| `DEEPSEEK_MODEL` | `deepseek-chat` or `deepseek-reasoner` |
| `OPENAI_API_KEY` | OpenAI API Key |
| `ANTHROPIC_API_KEY` | Claude API Key |
| `GOOGLE_API_KEY` | Gemini API Key |
| `DASHSCOPE_API_KEY` | Qwen API Key |
| `MOONSHOT_API_KEY` | Kimi API Key |
| `ZHIPU_API_KEY` | GLM API Key |
| `TASK_PERSISTENCE` / `BEE_TASK_PERSISTENCE` | 任务存储：`json`（默认）\|`sql`\|`dual_write`；见 `task_repository` |
