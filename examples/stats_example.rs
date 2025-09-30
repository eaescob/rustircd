//! Example demonstrating the enhanced STATS command functionality
//! 
//! This example shows how to:
//! 1. Use RFC 1459 compliant STATS commands
//! 2. Access module-specific STATS (like throttling)
//! 3. View real-time server statistics

use rustircd_core::{Config, Server, Result};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCD Enhanced STATS Command Example");
    println!("=====================================");
    
    // Create a configuration with throttling enabled
    let mut config = Config::default();
    
    // Enable the throttling module
    config.modules.enabled_modules.push("throttling".to_string());
    
    // Configure throttling settings for testing
    config.modules.throttling.enabled = true;
    config.modules.throttling.max_connections_per_ip = 3;
    config.modules.throttling.time_window_seconds = 60;
    config.modules.throttling.initial_throttle_seconds = 5;
    
    // Configure server settings
    config.server.name = "stats.example.com".to_string();
    config.server.description = "STATS Test Server".to_string();
    
    // Configure a simple port
    config.connection.ports.clear();
    config.connection.ports.push(rustircd_core::config::PortConfig {
        port: 6667,
        connection_type: rustircd_core::config::PortConnectionType::Client,
        tls: false,
        description: Some("STATS test port".to_string()),
    });
    
    println!("Configuration:");
    println!("  Server: {}", config.server.name);
    println!("  Throttling enabled: {}", config.modules.throttling.enabled);
    println!();
    
    // Create and initialize the server
    let mut server = Server::new(config);
    server.init().await?;
    
    println!("Server initialized with enhanced STATS support");
    println!("Starting server on port 6667...");
    
    // Start the server in the background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("Server error: {}", e);
        }
    });
    
    // Give the server time to start
    sleep(Duration::from_secs(1)).await;
    
    println!("Server started successfully!");
    println!();
    println!("Available STATS commands:");
    println!("========================");
    println!();
    println!("RFC 1459 Standard STATS:");
    println!("  /STATS l  - List of server links");
    println!("  /STATS m  - Commands usage statistics");
    println!("  /STATS o  - List of operators currently online");
    println!("  /STATS u  - Server uptime");
    println!("  /STATS y  - Class information");
    println!("  /STATS c  - Connection information");
    println!();
    println!("Module-specific STATS:");
    println!("  /STATS T  - Throttling module statistics (IPs, stages, remaining time)");
    println!();
    println!("Example IRC client commands to test:");
    println!("  /connect localhost 6667");
    println!("  /nick testuser");
    println!("  /user testuser 0 * :Test User");
    println!("  /stats T");
    println!("  /stats m");
    println!("  /stats u");
    println!();
    println!("The throttling module will show:");
    println!("  - IP addresses being tracked");
    println!("  - Current throttling stage");
    println!("  - Remaining throttle time in seconds");
    println!("  - Number of active connections");
    println!();
    println!("Try connecting multiple times quickly to see throttling in action!");
    println!("Then use /STATS T to see the throttling statistics.");
    println!();
    println!("Press Ctrl+C to stop the server");
    
    // Wait for the server to finish (or be interrupted)
    server_handle.await??;
    
    Ok(())
}

/// Helper function to demonstrate different STATS configurations
#[allow(dead_code)]
fn show_stats_examples() {
    println!("STATS Command Examples:");
    println!("======================");
    println!();
    println!("1. Standard RFC 1459 STATS:");
    println!("   /stats l    - Show server links");
    println!("   /stats m    - Show command usage");
    println!("   /stats o    - Show online operators");
    println!("   /stats u    - Show server uptime");
    println!("   /stats y    - Show class information");
    println!("   /stats c    - Show connection info");
    println!();
    println!("2. Module-specific STATS:");
    println!("   /stats T    - Throttling module (IPs, stages, times)");
    println!("   /stats C    - Channel module (if implemented)");
    println!("   /stats I    - IRCv3 module (if implemented)");
    println!();
    println!("3. Expected output format:");
    println!("   :server 211 * l server.example.com 0 0 0 0 0 0");
    println!("   :server 212 * PRIVMSG 150 15000 0");
    println!("   :server 244 * T 192.168.1.100 THROTTLED stage=2 remaining=45s");
    println!("   :server 219 * T :End of STATS report");
}
