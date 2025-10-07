//! Modular Messaging System Example
//!
//! This example demonstrates the complete modular messaging system with both
//! WALLOPS and GLOBOPS commands, where both modes (+w and +g) are registered
//! by their respective modules rather than being hardcoded in core.

use rustircd_core::{Server, ServerConfig, CustomUserMode, register_custom_mode, unregister_custom_mode};
use rustircd_modules::messaging::{MessagingWrapper, create_default_messaging_module};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Modular Messaging System Example");
    println!("=================================");

    // Demonstrate the modular approach
    println!("\n1. Modular Mode Registration:");
    println!("   - WALLOPS module registers +w mode");
    println!("   - GLOBOPS module registers +g mode");
    println!("   - Core doesn't know about these modes until modules load");

    // Create server configuration
    let config = ServerConfig::default()
        .with_port(6667)
        .with_server_name("modular.example.com");

    // Create server
    let server = Server::new(config).await?;
    
    // Create and register messaging module
    // This will automatically register both +w and +g modes
    println!("\n2. Loading messaging modules...");
    let messaging_module = create_default_messaging_module();
    println!("   ✓ WALLOPS module loaded - registers +w mode");
    println!("   ✓ GLOBOPS module loaded - registers +g mode");
    server.register_module(Box::new(messaging_module)).await?;

    // Start server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    println!("\nServer started on port 6667");
    println!("\nAvailable user modes:");
    println!("===================");
    println!("Core IRC modes (always available):");
    println!("  +a - Away mode");
    println!("  +i - Invisible mode");
    println!("  +r - Restricted mode");
    println!("  +o - Operator mode");
    println!("  +O - Local operator mode");
    println!("  +s - Server notices mode");
    
    println!("\nModule-defined modes (loaded dynamically):");
    println!("  +w - Wallops mode (receives WALLOPS messages) - from wallops module");
    println!("  +g - Globops mode (receives GLOBOPS messages) - from globops module");
    
    println!("\nCommand requirements:");
    println!("===================");
    println!("WALLOPS <message>:");
    println!("  - Sender: Must be operator with +o mode");
    println!("  - Receiver: Users with +w mode");
    println!("  - Mode +w: Self-only, no operator required");
    
    println!("\nGLOBOPS <message>:");
    println!("  - Sender: Must be operator with +o mode");
    println!("  - Receiver: Users with +g mode");
    println!("  - Mode +g: Self-only, no operator required");
    
    println!("\nTesting scenarios:");
    println!("==================");
    println!("1. Connect as regular user:");
    println!("   - Set wallops mode: MODE yournick +w");
    println!("   - Set globops mode: MODE yournick +g");
    println!("   - Set both modes: MODE yournick +wg");
    println!("   - Try to send WALLOPS/GLOBOPS (should fail - not operator)");
    
    println!("\n2. Connect as operator:");
    println!("   - Send wallops: WALLOPS :This is a wallops message");
    println!("   - Send globops: GLOBOPS :This is a globops message");
    println!("   - Only users with respective modes will receive messages");
    
    println!("\n3. Module independence:");
    println!("   - WALLOPS and GLOBOPS are completely independent");
    println!("   - Each module manages its own mode");
    println!("   - Core validates both core and custom modes");
    println!("   - Modules can be loaded/unloaded independently");
    
    println!("\n4. Configuration benefits:");
    println!("   - Server can run without messaging modules");
    println!("   - Can load only WALLOPS or only GLOBOPS");
    println!("   - Easy to add new messaging commands");
    println!("   - Custom modes are properly validated");
    
    println!("\nPress Ctrl+C to stop the server...");
    
    // Wait for server to finish (or be interrupted)
    if let Err(e) = server_handle.await {
        eprintln!("Server task error: {}", e);
    }

    Ok(())
}

/// Demonstrate the modular architecture benefits
#[allow(dead_code)]
fn demonstrate_modular_benefits() {
    println!("\nModular Architecture Benefits");
    println!("=============================");
    
    println!("\n1. Core Independence:");
    println!("   - Core doesn't need to know about messaging-specific modes");
    println!("   - Core MODE command works with any registered mode");
    println!("   - Easy to add new messaging commands without core changes");
    
    println!("\n2. Module Autonomy:");
    println!("   - Each module defines its own mode requirements");
    println!("   - Modules handle their own validation logic");
    println!("   - Modules can register/unregister modes dynamically");
    
    println!("\n3. Configuration Flexibility:");
    println!("   - Modules can be enabled/disabled via configuration");
    println!("   - Different server setups can load different modules");
    println!("   - Easy to create specialized server configurations");
    
    println!("\n4. Extensibility:");
    println!("   - New messaging commands can be added easily");
    println!("   - Custom modes can have any validation rules");
    println!("   - Modules can define complex mode relationships");
    
    println!("\n5. Clean Separation:");
    println!("   - Core handles IRC protocol basics");
    println!("   - Modules handle application-specific features");
    println!("   - Clear boundaries between core and module functionality");
}

