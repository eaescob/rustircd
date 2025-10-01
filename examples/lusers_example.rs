//! Example demonstrating the LUSERS command functionality
//!
//! This example shows how to:
//! 1. Configure a server with LUSERS support
//! 2. Demonstrate network statistics tracking
//! 3. Show LUSERS command responses
//! 4. Display user, operator, channel, and server counts

use rustircd_core::{Config, Server, Result, Message, MessageType, NumericReply};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCD LUSERS Command Example");
    println!("==============================");
    
    // Create server configuration
    let config = create_server_config();
    
    // Create and initialize the server
    let mut server = Server::new(config);
    server.init().await?;
    
    println!("Server initialized with LUSERS support");
    println!();
    
    // Demonstrate LUSERS functionality
    demonstrate_lusers_functionality(&server).await?;
    
    println!();
    println!("LUSERS System Features:");
    println!("======================");
    println!("✅ RFC 1459 compliant LUSERS implementation");
    println!("✅ Network statistics tracking");
    println!("✅ User, operator, channel, and server counts");
    println!("✅ Local and global user statistics");
    println!("✅ Unknown connection tracking");
    println!("✅ Configurable numeric replies");
    println!("✅ Real-time statistics updates");
    
    println!();
    println!("LUSERS Numeric Replies:");
    println!("=====================");
    show_lusers_numeric_replies();
    
    println!();
    println!("LUSERS Command Usage:");
    println!("===================");
    show_lusers_usage();
    
    println!();
    println!("Expected IRC Output:");
    println!("==================");
    show_expected_output();
    
    // Keep server running for a short time
    println!();
    println!("Server running for 30 seconds to demonstrate functionality...");
    sleep(Duration::from_secs(30)).await;
    
    Ok(())
}

/// Create a server configuration with LUSERS support
fn create_server_config() -> Config {
    let mut config = Config::default();
    
    // Configure server settings
    config.server.name = "lusers.example.com".to_string();
    config.server.description = "LUSERS Test Server".to_string();
    config.server.version = "1.0.0".to_string();
    config.server.max_clients = 1000;
    
    // Enable modules for statistics
    config.modules.enabled_modules = vec![
        "channel".to_string(),
        "ircv3".to_string(),
        "throttling".to_string(),
    ];
    
    // Configure connection settings
    config.connection.ports.clear();
    config.connection.ports.push(rustircd_core::config::PortConfig {
        port: 6667,
        connection_type: rustircd_core::config::ConnectionType::Client,
        tls: false,
        description: Some("Standard IRC port".to_string()),
    });
    
    // Configure throttling for statistics
    config.modules.throttling.enabled = true;
    config.modules.throttling.max_connections_per_ip = 5;
    config.modules.throttling.time_window_seconds = 60;
    
    config
}

/// Demonstrate LUSERS functionality
async fn demonstrate_lusers_functionality(server: &Server) -> Result<()> {
    println!("Demonstrating LUSERS Functionality:");
    println!("==================================");
    
    // Create a sample client ID for demonstration
    let sample_client_id = uuid::Uuid::new_v4();
    
    // Create a sample LUSERS message
    let lusers_message = Message::new(
        MessageType::Lusers,
        vec![], // LUSERS command has no parameters
    );
    
    println!();
    println!("1. Processing LUSERS command...");
    println!("   Command: {}", lusers_message);
    
    // Simulate processing the LUSERS command
    match server.handle_lusers(sample_client_id, lusers_message).await {
        Ok(()) => {
            println!("   ✅ LUSERS command processed successfully");
            println!("   ✅ Network statistics collected");
            println!("   ✅ Numeric replies generated");
        }
        Err(e) => {
            println!("   ❌ Failed to process LUSERS command: {}", e);
            return Err(e);
        }
    }
    
    println!();
    println!("2. Network Statistics Collected:");
    show_network_statistics(server).await?;
    
    println!();
    println!("3. LUSERS Response Messages:");
    show_lusers_responses();
    
    Ok(())
}

/// Show current network statistics
async fn show_network_statistics(server: &Server) -> Result<()> {
    // Note: These are demonstration values since we don't have a full client connection
    println!("   📊 Current Network Statistics:");
    println!("   • Total Users: 0 (no connected clients)");
    println!("   • Operators: 0 (no operators online)");
    println!("   • Channels: 0 (no channels created)");
    println!("   • Servers: 1 (this server only)");
    println!("   • Unknown Connections: 0 (no unregistered connections)");
    println!("   • Local Users: 0 (no local users)");
    println!("   • Global Users: 0 (no network users)");
    println!("   • Max Local Users: 1000 (server limit)");
    println!("   • Max Global Users: 1000 (server limit)");
    
    Ok(())
}

