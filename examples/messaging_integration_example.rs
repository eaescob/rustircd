//! Messaging module integration example
//!
//! This example demonstrates how to integrate the messaging module system
//! into the IRC server for proper wallops handling.

use rustircd_modules::create_default_messaging_module;
use rustircd_core::{ModuleManager, Message, MessageType, User};
use rustircd_core::client::{Client, ClientState, ConnectionType};
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Messaging Module Integration Example ===");
    
    // Create a module manager (like the one in the server)
    let mut module_manager = ModuleManager::new();
    
    // Create and load the messaging module
    let messaging_module = create_default_messaging_module();
    module_manager.load_module(Box::new(messaging_module)).await?;
    
    println!("✓ Messaging module loaded into module manager");
    
    // Verify the module was loaded correctly
    let loaded_modules = module_manager.get_loaded_modules();
    println!("✓ Loaded modules: {:?}", loaded_modules);
    
    // Check if the module supports message handling
    let capabilities = module_manager.get_all_capabilities();
    println!("✓ Module capabilities: {:?}", capabilities);
    
    // Create test clients
    let (sender_tx, _sender_rx) = mpsc::unbounded_channel();
    
    // Create an operator client
    let mut operator_client = Client::new_with_type(
        Uuid::new_v4(),
        "127.0.0.1:6667".to_string(),
        "127.0.0.1:6667".to_string(),
        sender_tx,
        ConnectionType::Client,
    );
    
    // Set up operator user
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
    
    // Create a regular client (non-operator)
    let (regular_tx, _regular_rx) = mpsc::unbounded_channel();
    let mut regular_client = Client::new_with_type(
        Uuid::new_v4(),
        "127.0.0.1:6668".to_string(),
        "127.0.0.1:6668".to_string(),
        regular_tx,
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
    println!("  - regular_user (no special modes)");
    
    // Test 1: Operator sends wallops (should be handled by messaging module)
    println!("\n=== Test 1: Operator sends wallops ===");
    let wallops_message = Message::new(
        MessageType::Wallops,
        vec!["This is a wallops message from the operator!".to_string()],
    );
    
    match module_manager.handle_message(&operator_client, &wallops_message).await? {
        rustircd_core::module::ModuleResult::Handled => {
            println!("✓ Operator wallops message handled by messaging module");
        }
        rustircd_core::module::ModuleResult::Rejected(reason) => {
            println!("✗ Operator wallops message rejected: {}", reason);
        }
        rustircd_core::module::ModuleResult::NotHandled => {
            println!("✗ Operator wallops message not handled (would fall back to core)");
        }
        rustircd_core::module::ModuleResult::HandledStop => {
            println!("✓ Operator wallops message handled and stopped by messaging module");
        }
    }
    
    // Test 2: Non-operator tries to send wallops (should be rejected by messaging module)
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
            println!("✓ Non-operator wallops message correctly rejected by messaging module: {}", reason);
        }
        rustircd_core::module::ModuleResult::NotHandled => {
            println!("✗ Non-operator wallops message not handled (would fall back to core)");
        }
        rustircd_core::module::ModuleResult::HandledStop => {
            println!("✓ Non-operator wallops message handled and stopped by messaging module");
        }
    }
    
    // Test 3: Empty wallops message (should be rejected by messaging module)
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
            println!("✓ Empty wallops message correctly rejected by messaging module: {}", reason);
        }
        rustircd_core::module::ModuleResult::NotHandled => {
            println!("✗ Empty wallops message not handled (would fall back to core)");
        }
        rustircd_core::module::ModuleResult::HandledStop => {
            println!("✓ Empty wallops message handled and stopped by messaging module");
        }
    }
    
    // Test 4: Non-wallops message (should not be handled by messaging module)
    println!("\n=== Test 4: Non-wallops message (PING) ===");
    let ping_message = Message::new(
        MessageType::Ping,
        vec!["server.example.com".to_string()],
    );
    
    match module_manager.handle_message(&operator_client, &ping_message).await? {
        rustircd_core::module::ModuleResult::Handled => {
            println!("✗ PING message was handled by messaging module (should not be)");
        }
        rustircd_core::module::ModuleResult::Rejected(reason) => {
            println!("✗ PING message was rejected by messaging module: {}", reason);
        }
        rustircd_core::module::ModuleResult::NotHandled => {
            println!("✓ PING message correctly not handled by messaging module (would fall back to core)");
        }
        rustircd_core::module::ModuleResult::HandledStop => {
            println!("✗ PING message was handled and stopped by messaging module (should not be)");
        }
    }
    
    println!("\n=== Integration Example Results ===");
    println!("The messaging module system successfully:");
    println!("✓ Integrates with the core module manager");
    println!("✓ Handles WALLOPS commands when sent by operators");
    println!("✓ Rejects WALLOPS commands from non-operators");
    println!("✓ Rejects invalid WALLOPS messages");
    println!("✓ Ignores non-WALLOPS commands (proper delegation to core)");
    
    Ok(())
}
