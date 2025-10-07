//! Extensible User Modes Example
//!
//! This example demonstrates how messaging modules can register custom user modes
//! with the core system, allowing for flexible and modular mode management.

use rustircd_core::{Server, ServerConfig, CustomUserMode, register_custom_mode, unregister_custom_mode};
use rustircd_modules::messaging::{MessagingWrapper, create_default_messaging_module};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Extensible User Modes Example");
    println!("=============================");

    // Demonstrate manual mode registration (for testing)
    println!("\n1. Registering custom modes manually...");
    
    let test_mode = CustomUserMode {
        character: 'x',
        description: "Test mode for demonstration".to_string(),
        requires_operator: false,
        self_only: true,
        oper_only: false,
        module_name: "example".to_string(),
    };
    
    match register_custom_mode(test_mode) {
        Ok(()) => println!("   ✓ Successfully registered custom mode 'x'"),
        Err(e) => println!("   ✗ Failed to register mode 'x': {}", e),
    }

    // Create server configuration
    let config = ServerConfig::default()
        .with_port(6667)
        .with_server_name("modes.example.com");

    // Create server
    let server = Server::new(config).await?;
    
    // Create and register messaging module with GLOBOPS support
    // This will automatically register the 'g' mode
    println!("\n2. Creating messaging module with GLOBOPS...");
    let messaging_module = create_default_messaging_module();
    println!("   ✓ Messaging module created with modes: +g (GLOBOPS)");
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
    println!("Core IRC modes:");
    println!("  +a - Away mode");
    println!("  +i - Invisible mode");
    println!("  +w - Wallops mode (receives WALLOPS messages)");
    println!("  +r - Restricted mode");
    println!("  +o - Operator mode");
    println!("  +O - Local operator mode");
    println!("  +s - Server notices mode");
    
    println!("\nCustom modes:");
    println!("  +g - Globops mode (receives GLOBOPS messages) - registered by globops module");
    println!("  +x - Test mode - registered manually for demonstration");
    
    println!("\nTesting scenarios:");
    println!("==================");
    println!("1. Connect as regular user:");
    println!("   - Set globops mode: MODE yournick +g");
    println!("   - Set test mode: MODE yournick +x");
    println!("   - Set multiple modes: MODE yournick +gx");
    
    println!("\n2. Connect as operator:");
    println!("   - Send wallops: WALLOPS :This is a wallops message");
    println!("   - Send globops: GLOBOPS :This is a globops message");
    println!("   - Only users with respective modes will receive messages");
    
    println!("\n3. Mode validation:");
    println!("   - Non-operators cannot set operator-required modes");
    println!("   - Users can only set self-only modes on themselves");
    println!("   - Custom modes follow the same validation rules as core modes");
    
    println!("\n4. Module integration:");
    println!("   - GLOBOPS module automatically registers +g mode on startup");
    println!("   - Core MODE command validates both core and custom modes");
    println!("   - Custom modes can be unregistered when modules are unloaded");
    
    println!("\nPress Ctrl+C to stop the server...");
    
    // Wait for server to finish (or be interrupted)
    if let Err(e) = server_handle.await {
        eprintln!("Server task error: {}", e);
    }

    // Cleanup: Unregister custom modes
    println!("\nCleaning up custom modes...");
    if let Err(e) = unregister_custom_mode('x', "example") {
        println!("   ✗ Failed to unregister mode 'x': {}", e);
    } else {
        println!("   ✓ Successfully unregistered mode 'x'");
    }

    Ok(())
}

/// Demonstrate the extensible mode system architecture
#[allow(dead_code)]
fn demonstrate_architecture() {
    println!("\nExtensible Mode System Architecture");
    println!("===================================");
    
    println!("\n1. Core Components:");
    println!("   - ExtensibleModeRegistry: Manages custom mode registration");
    println!("   - CustomUserMode: Defines mode properties and validation rules");
    println!("   - Global registry: Thread-safe access to registered modes");
    
    println!("\n2. Module Integration:");
    println!("   - Modules register modes during initialization");
    println!("   - Core MODE command validates both core and custom modes");
    println!("   - Modules can unregister modes during cleanup");
    
    println!("\n3. Mode Properties:");
    println!("   - character: The mode letter (e.g., 'g', 'x')");
    println!("   - description: Human-readable description");
    println!("   - requires_operator: Whether operator privileges needed to set/unset");
    println!("   - self_only: Whether only the user can set it on themselves");
    println!("   - oper_only: Whether only OPER command can set it");
    println!("   - module_name: Which module registered this mode");
    
    println!("\n4. Benefits:");
    println!("   - Core doesn't need to know about all possible modes");
    println!("   - Modules can define their own mode requirements");
    println!("   - Easy to add new messaging commands with custom modes");
    println!("   - Proper validation and error handling for custom modes");
    println!("   - Clean separation between core and module functionality");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustircd_core::{register_custom_mode, unregister_custom_mode, CustomUserMode};

    #[test]
    fn test_custom_mode_registration() {
        let test_mode = CustomUserMode {
            character: 't',
            description: "Test mode".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "test".to_string(),
        };
        
        // Register mode
        assert!(register_custom_mode(test_mode).is_ok());
        
        // Try to register same mode again (should fail)
        let duplicate_mode = CustomUserMode {
            character: 't',
            description: "Duplicate test mode".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "test2".to_string(),
        };
        assert!(register_custom_mode(duplicate_mode).is_err());
        
        // Unregister mode
        assert!(unregister_custom_mode('t', "test").is_ok());
        
        // Try to unregister non-existent mode (should fail)
        assert!(unregister_custom_mode('t', "test").is_err());
    }

    #[test]
    fn test_mode_conflict_detection() {
        // Try to register a core mode (should fail)
        let core_mode = CustomUserMode {
            character: 'o',  // Core operator mode
            description: "Custom operator mode".to_string(),
            requires_operator: true,
            self_only: false,
            oper_only: true,
            module_name: "test".to_string(),
        };
        
        assert!(register_custom_mode(core_mode).is_err());
    }

    #[test]
    fn test_mode_properties() {
        let operator_mode = CustomUserMode {
            character: 'a',
            description: "Admin mode".to_string(),
            requires_operator: true,
            self_only: false,
            oper_only: false,
            module_name: "admin".to_string(),
        };
        
        assert_eq!(operator_mode.character, 'a');
        assert_eq!(operator_mode.description, "Admin mode");
        assert!(operator_mode.requires_operator);
        assert!(!operator_mode.self_only);
        assert!(!operator_mode.oper_only);
        assert_eq!(operator_mode.module_name, "admin");
    }
}
