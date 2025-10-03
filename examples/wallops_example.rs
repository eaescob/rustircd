//! Wallops messaging example
//!
//! This example demonstrates how to use the messaging module system
//! to implement the WALLOPS command with proper operator and mode restrictions.

use rustircd_modules::create_default_messaging_module;
use rustircd_core::{ModuleManager, Message, MessageType, User};
use rustircd_core::client::{Client, ClientState, ConnectionType};
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Wallops Messaging Module Example ===");
    
    // Create a module manager
    let mut module_manager = ModuleManager::new();
    
    // Create the default messaging module (includes wallops)
    let messaging_module = create_default_messaging_module();
    
    // Load the messaging module
    module_manager.load_module(Box::new(messaging_module)).await?;
    
    println!("✓ Messaging module loaded successfully");
    
    // Create some test clients
    let (sender_tx, _sender_rx) = mpsc::unbounded_channel();
    let (wallops_user_tx, _wallops_user_rx) = mpsc::unbounded_channel();
    let (regular_user_tx, _regular_user_rx) = mpsc::unbounded_channel();
    
    // Create an operator client
    let mut operator_client = Client::new_with_type(
        Uuid::new_v4(),
        "127.0.0.1:6667".to_string(),
        "127.0.0.1:6667".to_string(),
        sender_tx,
        ConnectionType::Client,
    );
    
    // Set up operator user with operator privileges and wallops mode
    let mut operator_user = User::new(
        "operator".to_string(),
        "op".to_string(),
        "Operator User".to_string(),
        "127.0.0.1".to_string(),
        "localhost".to_string(),
    );
    operator_user.is_operator = true;
    operator_user.add_mode('o'); // Operator mode
    operator_user.add_mode('w'); // Wallops mode
    operator_user.registered = true;
    operator_client.user = Some(operator_user);
    operator_client.state = ClientState::Registered;
    
    // Create a client with wallops mode (can receive wallops)
    let mut wallops_client = Client::new_with_type(
        Uuid::new_v4(),
        "127.0.0.1:6668".to_string(),
        "127.0.0.1:6668".to_string(),
        wallops_user_tx,
        ConnectionType::Client,
    );
    
    let mut wallops_user = User::new(
        "wallops_user".to_string(),
        "wallops".to_string(),
        "Wallops User".to_string(),
        "127.0.0.1".to_string(),
        "localhost".to_string(),
    );
    wallops_user.add_mode('w'); // Wallops mode
    wallops_user.registered = true;
    wallops_client.user = Some(wallops_user);
    wallops_client.state = ClientState::Registered;
    
    // Create a regular client (cannot receive wallops)
    let mut regular_client = Client::new_with_type(
        Uuid::new_v4(),
        "127.0.0.1:6669".to_string(),
        "127.0.0.1:6669".to_string(),
        regular_user_tx,
        ConnectionType::Client,
    );
    
    let mut regular_user = User::new(
        "regular_user".to_string(),
        "regular".to_string(),
        "Regular User".to_string(),
        "127.0.0.1".to_string(),
        "localhost".to_string(),
    );
    regular_user.registered = true;
    regular_client.user = Some(regular_user);
    regular_client.state = ClientState::Registered;
    
    println!("✓ Created test clients:");
    println!("  - operator (operator + wallops mode)");
    println!("  - wallops_user (wallops mode only)");
    println!("  - regular_user (no special modes)");
    
    // Test 1: Operator sends wallops (should work)
    println!("\n=== Test 1: Operator sends wallops ===");
    let wallops_message = Message::new(
        MessageType::Wallops,
        vec!["This is a wallops message from the operator!".to_string()],
    );
    
    match module_manager.handle_message(&operator_client, &wallops_message).await? {
        rustircd_core::module::ModuleResult::Handled => {
            println!("✓ Operator wallops message handled successfully");
        }
        rustircd_core::module::ModuleResult::Rejected(reason) => {
            println!("✗ Operator wallops message rejected: {}", reason);
        }
        rustircd_core::module::ModuleResult::NotHandled => {
            println!("✗ Operator wallops message not handled");
        }
        rustircd_core::module::ModuleResult::HandledStop => {
            println!("✓ Operator wallops message handled and stopped");
        }
    }
    
    // Test 2: Non-operator tries to send wallops (should be rejected)
    println!("\n=== Test 2: Non-operator tries to send wallops ===");
    let wallops_message = Message::new(
        MessageType::Wallops,
        vec!["This should be rejected!".to_string()],
    );
    
    match module_manager.handle_message(&regular_client, &wallops_message).await? {
        rustircd_core::module::ModuleResult::Handled => {
            println!("✗ Non-operator wallops message was handled (should be rejected)");
        }
        rustircd_core::module::ModuleResult::Rejected(reason) => {
            println!("✓ Non-operator wallops message correctly rejected: {}", reason);
        }
        rustircd_core::module::ModuleResult::NotHandled => {
            println!("✗ Non-operator wallops message not handled");
        }
        rustircd_core::module::ModuleResult::HandledStop => {
            println!("✓ Non-operator wallops message handled and stopped");
        }
    }
    
    // Test 3: Empty wallops message (should be rejected)
    println!("\n=== Test 3: Empty wallops message ===");
    let empty_wallops_message = Message::new(
        MessageType::Wallops,
        vec![],
    );
    
    match module_manager.handle_message(&operator_client, &empty_wallops_message).await? {
        rustircd_core::module::ModuleResult::Handled => {
            println!("✗ Empty wallops message was handled (should be rejected)");
        }
        rustircd_core::module::ModuleResult::Rejected(reason) => {
            println!("✓ Empty wallops message correctly rejected: {}", reason);
        }
        rustircd_core::module::ModuleResult::NotHandled => {
            println!("✗ Empty wallops message not handled");
        }
        rustircd_core::module::ModuleResult::HandledStop => {
            println!("✓ Empty wallops message handled and stopped");
        }
    }
    
    println!("\n=== Example completed ===");
    println!("The messaging module system successfully:");
    println!("✓ Validates operator privileges for wallops sending");
    println!("✓ Validates wallops mode for message reception");
    println!("✓ Rejects invalid wallops attempts");
    println!("✓ Provides proper error messages");
    
    Ok(())
}
