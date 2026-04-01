//! Bee Web UI：对话、静态资源与管理 API（合并路由，兼容原 8080 行为）
//!
//! 启动: `cargo run --bin bee-web --features web`

#![cfg(feature = "web")]

mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::run_web_server().await
}
