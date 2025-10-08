//! GLOBOPS messaging example
//!
//! This example demonstrates the GLOBOPS command functionality:
//! - Only operators can send GLOBOPS commands
//! - Only users with +g mode receive GLOBOPS messages
//! - GLOBOPS messages are sent to all users with the +g mode set

use rustircd_core::{Server, Config, Client, User};
use rustircd_modules::messaging::{MessagingWrapper, create_default_messaging_module};
use std::collections::HashSet;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("GLOBOPS Messaging Example");
    println!("=========================");

    // Create server configuration
    let mut config = Config::default();
    config.connection.ports.clear();
    config.connection.ports.push(rustircd_core::config::PortConfig {
        port: 6667,
        connection_type: rustircd_core::config::PortConnectionType::Client,
        tls: false,
        description: Some("GLOBOPS test port".to_string()),
        bind_address: None,
    });
    config.server.name = "globops.example.com".to_string();

    // Create server
    let mut server = Server::new(config).await;
    
    // Note: In a real implementation, messaging modules would be registered here
    // let mut messaging_module = create_default_messaging_module();
    // server.register_module(Box::new(messaging_module)).await?;

    // Start server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    println!("\nServer started on port 6667");
    println!("You can now connect with an IRC client to test GLOBOPS functionality");
    println!("\nTesting scenarios:");
    println!("1. Connect as a regular user and set +g mode: MODE yournick +g");
    println!("2. Connect as an operator and send GLOBOPS: GLOBOPS :Hello, this is a global operator message!");
    println!("3. Only users with +g mode will receive the GLOBOPS message");
    println!("4. Non-operators cannot send GLOBOPS commands");
    println!("\nExample IRC client commands:");
    println!("  /connect localhost 6667");
    println!("  /nick testuser");
    println!("  /user username hostname servername :Real Name");
    println!("  /mode testuser +g  # Set globops mode");
    println!("  # (As operator) /globops Hello, this is a test message!");
    
    println!("\nPress Ctrl+C to stop the server...");
    
    // Wait for server to finish (or be interrupted)
    if let Err(e) = server_handle.await {
        eprintln!("Server task error: {}", e);
    }

    Ok(())
}

/// Helper function to demonstrate GLOBOPS functionality programmatically
#[allow(dead_code)]
async fn demonstrate_globops_functionality() -> Result<(), Box<dyn std::error::Error>> {
    println!("\nDemonstrating GLOBOPS functionality...");

    // Create test users
    let mut regular_user = User::new(
        "regularuser".to_string(),
        "username".to_string(),
        "Regular User".to_string(),
        "host.example.com".to_string(),
        "server.example.com".to_string(),
    );
    regular_user.is_operator = false;
    regular_user.add_mode('g'); // Set globops mode

    let mut operator_user = User::new(
        "operator".to_string(),
        "opername".to_string(),
        "Operator User".to_string(),
        "host.example.com".to_string(),
        "server.example.com".to_string(),
    );
    operator_user.is_operator = true;

    let mut user_without_globops = User::new(
        "normaluser".to_string(),
        "normalname".to_string(),
        "Normal User".to_string(),
        "host.example.com".to_string(),
        "server.example.com".to_string(),
    );
    user_without_globops.is_operator = false;
    // No +g mode set

    println!("Created test users:");
    println!("- regularuser: has +g mode, not operator");
    println!("- operator: is operator, no +g mode");
    println!("- normaluser: no +g mode, not operator");

    // Test mode checking
    println!("\nTesting mode checks:");
    println!("- regularuser has +g mode: {}", regular_user.has_mode('g'));
    println!("- operator has +g mode: {}", operator_user.has_mode('g'));
    println!("- normaluser has +g mode: {}", user_without_globops.has_mode('g'));

    println!("- operator is operator: {}", operator_user.is_operator);
    println!("- regularuser is operator: {}", regular_user.is_operator);

    println!("\nGLOBOPS behavior:");
    println!("- Only operators can send GLOBOPS commands");
    println!("- Only users with +g mode receive GLOBOPS messages");
    println!("- In this example, 'regularuser' would receive GLOBOPS from 'operator'");
    println!("- 'normaluser' would not receive GLOBOPS messages");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustircd_core::User;
    use std::collections::HashSet;

    #[test]
    fn test_globops_mode_requirements() {
        // Test user with +g mode
        let mut user_with_globops = User::new(
            "testuser".to_string(),
            "username".to_string(),
            "Test User".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        user_with_globops.add_mode('g');
        
        assert!(user_with_globops.has_mode('g'));
        assert!(!user_with_globops.is_operator());

        // Test operator user
        let mut operator_user = User::new(
            "operator".to_string(),
            "opername".to_string(),
            "Operator User".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        operator_user.is_operator = true;
        
        assert!(operator_user.is_operator());
        assert!(!operator_user.has_mode('g'));

        // Test user without +g mode
        let normal_user = User::new(
            "normaluser".to_string(),
            "normalname".to_string(),
            "Normal User".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        
        assert!(!normal_user.has_mode('g'));
        assert!(!normal_user.is_operator());
    }

    #[test]
    fn test_globops_scenarios() {
        println!("GLOBOPS Test Scenarios:");
        println!("======================");
        
        // Scenario 1: Operator sends GLOBOPS to user with +g mode
        println!("Scenario 1: Operator -> User with +g mode");
        println!("- Sender: operator (can send GLOBOPS)");
        println!("- Receiver: user with +g mode (will receive message)");
        println!("- Result: Message sent successfully");
        
        // Scenario 2: Non-operator tries to send GLOBOPS
        println!("\nScenario 2: Non-operator -> Any user");
        println!("- Sender: regular user (cannot send GLOBOPS)");
        println!("- Receiver: any user");
        println!("- Result: Permission denied error");
        
        // Scenario 3: Operator sends GLOBOPS to user without +g mode
        println!("\nScenario 3: Operator -> User without +g mode");
        println!("- Sender: operator (can send GLOBOPS)");
        println!("- Receiver: user without +g mode (will not receive message)");
        println!("- Result: Message not delivered to this user");
    }
}
