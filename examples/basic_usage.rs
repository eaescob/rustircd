//! Basic usage example for Rust IRC Daemon

use rustircd_core::{Config, Server};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    // Load configuration
    let config = Config::from_file("config.toml")?;
    
    // Validate configuration
    config.validate()?;
    
    // Create and initialize server
    let mut server = Server::new(config);
    server.init().await?;
    
    // Start server
    tracing::info!("Starting Rust IRC Daemon...");
    server.start().await?;
    
    // Keep the server running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
