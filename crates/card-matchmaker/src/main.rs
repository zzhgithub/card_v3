use anyhow::Result;
mod server;
use server::MatchmakerServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let server = MatchmakerServer::new("0.1.0".to_string(), 9090);
    server.run().await
}
