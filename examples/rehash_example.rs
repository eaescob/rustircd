//! Rehash Command Example
//! 
//! This example demonstrates the rehash command functionality for runtime configuration reloading.
//! The rehash command allows operators to reload various parts of the server configuration
//! without restarting the server.

use rustircd_core::{Config, Server};
use tracing::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("Rehash Command Example");
    info!("=====================");

    // Load configuration
    let config = Config::from_file("config.toml")?;
    
    // Create server instance
    let mut server = Server::new(config).await;
    server.init().await?;

    info!("Server initialized successfully");
    info!("Rehash service is available for runtime configuration reloading");

    // Demonstrate rehash service functionality
    let rehash_service = server.rehash_service();
    
    // Show current configuration info
    let config_info = rehash_service.get_config_info().await;
    info!("Current configuration: {}", config_info);

    // Demonstrate different rehash operations
    info!("\nRehash Command Examples:");
    info!("========================");
    
    info!("1. REHASH - Reload main configuration file");
    info!("   This reloads the main config.toml file including:");
    info!("   - Server settings (name, description, version)");
    info!("   - Network settings (operators, super servers)");
    info!("   - Connection settings (ports, bind address)");
    info!("   - Security settings (TLS, authentication)");
    info!("   - Module settings (enabled modules, configuration)");
    
    info!("\n2. REHASH SSL - Reload TLS/SSL settings");
    info!("   This reloads TLS configuration including:");
    info!("   - Certificate files");
    info!("   - Private key files");
    info!("   - Cipher suites");
    info!("   - TLS version settings");
    
    info!("\n3. REHASH MOTD - Reload Message of the Day");
    info!("   This reloads the MOTD file so users see updated messages");
    info!("   when they use the /MOTD command");
    
    info!("\n4. REHASH MODULES - Reload all modules");
    info!("   This reloads module configuration and settings");
    info!("   without restarting the server");

    info!("\nUsage in IRC client:");
    info!("===================");
    info!("As an operator, you can use these commands:");
    info!("  /REHASH           - Reload main configuration");
    info!("  /REHASH SSL       - Reload TLS settings");
    info!("  /REHASH MOTD      - Reload MOTD file");
    info!("  /REHASH MODULES   - Reload all modules");
    info!("  /LOCops REHASH    - Same as above, through LOCops");

    info!("\nTesting rehash operations:");
    info!("=========================");

    // Test MOTD reload
    match rehash_service.reload_motd().await {
        Ok(_) => info!("✓ MOTD reload test successful"),
        Err(e) => error!("✗ MOTD reload test failed: {}", e),
    }

    // Test SSL reload
    match rehash_service.reload_ssl().await {
        Ok(_) => info!("✓ SSL reload test successful"),
        Err(e) => error!("✗ SSL reload test failed: {}", e),
    }

    // Test modules reload
    match rehash_service.reload_modules().await {
        Ok(_) => info!("✓ Modules reload test successful"),
        Err(e) => error!("✗ Modules reload test failed: {}", e),
    }

    // Test main config reload
    match rehash_service.reload_main_config().await {
        Ok(_) => info!("✓ Main config reload test successful"),
        Err(e) => error!("✗ Main config reload test failed: {}", e),
    }

    info!("\nRehash functionality is now available!");
    info!("Operators can use the REHASH command to reload configuration at runtime.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rehash_service_creation() {
        let config = Config::default();
        let config_arc = Arc::new(RwLock::new(config));
        let motd_manager = Arc::new(rustircd_core::MotdManager::new());
        let rehash_service = RehashService::new(
            config_arc,
            motd_manager,
            "test_config.toml".to_string(),
        );
        
        let info = rehash_service.get_config_info().await;
        assert!(info.contains("rustircd"));
    }

    #[tokio::test]
    async fn test_rehash_sections() {
        let config = Config::default();
        let config_arc = Arc::new(RwLock::new(config));
        let motd_manager = Arc::new(rustircd_core::MotdManager::new());
        let rehash_service = RehashService::new(
            config_arc,
            motd_manager,
            "test_config.toml".to_string(),
        );
        
        // Test all rehash sections
        assert!(rehash_service.reload_section("SSL").await.is_ok());
        assert!(rehash_service.reload_section("MOTD").await.is_ok());
        assert!(rehash_service.reload_section("MODULES").await.is_ok());
        assert!(rehash_service.reload_section("INVALID").await.is_err());
    }
}
