//! Example demonstrating STATS security features
//! 
//! This example shows how to:
//! 1. Configure STATS information disclosure
//! 2. Test operator vs non-operator STATS access
//! 3. Verify security settings work correctly

use rustircd_core::{Config, Server, Result};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCD STATS Security Example");
    println!("==============================");
    
    // Create a configuration with security settings
    let mut config = Config::default();
    
    // Configure server settings
    config.server.name = "secure.example.com".to_string();
    config.server.description = "STATS Security Test Server".to_string();
    
    // SECURITY SETTING: Control server IP/hostname disclosure
    config.server.show_server_details_in_stats = false; // Hide details even from operators
    
    // Enable the throttling module
    config.modules.enabled_modules.push("throttling".to_string());
    
    // Configure throttling settings for testing
    config.modules.throttling.enabled = true;
    config.modules.throttling.max_connections_per_ip = 3;
    config.modules.throttling.time_window_seconds = 60;
    config.modules.throttling.initial_throttle_seconds = 5;
    
    // Configure a simple port
    config.connection.ports.clear();
    config.connection.ports.push(rustircd_core::config::PortConfig {
        port: 6667,
        connection_type: rustircd_core::config::PortConnectionType::Client,
        tls: false,
        description: Some("Security test port".to_string()),
    });
    
    // Add an operator for testing
    config.network.operators.push(rustircd_core::config::OperatorConfig {
        nickname: "admin".to_string(),
        password_hash: "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8".to_string(), // "password"
        hostmask: "admin@*".to_string(),
        flags: vec![
            rustircd_core::config::OperatorFlag::LocalOperator,
            rustircd_core::config::OperatorFlag::GlobalOperator,
        ],
    });
    
    println!("Configuration:");
    println!("  Server: {}", config.server.name);
    println!("  Show server details in STATS: {}", config.server.show_server_details_in_stats);
    println!("  Throttling enabled: {}", config.modules.throttling.enabled);
    println!();
    
    // Create and initialize the server
    let mut server = Server::new(config);
    server.init().await?;
    
    println!("Server initialized with STATS security settings");
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
    println!("STATS Security Features:");
    println!("=======================");
    println!();
    println!("1. Operator Access Control:");
    println!("   - Operators can see full information when show_server_details_in_stats=true");
    println!("   - Non-operators always see limited information");
    println!("   - Hostmasks are hidden from non-operators");
    println!();
    println!("2. Server Detail Control:");
    println!("   - show_server_details_in_stats=false hides server IPs/hostnames even from operators");
    println!("   - This provides additional security for sensitive server information");
    println!();
    println!("3. Throttling Module Security:");
    println!("   - /STATS T shows IP addresses only to operators (if configured)");
    println!("   - Non-operators see only aggregate counts");
    println!();
    println!("Test Scenarios:");
    println!("==============");
    println!();
    println!("Scenario 1: Non-Operator Access");
    println!("  /connect localhost 6667");
    println!("  /nick testuser");
    println!("  /user testuser 0 * :Test User");
    println!("  /stats o    # Should show '***@***' for operators");
    println!("  /stats l    # Should show '***' for server names");
    println!("  /stats c    # Should show limited connection info");
    println!("  /stats T    # Should show only aggregate counts");
    println!();
    println!("Scenario 2: Operator Access (with show_server_details_in_stats=false)");
    println!("  /oper admin password");
    println!("  /stats o    # Should show '***@***' for operators (details hidden)");
    println!("  /stats l    # Should show '***' for server names (details hidden)");
    println!("  /stats c    # Should show limited connection info (details hidden)");
    println!("  /stats T    # Should show only aggregate counts (IPs hidden)");
    println!();
    println!("Scenario 3: Operator Access (with show_server_details_in_stats=true)");
    println!("  # Change config to show_server_details_in_stats=true and restart");
    println!("  /oper admin password");
    println!("  /stats o    # Should show full hostmasks for operators");
    println!("  /stats l    # Should show full server names");
    println!("  /stats c    # Should show detailed connection info");
    println!("  /stats T    # Should show individual IP addresses and status");
    println!();
    println!("Security Benefits:");
    println!("=================");
    println!("- Prevents information leakage about server infrastructure");
    println!("- Protects operator hostmasks from disclosure");
    println!("- Hides IP addresses from potential attackers");
    println!("- Provides granular control over information disclosure");
    println!("- Maintains RFC compliance while adding security layers");
    println!();
    println!("Press Ctrl+C to stop the server");
    
    // Wait for the server to finish (or be interrupted)
    server_handle.await??;
    
    Ok(())
}

/// Helper function to demonstrate different security configurations
#[allow(dead_code)]
fn show_security_examples() {
    println!("STATS Security Configuration Examples:");
    println!("====================================");
    println!();
    println!("1. Maximum Security (recommended for production):");
    println!("   show_server_details_in_stats = false");
    println!("   # Even operators cannot see server IPs/hostnames");
    println!();
    println!("2. Standard Security (default):");
    println!("   show_server_details_in_stats = true");
    println!("   # Operators can see full details, non-operators see limited info");
    println!();
    println!("3. Expected Output Examples:");
    println!();
    println!("   Non-Operator /STATS o:");
    println!("   :server 243 * O ***@*** * admin 0 Operator");
    println!();
    println!("   Operator /STATS o (with show_server_details_in_stats=false):");
    println!("   :server 243 * O ***@*** * admin 0 Operator");
    println!();
    println!("   Operator /STATS o (with show_server_details_in_stats=true):");
    println!("   :server 243 * O admin@192.168.1.100 * admin 0 Operator");
    println!();
    println!("   Non-Operator /STATS T:");
    println!("   :server 244 * THROTTLING 3 throttled IPs, 2 active IPs");
    println!();
    println!("   Operator /STATS T (with show_server_details_in_stats=true):");
    println!("   :server 244 * 192.168.1.100 THROTTLED stage=2 remaining=45s");
    println!("   :server 244 * 192.168.1.101 ACTIVE connections=3");
}
