//! Bee Admin：仅管理类 REST API（默认 8081，环境变量 `BEE_ADMIN_PORT`）
//!
//! 启动: `cargo run --bin bee-admin --features web`

#![cfg(feature = "web")]

#[path = "../web/server.rs"]
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::run_admin_server().await
}