/// Show how different server configurations could work
#[allow(dead_code)]
fn demonstrate_configuration_examples() {
    println!("\nConfiguration Examples");
    println!("=====================");
    
    println!("\n1. Minimal Server (no messaging modules):");
    println!("   - Only core IRC modes available");
    println!("   - No WALLOPS or GLOBOPS commands");
    println!("   - Lightweight server setup");
    
    println!("\n2. Standard Server (wallops only):");
    println!("   - Load wallops module");
    println!("   - +w mode available");
    println!("   - WALLOPS command available");
    
    println!("\n3. Enhanced Server (wallops + globops):");
    println!("   - Load both messaging modules");
    println!("   - +w and +g modes available");
    println!("   - Both WALLOPS and GLOBOPS commands");
    
    println!("\n4. Custom Server (custom messaging):");
    println!("   - Load custom messaging modules");
    println!("   - Custom modes (e.g., +x, +y, +z)");
    println!("   - Custom messaging commands");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustircd_core::{register_custom_mode, unregister_custom_mode, CustomUserMode};

    #[test]
    fn test_wallops_mode_registration() {
        let wallops_mode = CustomUserMode {
            character: 'w',
            description: "Receive wallop messages".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "wallops".to_string(),
        };
        
        // Register wallops mode
        assert!(register_custom_mode(wallops_mode).is_ok());
        
        // Verify mode properties
        let mode = rustircd_core::get_custom_mode('w').unwrap();
        assert_eq!(mode.character, 'w');
        assert_eq!(mode.description, "Receive wallop messages");
        assert!(!mode.requires_operator);
        assert!(mode.self_only);
        assert!(!mode.oper_only);
        assert_eq!(mode.module_name, "wallops");
        
        // Cleanup
        assert!(unregister_custom_mode('w', "wallops").is_ok());
    }

    #[test]
    fn test_globops_mode_registration() {
        let globops_mode = CustomUserMode {
            character: 'g',
            description: "Receive global operator notices".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "globops".to_string(),
        };
        
        // Register globops mode
        assert!(register_custom_mode(globops_mode).is_ok());
        
        // Verify mode properties
        let mode = rustircd_core::get_custom_mode('g').unwrap();
        assert_eq!(mode.character, 'g');
        assert_eq!(mode.description, "Receive global operator notices");
        assert!(!mode.requires_operator);
        assert!(mode.self_only);
        assert!(!mode.oper_only);
        assert_eq!(mode.module_name, "globops");
        
        // Cleanup
        assert!(unregister_custom_mode('g', "globops").is_ok());
    }

    #[test]
    fn test_mode_independence() {
        // Register both modes
        let wallops_mode = CustomUserMode {
            character: 'w',
            description: "Wallops mode".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "wallops".to_string(),
        };
        
        let globops_mode = CustomUserMode {
            character: 'g',
            description: "Globops mode".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "globops".to_string(),
        };
        
        assert!(register_custom_mode(wallops_mode).is_ok());
        assert!(register_custom_mode(globops_mode).is_ok());
        
        // Both modes should be available
        assert!(rustircd_core::is_valid_user_mode('w'));
        assert!(rustircd_core::is_valid_user_mode('g'));
        
        // Unregister one mode
        assert!(unregister_custom_mode('w', "wallops").is_ok());
        
        // Only globops mode should be available
        assert!(!rustircd_core::is_valid_user_mode('w'));
        assert!(rustircd_core::is_valid_user_mode('g'));
        
        // Cleanup
        assert!(unregister_custom_mode('g', "globops").is_ok());
    }

    #[test]
    fn test_operator_requirements() {
        // Test that both commands require operator privileges
        use rustircd_modules::messaging::{WallopsModule, GlobopsModule};
        
        let wallops = WallopsModule::new();
        let globops = GlobopsModule::new();
        
        // Both should indicate they require operator privileges
        assert!(wallops.help_text().contains("operator"));
        assert!(globops.help_text().contains("operator"));
        
        // Both should handle their own operator validation
        assert_eq!(wallops.sender_mode_required(), None); // Handled manually
        assert_eq!(globops.sender_mode_required(), None); // Handled manually
    }
}
