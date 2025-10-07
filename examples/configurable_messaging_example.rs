//! Configurable Messaging Modules Example
//!
//! This example demonstrates how to load messaging modules based on configuration files,
//! allowing server administrators to choose which messaging features to enable.

use rustircd_core::{Server, ServerConfig, Config, MessagingConfig};
use rustircd_modules::messaging::{create_messaging_module_with_config, create_default_messaging_module};
use std::path::Path;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Configurable Messaging Modules Example");
    println!("======================================");

    // Example 1: Load from configuration file
    println!("\n1. Loading configuration from file...");
    let config_path = "examples/configs/messaging_default.toml";
    
    if Path::new(config_path).exists() {
        let config = Config::from_file(config_path)?;
        demonstrate_config_loading(&config).await?;
    } else {
        println!("   Configuration file not found, using default configuration");
        demonstrate_default_loading().await?;
    }

    // Example 2: Different configuration scenarios
    println!("\n2. Demonstrating different configuration scenarios...");
    demonstrate_configuration_scenarios().await?;

    Ok(())
}

/// Demonstrate loading messaging modules from configuration
async fn demonstrate_config_loading(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("   ✓ Configuration loaded successfully");
    
    // Create server configuration
    let server_config = ServerConfig::default()
        .with_port(6667)
        .with_server_name("configurable.example.com");

    // Create server
    let server = Server::new(server_config).await?;
    
    // Create messaging module with configuration
    let messaging_module = create_messaging_module_with_config(&config.modules.messaging);
    server.register_module(Box::new(messaging_module)).await?;

    println!("   ✓ Messaging modules loaded based on configuration");
    println!("   - Messaging enabled: {}", config.modules.messaging.enabled);
    println!("   - WALLOPS enabled: {}", config.modules.messaging.wallops.enabled);
    println!("   - GLOBOPS enabled: {}", config.modules.messaging.globops.enabled);
    
    if config.modules.messaging.wallops.enabled {
        println!("   - WALLOPS mode: {}", 
                 config.modules.messaging.wallops.receiver_mode.map_or("None".to_string(), |c| c.to_string()));
    }
    
    if config.modules.messaging.globops.enabled {
        println!("   - GLOBOPS mode: {}", 
                 config.modules.messaging.globops.receiver_mode.map_or("None".to_string(), |c| c.to_string()));
    }

    // Start server briefly to demonstrate
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    sleep(Duration::from_millis(100)).await;
    server_handle.abort();

    Ok(())
}

/// Demonstrate default configuration loading
async fn demonstrate_default_loading() -> Result<(), Box<dyn std::error::Error>> {
    println!("   Using default configuration (both WALLOPS and GLOBOPS enabled)");
    
    // Create server configuration
    let server_config = ServerConfig::default()
        .with_port(6667)
        .with_server_name("default.example.com");

    // Create server
    let server = Server::new(server_config).await?;
    
    // Create default messaging module
    let messaging_module = create_default_messaging_module();
    server.register_module(Box::new(messaging_module)).await?;

    println!("   ✓ Default messaging modules loaded");
    
    // Start server briefly to demonstrate
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    sleep(Duration::from_millis(100)).await;
    server_handle.abort();

    Ok(())
}

/// Demonstrate different configuration scenarios
async fn demonstrate_configuration_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = vec![
        ("Default (Both enabled)", create_default_config()),
        ("WALLOPS only", create_wallops_only_config()),
        ("GLOBOPS only", create_globops_only_config()),
        ("Both disabled", create_disabled_config()),
        ("Custom modes", create_custom_modes_config()),
    ];

    for (name, config) in scenarios {
        println!("\n   Scenario: {}", name);
        println!("   - Messaging enabled: {}", config.enabled);
        println!("   - WALLOPS enabled: {} (mode: {})", 
                 config.wallops.enabled,
                 config.wallops.receiver_mode.map_or("None".to_string(), |c| c.to_string()));
        println!("   - GLOBOPS enabled: {} (mode: {})", 
                 config.globops.enabled,
                 config.globops.receiver_mode.map_or("None".to_string(), |c| c.to_string()));
        
        // Demonstrate what would be available
        let available_commands = get_available_commands(&config);
        println!("   - Available commands: {}", available_commands.join(", "));
        
        let available_modes = get_available_modes(&config);
        println!("   - Available modes: {}", available_modes.join(", "));
    }

    Ok(())
}

/// Create default configuration
fn create_default_config() -> MessagingConfig {
    MessagingConfig::default()
}

/// Create wallops-only configuration
fn create_wallops_only_config() -> MessagingConfig {
    MessagingConfig {
        enabled: true,
        wallops: rustircd_core::MessagingModuleConfig {
            enabled: true,
            require_operator: true,
            receiver_mode: Some('w'),
            self_only_mode: true,
            mode_requires_operator: false,  // Users can set +w themselves
        },
        globops: rustircd_core::MessagingModuleConfig {
            enabled: false,
            require_operator: true,
            receiver_mode: None,
            self_only_mode: false,
            mode_requires_operator: true,
        },
    }
}

/// Create globops-only configuration
fn create_globops_only_config() -> MessagingConfig {
    MessagingConfig {
        enabled: true,
        wallops: rustircd_core::MessagingModuleConfig {
            enabled: false,
            require_operator: true,
            receiver_mode: None,
            self_only_mode: true,
            mode_requires_operator: false,
        },
        globops: rustircd_core::MessagingModuleConfig {
            enabled: true,
            require_operator: true,
            receiver_mode: Some('g'),
            self_only_mode: false,  // Operators can set +g on others
            mode_requires_operator: true,  // Only operators can set +g
        },
    }
}

