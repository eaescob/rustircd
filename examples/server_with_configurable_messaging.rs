//! Server with Configurable Messaging Example
//!
//! This example demonstrates how to run a complete IRC server with configurable
//! messaging modules loaded from configuration files.

use rustircd_core::{Server, Config};
use rustircd_modules::messaging::create_messaging_module_with_config;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("IRC Server with Configurable Messaging");
    println!("=====================================");

    // Load configuration
    println!("\n1. Loading configuration...");
    let config_path = "config.toml";
    let config = Config::from_file(config_path)?;
    println!("   ✓ Configuration loaded from {}", config_path);

    // Validate configuration
    config.validate()?;
    println!("   ✓ Configuration validated");

    // Create server with configuration
    println!("\n2. Creating server...");
    let mut server = Server::new(config.clone()).await;
    println!("   ✓ Server created: {}", config.server.name);

    // Load messaging modules based on configuration
    println!("\n3. Loading messaging modules...");
    if config.modules.messaging.enabled {
        // Note: Module loading would happen here in a real implementation
        // let messaging_module = create_messaging_module_with_config(&config.modules.messaging);
        // server.register_module(Box::new(messaging_module)).await?;
        
        println!("   ✓ Messaging modules configured");
        println!("   - WALLOPS: {} (mode: {})", 
                 config.modules.messaging.wallops.enabled,
                 config.modules.messaging.wallops.receiver_mode.map_or("None".to_string(), |c| c.to_string()));
        println!("   - GLOBOPS: {} (mode: {})", 
                 config.modules.messaging.globops.enabled,
                 config.modules.messaging.globops.receiver_mode.map_or("None".to_string(), |c| c.to_string()));
    } else {
        println!("   - Messaging modules disabled in configuration");
    }

    // Display server information
    println!("\n4. Server Information:");
    println!("   - Server name: {}", config.server.name);
    println!("   - Server description: {}", config.server.description);
    println!("   - Max clients: {}", config.server.max_clients);
    println!("   - Ports configured: {}", config.connection.ports.len());
    
    for port_config in &config.connection.ports {
        let connection_type = match port_config.connection_type {
            rustircd_core::config::PortConnectionType::Client => "Client",
            rustircd_core::config::PortConnectionType::Server => "Server",
            rustircd_core::config::PortConnectionType::Both => "Both",
        };
        println!("     * Port {} ({}, TLS: {})", port_config.port, connection_type, port_config.tls);
    }

    // Display available commands and modes
    println!("\n5. Available Commands and Modes:");
    display_available_features(&config);

    // Start server
    println!("\n6. Starting server...");
    println!("   Server is now running on the configured ports");
    println!("   Press Ctrl+C to stop the server");
    
    // Display usage instructions
    display_usage_instructions(&config);

    // Run server
    if let Err(e) = server.start().await {
        eprintln!("Server error: {}", e);
    }

    println!("\nServer stopped.");
    Ok(())
}

/// Display available features based on configuration
fn display_available_features(config: &Config) {
    println!("   Core IRC commands: PRIVMSG, NOTICE, JOIN, PART, MODE, WHOIS, etc.");
    
    if config.modules.messaging.enabled {
        println!("   Messaging commands:");
        
        if config.modules.messaging.wallops.enabled {
            println!("     * WALLOPS <message> - Send message to users with {} mode", 
                     config.modules.messaging.wallops.receiver_mode.map_or("?".to_string(), |c| c.to_string()));
        }
        
        if config.modules.messaging.globops.enabled {
            println!("     * GLOBOPS <message> - Send message to users with {} mode", 
                     config.modules.messaging.globops.receiver_mode.map_or("?".to_string(), |c| c.to_string()));
        }
        
        println!("   Available user modes:");
        println!("     * Core modes: +a (away), +i (invisible), +r (restricted), +o (operator), +O (local operator), +s (server notices)");
        
        if config.modules.messaging.wallops.enabled {
            if let Some(mode) = config.modules.messaging.wallops.receiver_mode {
                println!("     * +{} (wallops) - Receive WALLOPS messages", mode);
            }
        }
        
        if config.modules.messaging.globops.enabled {
            if let Some(mode) = config.modules.messaging.globops.receiver_mode {
                println!("     * +{} (globops) - Receive GLOBOPS messages", mode);
            }
        }
    } else {
        println!("   Messaging commands: None (disabled in configuration)");
        println!("   Available user modes: Core IRC modes only");
    }
}

/// Display usage instructions
fn display_usage_instructions(config: &Config) {
    println!("\n7. Usage Instructions:");
    println!("   Connect with an IRC client to one of the configured ports");
    
    if config.modules.messaging.enabled {
        println!("\n   To test messaging commands:");
        
        if config.modules.messaging.wallops.enabled {
            println!("   - Set wallops mode: MODE yournick +{}", 
                     config.modules.messaging.wallops.receiver_mode.map_or("?".to_string(), |c| c.to_string()));
            println!("   - Send wallops (as operator): WALLOPS :Hello, this is a wallops message!");
        }
        
        if config.modules.messaging.globops.enabled {
            println!("   - Set globops mode: MODE yournick +{}", 
                     config.modules.messaging.globops.receiver_mode.map_or("?".to_string(), |c| c.to_string()));
            println!("   - Send globops (as operator): GLOBOPS :Hello, this is a globops message!");
        }
        
        println!("   - Become operator: OPER admin password (if configured)");
    }
    
    println!("\n   Example IRC client connection:");
    println!("   /connect localhost 6667");
    println!("   /nick testuser");
    println!("   /user username hostname servername :Real Name");
}

/// Demonstrate different configuration scenarios
#[allow(dead_code)]
async fn demonstrate_different_configs() -> Result<(), Box<dyn std::error::Error>> {
    let config_files = vec![
        "examples/configs/messaging_default.toml",
        "examples/configs/messaging_wallops_only.toml",
        "examples/configs/messaging_globops_only.toml",
        "examples/configs/messaging_disabled.toml",
        "examples/configs/messaging_custom_modes.toml",
    ];

    for config_file in config_files {
        if std::path::Path::new(config_file).exists() {
            println!("\n--- Testing configuration: {} ---", config_file);
            
            let config = Config::from_file(config_file)?;
            config.validate()?;
            
            println!("Messaging enabled: {}", config.modules.messaging.enabled);
            println!("WALLOPS: {} (mode: {})", 
                     config.modules.messaging.wallops.enabled,
                     config.modules.messaging.wallops.receiver_mode.map_or("None".to_string(), |c| c.to_string()));
            println!("GLOBOPS: {} (mode: {})", 
                     config.modules.messaging.globops.enabled,
                     config.modules.messaging.globops.receiver_mode.map_or("None".to_string(), |c| c.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loading() {
        // Test that we can create a default config
        let config = Config::default();
        assert!(config.modules.messaging.enabled);
        assert!(config.modules.messaging.wallops.enabled);
        assert!(config.modules.messaging.globops.enabled);
    }

    #[test]
    fn test_feature_display() {
        let config = Config::default();
        // This test just ensures the function doesn't panic
        display_available_features(&config);
        display_usage_instructions(&config);
    }
}
