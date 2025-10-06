//! Services Integration Example
//! 
//! This example demonstrates how to use the new services configuration
//! and Atheme integration with the modular architecture.

use rustircd_core::{Config, CoreExtensionManager, Server};
use rustircd_modules::{OperModule, OperConfig, SaslModule, SaslConfig};
use rustircd_services::{AthemeServicesModule, AthemeConfig, AthemeConfigBuilder};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Rust IRC Daemon - Services Integration Example");
    println!("==============================================");
    
    // Load configuration with services
    let config = Config::from_file("examples/configs/services.toml")?;
    
    // Validate configuration
    config.validate()?;
    
    println!("Configuration loaded successfully!");
    println!("Server: {}", config.server.name);
    println!("Network: {}", config.network.name);
    
    // Check services configuration
    println!("\nServices Configuration:");
    println!("=======================");
    
    for service in &config.services.services {
        println!("Service: {} (Type: {})", service.name, service.service_type);
        println!("  Hostname: {}", service.hostname);
        println!("  Port: {}", service.port);
        println!("  TLS: {}", service.tls);
        println!("  Enabled: {}", service.enabled);
    }
    
    // Initialize core extensions
    println!("\nInitializing Core Extensions:");
    println!("=============================");
    
    let core_extensions = CoreExtensionManager::new("services.example.ircd.org".to_string());
    core_extensions.initialize().await?;
    
    println!("✓ Core extensions initialized");
    println!("  - Identify Message Extension");
    println!("  - Account Tracking Extension");
    println!("  - Server Time Extension");
    println!("  - Batch Extension");
    
    // Initialize oper module
    println!("\nInitializing Oper Module:");
    println!("=========================");
    
    let oper_config = OperConfig {
        enabled: true,
        require_oper_for_connect: true,
        show_server_details_in_stats: true,
        log_operator_actions: true,
    };
    
    let oper_module = OperModule::new(oper_config);
    println!("✓ Oper module initialized");
    
    // Initialize SASL module
    println!("\nInitializing SASL Module:");
    println!("========================");
    
    let sasl_config = SaslConfig {
        enabled: true,
        mechanisms: vec!["PLAIN".to_string(), "EXTERNAL".to_string()],
        service_name: "services.example.ircd.org".to_string(),
        require_sasl: false,
        timeout_seconds: 300,
    };
    
    let sasl_module = SaslModule::new(sasl_config);
    println!("✓ SASL module initialized");
    println!("  Supported mechanisms: {:?}", sasl_module.get_supported_mechanisms());
    
    // Initialize Atheme integration
    println!("\nInitializing Atheme Integration:");
    println!("===============================");
    
    let atheme_config = AthemeConfigBuilder::new()
        .service_name("services.example.ircd.org".to_string())
        .hostname("localhost".to_string())
        .port(6666)
        .password("atheme_password".to_string())
        .tls(false)
        .timeout(30)
        .build();
    
    let atheme_module = AthemeServicesModule::new(atheme_config);
    atheme_module.initialize(&config).await?;
    
    println!("✓ Atheme integration initialized");
    
    // Check connection status
    let connection_status = atheme_module.get_connection_status().await;
    println!("  Connection status: {:?}", connection_status);
    
    // Get statistics
    let stats = atheme_module.get_stats().await;
    println!("  Statistics:");
    println!("    Total connections: {}", stats.total_connections);
    println!("    Messages sent: {}", stats.messages_sent);
    println!("    Messages received: {}", stats.messages_received);
    
    // Demonstrate operator functionality
    println!("\nOperator Functionality Demo:");
    println!("===========================");
    
    // Create a mock user for demonstration
    let mut user = rustircd_core::User::new(
        "testuser".to_string(),
        "testuser".to_string(),
        "localhost".to_string(),
        "Test User".to_string(),
        "example.ircd.org".to_string(),
    );
    
    // Set operator flags
    use rustircd_core::config::OperatorFlag;
    let mut operator_flags = std::collections::HashSet::new();
    operator_flags.insert(OperatorFlag::GlobalOper);
    operator_flags.insert(OperatorFlag::Administrator);
    user.set_operator_flags(operator_flags);
    
    println!("User: {}", user.nick);
    println!("Is operator: {}", user.is_operator());
    println!("Is global oper: {}", user.is_global_oper());
    println!("Is administrator: {}", user.is_administrator());
    println!("Can remote connect: {}", user.can_remote_connect());
    println!("Can squit: {}", user.can_squit());
    
    // Demonstrate SASL functionality
    println!("\nSASL Functionality Demo:");
    println!("=======================");
    
    let session = sasl_module.get_session(user.id).await;
    println!("SASL session: {:?}", session);
    
    let is_authenticated = sasl_module.is_authenticated(user.id).await;
    println!("Is SASL authenticated: {}", is_authenticated);
    
    // Demonstrate services integration
    println!("\nServices Integration Demo:");
    println!("=========================");
    
    // Simulate user registration
    atheme_module.handle_user_registration(&user).await?;
    println!("✓ User registration sent to Atheme");
    
    // Simulate channel creation
    atheme_module.handle_channel_creation("#test", &user).await?;
    println!("✓ Channel creation sent to Atheme");
    
    // Show final statistics
    let final_stats = atheme_module.get_stats().await;
    println!("\nFinal Statistics:");
    println!("=================");
    println!("Total connections: {}", final_stats.total_connections);
    println!("Successful auths: {}", final_stats.successful_auths);
    println!("Failed auths: {}", final_stats.failed_auths);
    println!("Messages sent: {}", final_stats.messages_sent);
    println!("Messages received: {}", final_stats.messages_received);
    
    if let Some(last_connection) = final_stats.last_connection {
        println!("Last connection: {}", last_connection.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    
    println!("\nExample completed successfully!");
    println!("This demonstrates the modular architecture with services integration.");
    
    Ok(())
}