/// Create disabled configuration
fn create_disabled_config() -> MessagingConfig {
    MessagingConfig {
        enabled: false,
        wallops: rustircd_core::MessagingModuleConfig {
            enabled: false,
            require_operator: true,
            receiver_mode: None,
            self_only_mode: true,
            mode_requires_operator: false,
        },
        globops: rustircd_core::MessagingModuleConfig {
            enabled: false,
            require_operator: true,
            receiver_mode: None,
            self_only_mode: true,
            mode_requires_operator: false,
        },
    }
}

/// Create custom modes configuration
fn create_custom_modes_config() -> MessagingConfig {
    MessagingConfig {
        enabled: true,
        wallops: rustircd_core::MessagingModuleConfig {
            enabled: true,
            require_operator: true,
            receiver_mode: Some('x'),
            self_only_mode: true,
            mode_requires_operator: false,
        },
        globops: rustircd_core::MessagingModuleConfig {
            enabled: true,
            require_operator: true,
            receiver_mode: Some('y'),
            self_only_mode: true,
            mode_requires_operator: false,
        },
    }
}

/// Get available commands based on configuration
fn get_available_commands(config: &MessagingConfig) -> Vec<String> {
    let mut commands = Vec::new();
    
    if config.enabled {
        if config.wallops.enabled {
            commands.push("WALLOPS");
        }
        if config.globops.enabled {
            commands.push("GLOBOPS");
        }
    }
    
    if commands.is_empty() {
        commands.push("None (messaging disabled)");
    }
    
    commands
}

/// Get available modes based on configuration
fn get_available_modes(config: &MessagingConfig) -> Vec<String> {
    let mut modes = Vec::new();
    
    if config.enabled {
        if config.wallops.enabled {
            if let Some(mode) = config.wallops.receiver_mode {
                modes.push(format!("+{} (wallops)", mode));
            }
        }
        if config.globops.enabled {
            if let Some(mode) = config.globops.receiver_mode {
                modes.push(format!("+{} (globops)", mode));
            }
        }
    }
    
    if modes.is_empty() {
        modes.push("None (messaging disabled)".to_string());
    }
    
    modes
}

/// Demonstrate configuration file examples
#[allow(dead_code)]
fn demonstrate_configuration_files() {
    println!("\nConfiguration File Examples");
    println!("===========================");
    
    println!("\n1. Default configuration (messaging_default.toml):");
    println!("   [modules.messaging]");
    println!("   enabled = true");
    println!("   [modules.messaging.wallops]");
    println!("   enabled = true");
    println!("   receiver_mode = \"w\"");
    println!("   [modules.messaging.globops]");
    println!("   enabled = true");
    println!("   receiver_mode = \"g\"");
    
    println!("\n2. WALLOPS only (messaging_wallops_only.toml):");
    println!("   [modules.messaging]");
    println!("   enabled = true");
    println!("   [modules.messaging.wallops]");
    println!("   enabled = true");
    println!("   receiver_mode = \"w\"");
    println!("   [modules.messaging.globops]");
    println!("   enabled = false");
    
    println!("\n3. Custom modes (messaging_custom_modes.toml):");
    println!("   [modules.messaging.wallops]");
    println!("   enabled = true");
    println!("   receiver_mode = \"x\"");
    println!("   [modules.messaging.globops]");
    println!("   enabled = true");
    println!("   receiver_mode = \"y\"");
    
    println!("\n4. Disabled (messaging_disabled.toml):");
    println!("   [modules.messaging]");
    println!("   enabled = false");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustircd_core::MessagingConfig;

    #[test]
    fn test_configuration_scenarios() {
        // Test default configuration
        let default_config = create_default_config();
        assert!(default_config.enabled);
        assert!(default_config.wallops.enabled);
        assert!(default_config.globops.enabled);
        assert_eq!(default_config.wallops.receiver_mode, Some('w'));
        assert_eq!(default_config.globops.receiver_mode, Some('g'));
        
        // Test wallops-only configuration
        let wallops_only = create_wallops_only_config();
        assert!(wallops_only.enabled);
        assert!(wallops_only.wallops.enabled);
        assert!(!wallops_only.globops.enabled);
        
        // Test disabled configuration
        let disabled = create_disabled_config();
        assert!(!disabled.enabled);
        assert!(!disabled.wallops.enabled);
        assert!(!disabled.globops.enabled);
        
        // Test custom modes configuration
        let custom = create_custom_modes_config();
        assert!(custom.enabled);
        assert_eq!(custom.wallops.receiver_mode, Some('x'));
        assert_eq!(custom.globops.receiver_mode, Some('y'));
    }

    #[test]
    fn test_available_commands() {
        let default_config = create_default_config();
        let commands = get_available_commands(&default_config);
        assert!(commands.contains(&"WALLOPS".to_string()));
        assert!(commands.contains(&"GLOBOPS".to_string()));
        
        let wallops_only = create_wallops_only_config();
        let commands = get_available_commands(&wallops_only);
        assert!(commands.contains(&"WALLOPS".to_string()));
        assert!(!commands.contains(&"GLOBOPS".to_string()));
        
        let disabled = create_disabled_config();
        let commands = get_available_commands(&disabled);
        assert!(commands.contains(&"None (messaging disabled)".to_string()));
    }

    #[test]
    fn test_available_modes() {
        let default_config = create_default_config();
        let modes = get_available_modes(&default_config);
        assert!(modes.iter().any(|m| m.contains("+w (wallops)")));
        assert!(modes.iter().any(|m| m.contains("+g (globops)")));
        
        let custom = create_custom_modes_config();
        let modes = get_available_modes(&custom);
        assert!(modes.iter().any(|m| m.contains("+x (wallops)")));
        assert!(modes.iter().any(|m| m.contains("+y (globops)")));
    }
}
