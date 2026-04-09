use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;

use card_core::rules::GameRules;
use card_server::{RoomManager, WebSocketServer};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting card game server...");

    // Create room manager (without script loading for now)
    let rules = GameRules::default();
    let room_manager = Arc::new(RoomManager::new(rules));

    // Start WebSocket server
    let bind_addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let server = WebSocketServer::new(room_manager, bind_addr);

    info!("Server starting on {}", bind_addr);
    server.run().await?;

    Ok(())
}
