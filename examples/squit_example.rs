//! Example demonstrating SQUIT command usage
//! 
//! This example shows how to use the SQUIT command with proper operator validation.
//! The S flag must be set in the operator configuration for SQUIT to work.

use rustircd_core::{Config, Server, Result};
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("SQUIT Command Example");
    println!("====================");
    println!();
    println!("This example demonstrates the SQUIT command implementation.");
    println!("The SQUIT command requires:");
    println!("1. User must be an operator (authenticated with OPER command)");
    println!("2. Operator must have the 'S' flag (Squit) in their configuration");
    println!("3. Target server must be connected");
    println!();
    
    // Load configuration
    let config = Config::from_file("examples/configs/squit_operator.toml")?;
    let config = Arc::new(config);
    
    println!("Configuration loaded successfully!");
    println!("Available operators with SQUIT privileges:");
    
    // Display operators with SQUIT flag
    for operator in &config.network.operators {
        if operator.can_squit() {
            println!("  - {} (hostmask: {})", operator.nickname, operator.hostmask);
            println!("    Flags: {:?}", operator.flags);
        }
    }
    
    println!();
    println!("To test SQUIT command:");
    println!("1. Connect to the server");
    println!("2. Authenticate as an operator with S flag:");
    println!("   OPER admin admin123");
    println!("3. Issue SQUIT command:");
    println!("   SQUIT remote.server.com Server maintenance");
    println!();
    println!("Note: Only operators with the 'S' flag can use SQUIT.");
    println!("Operators without the S flag will receive ERR_NOPRIVILEGES.");
    
    // Create and start server
    let server = Server::new(config)?;
    
    println!();
    println!("Starting server...");
    println!("Connect with: telnet localhost 6667");
    println!("Press Ctrl+C to stop the server");
    
    // Start the server
    server.start().await?;
    
    Ok(())
}
