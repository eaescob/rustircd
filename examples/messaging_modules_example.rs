//! Messaging Modules Configuration Example
//!
//! This example demonstrates how to configure and use the modular messaging system
//! with different messaging commands (WALLOPS, GLOBOPS) and their custom mode requirements.

use rustircd_core::{Server, ServerConfig};
use rustircd_modules::messaging::{MessagingWrapper, WallopsModule, GlobopsModule};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Messaging Modules Configuration Example");
    println!("======================================");

    // Create server configuration
    let config = ServerConfig::default()
        .with_port(6667)
        .with_server_name("messaging.example.com");

    // Create server
    let server = Server::new(config).await?;
    
    // Example 1: Create messaging module with default configuration (WALLOPS + GLOBOPS)
    println!("\n1. Creating default messaging module with WALLOPS and GLOBOPS...");
    let default_messaging = create_default_messaging_module();
    println!("   Default messaging module created with commands: {:?}", 
             default_messaging.manager().get_commands());

    // Example 2: Create custom messaging module with only WALLOPS
    println!("\n2. Creating custom messaging module with only WALLOPS...");
    let mut wallops_only = MessagingWrapper::new(
        "wallops-only".to_string(),
        "1.0.0".to_string(),
        "WALLOPS messaging only".to_string(),
    );
    wallops_only.register_messaging_module(Box::new(WallopsModule::new()));
    println!("   WALLOPS-only module created with commands: {:?}", 
             wallops_only.manager().get_commands());

    // Example 3: Create custom messaging module with only GLOBOPS
    println!("\n3. Creating custom messaging module with only GLOBOPS...");
    let mut globops_only = MessagingWrapper::new(
        "globops-only".to_string(),
        "1.0.0".to_string(),
        "GLOBOPS messaging only".to_string(),
    );
    globops_only.register_messaging_module(Box::new(GlobopsModule::new()));
    println!("   GLOBOPS-only module created with commands: {:?}", 
             globops_only.manager().get_commands());

    // Example 4: Create custom messaging module with both commands
    println!("\n4. Creating custom messaging module with both WALLOPS and GLOBOPS...");
    let mut both_messaging = MessagingWrapper::new(
        "both-messaging".to_string(),
        "1.0.0".to_string(),
        "WALLOPS and GLOBOPS messaging".to_string(),
    );
    both_messaging.register_messaging_module(Box::new(WallopsModule::new()));
    both_messaging.register_messaging_module(Box::new(GlobopsModule::new()));
    println!("   Both messaging module created with commands: {:?}", 
             both_messaging.manager().get_commands());

    // Register the default messaging module with the server
    server.register_module(Box::new(default_messaging)).await?;

    // Start server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    println!("\nServer started on port 6667 with messaging modules");
    println!("\nAvailable commands:");
    println!("==================");
    println!("WALLOPS <message> - Send message to users with +w mode (operator required)");
    println!("GLOBOPS <message> - Send message to users with +g mode (operator required)");
    
    println!("\nUser modes:");
    println!("===========");
    println!("+w - Wallops mode (receives WALLOPS messages)");
    println!("+g - Globops mode (receives GLOBOPS messages)");
    
    println!("\nTesting scenarios:");
    println!("==================");
    println!("1. Connect as regular user:");
    println!("   - Set wallops mode: MODE yournick +w");
    println!("   - Set globops mode: MODE yournick +g");
    println!("   - Set both modes: MODE yournick +wg");
    
    println!("\n2. Connect as operator:");
    println!("   - Send wallops: WALLOPS :This is a wallops message");
    println!("   - Send globops: GLOBOPS :This is a globops message");
    println!("   - Only users with respective modes will receive messages");
    
    println!("\n3. Permission testing:");
    println!("   - Non-operators cannot send WALLOPS or GLOBOPS");
    println!("   - Users without +w mode won't receive WALLOPS");
    println!("   - Users without +g mode won't receive GLOBOPS");
    
    println!("\nPress Ctrl+C to stop the server...");
    
    // Wait for server to finish (or be interrupted)
    if let Err(e) = server_handle.await {
        eprintln!("Server task error: {}", e);
    }

    Ok(())
}

/// Demonstrate how messaging modules can be configured via configuration
#[allow(dead_code)]
fn demonstrate_configuration_approach() {
    println!("\nConfiguration-Based Messaging Modules");
    println!("=====================================");
    
    // This shows how the system could be extended to support configuration-based loading
    let example_config = r#"
[messaging]
enabled = true

[messaging.modules]
wallops = { enabled = true, require_operator = true, receiver_mode = "w" }
globops = { enabled = true, require_operator = true, receiver_mode = "g" }
"#;
    
    println!("Example configuration file:");
    println!("{}", example_config);
    
    println!("\nBenefits of this approach:");
    println!("- Messaging modules can be enabled/disabled via configuration");
    println!("- Custom modes can be defined per module");
    println!("- Core doesn't need to know about all possible user modes");
    println!("- Easy to add new messaging commands without core changes");
    println!("- Modules can define their own mode requirements");
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustircd_modules::messaging::{WallopsModule, GlobopsModule};

    #[test]
    fn test_messaging_module_creation() {
        let wallops = WallopsModule::new();
        let globops = GlobopsModule::new();
        
        assert_eq!(wallops.command(), "WALLOPS");
        assert_eq!(globops.command(), "GLOBOPS");
    }

    #[test]
    fn test_messaging_wrapper_creation() {
        let mut wrapper = MessagingWrapper::new(
            "test".to_string(),
            "1.0.0".to_string(),
            "Test messaging".to_string(),
        );
        
        wrapper.register_messaging_module(Box::new(WallopsModule::new()));
        wrapper.register_messaging_module(Box::new(GlobopsModule::new()));
        
        let commands = wrapper.manager().get_commands();
        assert!(commands.contains(&"WALLOPS"));
        assert!(commands.contains(&"GLOBOPS"));
    }

    #[test]
    fn test_help_text() {
        let wallops = WallopsModule::new();
        let globops = GlobopsModule::new();
        
        assert!(wallops.help_text().contains("WALLOPS"));
        assert!(wallops.help_text().contains("+w"));
        assert!(wallops.help_text().contains("operator"));
        
        assert!(globops.help_text().contains("GLOBOPS"));
        assert!(globops.help_text().contains("+g"));
        assert!(globops.help_text().contains("operator"));
    }
}