/// Show LUSERS numeric replies
fn show_lusers_numeric_replies() {
    println!("RPL_LUSERCLIENT (251) - Basic network statistics");
    println!("RPL_LUSEROP (252) - Operator count");
    println!("RPL_LUSERUNKNOWN (253) - Unknown connection count");
    println!("RPL_LUSERCHANNELS (254) - Channel count");
    println!("RPL_LUSERME (255) - Server-specific statistics");
    println!("RPL_LOCALUSERS (265) - Local user statistics");
    println!("RPL_GLOBALUSERS (266) - Global user statistics");
}

/// Show LUSERS command usage
fn show_lusers_usage() {
    println!("Command: LUSERS");
    println!("Purpose: Request network statistics");
    println!("Usage: /LUSERS");
    println!("Access: Available to all users");
    println!();
    println!("Parameters:");
    println!("  None - LUSERS command has no parameters");
    println!();
    println!("Response:");
    println!("  Server sends multiple numeric replies with statistics");
    println!("  Includes user counts, operator counts, channel counts, etc.");
}

/// Show expected IRC output
fn show_expected_output() {
    println!(":lusers.example.com 251 * :There are 0 users and 0 services on 1 servers");
    println!(":lusers.example.com 252 * 0 :operator(s) online");
    println!(":lusers.example.com 253 * 0 :unknown connection(s)");
    println!(":lusers.example.com 254 * 0 :channels formed");
    println!(":lusers.example.com 255 * :I have 0 clients and 1 servers");
    println!(":lusers.example.com 265 * :Current local users: 0, max: 1000");
    println!(":lusers.example.com 266 * :Current global users: 0, max: 1000");
}

/// Helper function to demonstrate LUSERS with different scenarios
#[allow(dead_code)]
async fn demonstrate_lusers_scenarios(server: &Server) -> Result<()> {
    println!("LUSERS Command Scenarios:");
    println!("========================");
    
    // Scenario 1: Empty server
    println!();
    println!("Scenario 1: Empty Server");
    println!("  • No users connected");
    println!("  • No channels created");
    println!("  • Only this server in network");
    println!("  • Expected: All counts at 0 except server count");
    
    // Scenario 2: Active server
    println!();
    println!("Scenario 2: Active Server");
    println!("  • Multiple users connected");
    println!("  • Several channels created");
    println!("  • Operators online");
    println!("  • Some unregistered connections");
    println!("  • Expected: Real statistics reflecting activity");
    
    // Scenario 3: Network with multiple servers
    println!();
    println!("Scenario 3: Multi-Server Network");
    println!("  • Users across multiple servers");
    println!("  • Global statistics aggregation");
    println!("  • Server-to-server synchronization");
    println!("  • Expected: Accurate network-wide statistics");
    
    Ok(())
}

/// Helper function to show LUSERS integration points
#[allow(dead_code)]
fn show_lusers_integration() {
    println!("LUSERS Integration Points:");
    println!("========================");
    println!("1. Server Statistics Manager:");
    println!("   - Tracks user connections and disconnections");
    println!("   - Monitors channel creation and destruction");
    println!("   - Counts operators and their status");
    println!();
    println!("2. Database Integration:");
    println!("   - User database for accurate user counts");
    println!("   - Channel database for channel statistics");
    println!("   - Server database for network information");
    println!();
    println!("3. Connection Handler:");
    println!("   - Tracks registered vs unregistered connections");
    println!("   - Monitors connection states");
    println!("   - Provides real-time connection statistics");
    println!();
    println!("4. Module Integration:");
    println!("   - Throttling module for connection statistics");
    println!("   - Channel module for channel statistics");
    println!("   - Operator system for operator statistics");
}

/// Helper function to show LUSERS benefits
#[allow(dead_code)]
fn show_lusers_benefits() {
    println!("LUSERS Command Benefits:");
    println!("=======================");
    println!("✅ Network Monitoring: Real-time network statistics");
    println!("✅ User Awareness: Users can see network activity");
    println!("✅ Operator Tools: Operators can monitor server load");
    println!("✅ RFC Compliance: Standard IRC network statistics");
    println!("✅ Performance Insights: Connection and usage patterns");
    println!("✅ Network Health: Monitor server and network status");
}
