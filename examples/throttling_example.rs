//! Example demonstrating the throttling module functionality
//! 
//! This example shows how to:
//! 1. Configure the throttling module
//! 2. Start a server with throttling enabled
//! 3. Test connection throttling behavior

use rustircd_core::{Config, Server, Result};
use std::net::{IpAddr, Ipv4Addr};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCD Throttling Module Example");
    println!("==================================");
    
    // Create a configuration with throttling enabled
    let mut config = Config::default();
    
    // Enable the throttling module
    config.modules.enabled_modules.push("throttling".to_string());
    
    // Configure throttling settings
    config.modules.throttling.enabled = true;
    config.modules.throttling.max_connections_per_ip = 3;  // Allow 3 connections per IP
    config.modules.throttling.time_window_seconds = 60;    // Within 60 seconds
    config.modules.throttling.initial_throttle_seconds = 10; // Initial throttle: 10 seconds
    config.modules.throttling.max_stages = 5;              // Up to 5 throttling stages
    config.modules.throttling.stage_factor = 2;            // Each stage doubles the throttle time
    config.modules.throttling.cleanup_interval_seconds = 300; // Clean up every 5 minutes
    
    // Configure server settings
    config.server.name = "throttling.example.com".to_string();
    config.server.description = "Throttling Test Server".to_string();
    
    // Configure a simple port
    config.connection.ports.clear();
    config.connection.ports.push(rustircd_core::config::PortConfig {
        port: 6667,
        connection_type: rustircd_core::config::PortConnectionType::Client,
        tls: false,
        description: Some("Throttling test port".to_string()),
    });
    
    println!("Configuration:");
    println!("  Max connections per IP: {}", config.modules.throttling.max_connections_per_ip);
    println!("  Time window: {} seconds", config.modules.throttling.time_window_seconds);
    println!("  Initial throttle: {} seconds", config.modules.throttling.initial_throttle_seconds);
    println!("  Max stages: {}", config.modules.throttling.max_stages);
    println!("  Stage factor: {}", config.modules.throttling.stage_factor);
    println!();
    
    // Create and initialize the server
    let mut server = Server::new(config);
    server.init().await?;
    
    println!("Server initialized with throttling module enabled");
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
    println!("Throttling behavior:");
    println!("1. First {} connections from the same IP within {} seconds are allowed", 
             config.modules.throttling.max_connections_per_ip, 
             config.modules.throttling.time_window_seconds);
    println!("2. After exceeding the limit, the IP is throttled for {} seconds (stage 1)", 
             config.modules.throttling.initial_throttle_seconds);
    println!("3. Each subsequent violation increases the throttle duration by a factor of {}", 
             config.modules.throttling.stage_factor);
    println!("4. Maximum of {} throttling stages", config.modules.throttling.max_stages);
    println!();
    
    // Demonstrate throttling with a test IP
    let test_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
    println!("Testing throttling with IP: {}", test_ip);
    
    // Create a throttling manager to test the functionality
    let throttling_manager = rustircd_core::ThrottlingManager::new(
        server.config().modules.throttling.clone()
    );
    throttling_manager.init().await?;
    
    // Test connection attempts
    for i in 1..=6 {
        let allowed = throttling_manager.check_connection_allowed(test_ip).await?;
        let (is_throttled, stage, remaining) = throttling_manager.get_throttle_status(test_ip).await;
        
        println!("Connection attempt {}: {} (throttled: {}, stage: {}, remaining: {}s)", 
                 i, 
                 if allowed { "ALLOWED" } else { "BLOCKED" },
                 is_throttled,
                 stage,
                 remaining);
        
        if !allowed && is_throttled {
            println!("  → IP is throttled for {} more seconds", remaining);
        }
        
        // Small delay between attempts
        sleep(Duration::from_millis(100)).await;
    }
    
    println!();
    println!("You can now test the server with an IRC client:");
    println!("  Server: localhost:6667");
    println!("  Try connecting multiple times quickly to see throttling in action");
    println!();
    println!("Press Ctrl+C to stop the server");
    
    // Wait for the server to finish (or be interrupted)
    server_handle.await??;
    
    Ok(())
}

/// Helper function to demonstrate different throttling configurations
#[allow(dead_code)]
fn create_throttling_configs() {
    // Conservative throttling (strict limits)
    let _conservative = rustircd_core::config::ThrottlingConfig {
        enabled: true,
        max_connections_per_ip: 2,
        time_window_seconds: 120,
        initial_throttle_seconds: 30,
        max_stages: 5,
        stage_factor: 5,
        cleanup_interval_seconds: 300,
    };
    
    // Relaxed throttling (more permissive)
    let _relaxed = rustircd_core::config::ThrottlingConfig {
        enabled: true,
        max_connections_per_ip: 10,
        time_window_seconds: 30,
        initial_throttle_seconds: 5,
        max_stages: 15,
        stage_factor: 15,
        cleanup_interval_seconds: 300,
    };
    
    // Aggressive throttling (very strict)
    let _aggressive = rustircd_core::config::ThrottlingConfig {
        enabled: true,
        max_connections_per_ip: 1,
        time_window_seconds: 300,
        initial_throttle_seconds: 60,
        max_stages: 8,
        stage_factor: 8,
        cleanup_interval_seconds: 300,
    };
    
    println!("Different throttling configurations created for reference");
}
