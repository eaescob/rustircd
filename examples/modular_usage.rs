//! Example showing modular IRC daemon usage

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
    
    // Print loaded modules
    println!("Enabled modules: {:?}", config.modules.enabled_modules);
    println!("Super servers: {:?}", config.network.super_servers);
    
    // Create and initialize server
    let mut server = Server::new(config).await;
    server.init().await?;
    
    // Start server
    tracing::info!("Starting modular IRC daemon...");
    server.start().await?;
    
    // Keep the server running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
