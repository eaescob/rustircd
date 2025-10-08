//! Example demonstrating the MOTD (Message of the Day) functionality
//! 
//! This example shows how to:
//! 1. Configure MOTD file in server configuration
//! 2. Create and format MOTD files
//! 3. Test MOTD display during registration and via command
//! 4. Handle missing MOTD files gracefully

use rustircd_core::{Config, Server, Result};
use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCD MOTD Example");
    println!("====================");
    
    // Create a sample MOTD file
    create_sample_motd_file().await?;
    
    // Create a configuration with MOTD enabled
    let mut config = Config::default();
    
    // Configure server settings
    config.server.name = "motd.example.com".to_string();
    config.server.description = "MOTD Test Server".to_string();
    
    // Configure MOTD file (relative path example)
    config.server.motd_file = Some("sample_motd.txt".to_string());
    
    // Alternative absolute path examples:
    // config.server.motd_file = Some("/etc/rustircd/motd.txt".to_string());  // Unix
    // config.server.motd_file = Some("C:\\Program Files\\RustIRCd\\motd.txt".to_string());  // Windows
    
    // Configure a simple port
    config.connection.ports.clear();
    config.connection.ports.push(rustircd_core::config::PortConfig {
        port: 6667,
        connection_type: rustircd_core::config::PortConnectionType::Client,
        tls: false,
        description: Some("MOTD test port".to_string()),
        bind_address: None,
    });
    
    println!("Configuration:");
    println!("  Server: {}", config.server.name);
    println!("  MOTD file: {:?}", config.server.motd_file);
    println!();
    
    // Create and initialize the server
    let mut server = Server::new(config).await;
    server.init().await?;
    
    println!("Server initialized with MOTD support");
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
    println!("MOTD Features:");
    println!("==============");
    println!();
    println!("1. Automatic MOTD Display:");
    println!("   - MOTD is shown automatically after user registration");
    println!("   - Appears after the welcome message");
    println!("   - Uses RPL_MOTDSTART, RPL_MOTD, and RPL_ENDOFMOTD replies");
    println!();
    println!("2. Manual MOTD Command:");
    println!("   - Users can request MOTD with /MOTD command");
    println!("   - Shows the same content as automatic display");
    println!("   - Useful for re-reading the MOTD");
    println!();
    println!("3. Graceful Handling:");
    println!("   - If MOTD file doesn't exist, shows ERR_NOMOTD");
    println!("   - If MOTD file is empty, shows ERR_NOMOTD");
    println!("   - No server errors for missing files");
    println!();
    println!("Test Scenarios:");
    println!("===============");
    println!();
    println!("Scenario 1: Normal Registration with MOTD");
    println!("  /connect localhost 6667");
    println!("  /nick testuser");
    println!("  /user testuser 0 * :Test User");
    println!("  # MOTD will be displayed automatically after welcome message");
    println!();
    println!("Scenario 2: Manual MOTD Request");
    println!("  /motd");
    println!("  # Will show the same MOTD content");
    println!();
    println!("Scenario 3: Missing MOTD File");
    println!("  # Change config to point to non-existent file");
    println!("  # Restart server and connect");
    println!("  # Will show 'MOTD file is missing' message");
    println!();
    println!("MOTD File Format:");
    println!("================");
    println!("• Plain text file with one message per line");
    println!("• No special formatting required");
    println!("• Lines are displayed with ':- ' prefix");
    println!("• Empty lines are preserved");
    println!("• Maximum line length recommended: 400 characters");
    println!();
    println!("Example MOTD Content:");
    println!("  Welcome to RustIRCd!");
    println!("  ====================");
    println!("  ");
    println!("  This server features:");
    println!("  • RFC 1459 compliance");
    println!("  • Enhanced security");
    println!("  • Modern features");
    println!("  ");
    println!("  Have a great time!");
    println!();
    println!("Configuration Options:");
    println!("=====================");
    println!("• motd_file: Path to MOTD file (optional)");
    println!("• Set to None to disable MOTD completely");
    println!("• Supports both relative and absolute paths");
    println!("• Relative paths resolved from server working directory");
    println!("• Absolute paths used as-is");
    println!("• File is read once at server startup");
    println!();
    println!("Path Examples:");
    println!("• Relative: \"motd.txt\"");
    println!("• Relative: \"config/messages/motd.txt\"");
    println!("• Absolute (Unix): \"/etc/rustircd/motd.txt\"");
    println!("• Absolute (Windows): \"C:\\\\Program Files\\\\RustIRCd\\\\motd.txt\"");
    println!();
    println!("Press Ctrl+C to stop the server");
    
    // Wait for the server to finish (or be interrupted)
    let _ = server_handle.await;
    
    Ok(())
}

/// Create a sample MOTD file for testing
async fn create_sample_motd_file() -> Result<()> {
    let motd_content = r#"Welcome to RustIRCd!
====================

This is a sample MOTD file demonstrating the MOTD system.

Features:
• RFC 1459 compliant IRC protocol
• Enhanced STATS system with security controls
• Connection throttling and rate limiting
• Configurable replies and MOTD system
• IRCv3 capabilities and extensions

Commands to try:
• /motd - Display this message again
• /whois <nick> - Get user information
• /list - List available channels
• /stats - Show server statistics (operators only)

Rules:
• Be respectful to other users
• No spam or flooding
• Follow channel rules

Enjoy your stay on RustIRCd!

For more information:
https://github.com/rustircd/rustircd"#;

    fs::write("sample_motd.txt", motd_content)
        .map_err(|e| rustircd_core::Error::Config(format!("Failed to create sample MOTD file: {}", e)))?;

    println!("Created sample MOTD file: sample_motd.txt");
    Ok(())
}

/// Helper function to demonstrate different MOTD configurations
#[allow(dead_code)]
fn show_motd_examples() {
    println!("MOTD Configuration Examples:");
    println!("===========================");
    println!();
    println!("1. Basic MOTD (relative path):");
    println!("   motd_file = \"motd.txt\"");
    println!();
    println!("2. Custom Relative Path:");
    println!("   motd_file = \"config/messages/motd.txt\"");
    println!();
    println!("3. Absolute Path (Unix/Linux):");
    println!("   motd_file = \"/etc/rustircd/motd.txt\"");
    println!();
    println!("4. Absolute Path (Windows):");
    println!("   motd_file = \"C:\\\\Program Files\\\\RustIRCd\\\\motd.txt\"");
    println!();
    println!("5. Disable MOTD:");
    println!("   motd_file = null");
    println!();
    println!("6. Dynamic MOTD:");
    println!("   # Use a script to generate MOTD file at startup");
    println!("   # Include server uptime, user count, etc.");
    println!();
    println!("Expected IRC Output:");
    println!("===================");
    println!(":server 375 * :- server.example.com Message of the Day -");
    println!(":server 372 * :- Welcome to RustIRCd!");
    println!(":server 372 * :- ====================");
    println!(":server 372 * :- ");
    println!(":server 372 * :- This server features:");
    println!(":server 372 * :- • RFC 1459 compliance");
    println!(":server 376 * :End of /MOTD command.");
    println!();
    println!("No MOTD File:");
    println!(":server 422 * :MOTD file is missing");
}
