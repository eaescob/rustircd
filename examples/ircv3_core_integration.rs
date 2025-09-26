//! IRCv3 Core Integration Example
//! 
//! This example demonstrates how all IRCv3 capabilities integrate
//! with the core system through the extension framework.

use rustircd_core::{Server, Config, User, Message, MessageType, ExtensionManager};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== IRCv3 Core Integration Demo ===\n");

    // Create server configuration
    let config = Config::default();
    
    // Create server instance
    let server = Server::new(config);
    
    // Register IRCv3 extensions
    server.register_ircv3_extensions().await?;
    
    // Demonstrate the integration
    demonstrate_capability_negotiation(&server).await?;
    demonstrate_message_tagging(&server).await?;
    demonstrate_user_extensions(&server).await?;
    demonstrate_message_extensions(&server).await?;
    
    Ok(())
}

async fn demonstrate_capability_negotiation(server: &Server) -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Capability Negotiation Integration");
    println!("====================================");
    
    let extension_manager = server.extension_manager();
    
    // Get all supported capabilities
    let capabilities = extension_manager.get_all_capabilities().await?;
    println!("Supported capabilities: {:?}", capabilities);
    
    // Check specific capabilities
    let supports_message_tags = extension_manager.supports_capability("message-tags").await;
    let supports_bot_mode = extension_manager.supports_capability("bot-mode").await;
    let supports_server_time = extension_manager.supports_capability("server-time").await;
    
    println!("Supports message-tags: {}", supports_message_tags);
    println!("Supports bot-mode: {}", supports_bot_mode);
    println!("Supports server-time: {}", supports_server_time);
    
    println!();
    Ok(())
}

async fn demonstrate_message_tagging(server: &Server) -> Result<(), Box<dyn std::error::Error>> {
    println!("2. Message Tagging Integration");
    println!("==============================");
    
    let extension_manager = server.extension_manager();
    
    // Create a test user
    let user = User::new(
        "TestUser".to_string(),
        "testuser".to_string(),
        "Test User".to_string(),
        "test.example.com".to_string(),
        "localhost".to_string(),
    );
    
    // Create a test message
    let message = Message::new(
        MessageType::PrivMsg,
        vec!["#general".to_string(), "Hello world!".to_string()],
    );
    
    // Generate outgoing tags
    let tags = extension_manager.generate_outgoing_tags(&user, &message).await?;
    println!("Generated tags: {:?}", tags);
    
    // Process incoming tags
    let mut incoming_tags = HashMap::new();
    incoming_tags.insert("time".to_string(), "2023-01-01T12:00:00Z".to_string());
    incoming_tags.insert("account".to_string(), "testuser".to_string());
    
    let processed_tags = extension_manager.process_incoming_tags(&rustircd_core::Client::new(Uuid::new_v4()), &incoming_tags).await?;
    println!("Processed incoming tags: {:?}", processed_tags);
    
    // Validate tags
    extension_manager.validate_tags(&processed_tags).await?;
    println!("Tags validation: OK");
    
    println!();
    Ok(())
}

async fn demonstrate_user_extensions(server: &Server) -> Result<(), Box<dyn std::error::Error>> {
    println!("3. User Extension Integration");
    println!("=============================");
    
    let extension_manager = server.extension_manager();
    
    // Create a test user
    let mut user = User::new(
        "ExtensionUser".to_string(),
        "extuser".to_string(),
        "Extension Test User".to_string(),
        "ext.example.com".to_string(),
        "localhost".to_string(),
    );
    
    // Simulate user registration
    extension_manager.on_user_registration(&user).await?;
    println!("User registration hooks called");
    
    // Simulate user property change
    extension_manager.on_user_property_change(&user, "away", "false", "true").await?;
    println!("User property change hooks called");
    
    // Simulate user join channel
    extension_manager.on_user_join_channel(&user, "#test").await?;
    println!("User join channel hooks called");
    
    // Simulate user away change
    extension_manager.on_user_away_change(&user, true, Some("I'm away")).await?;
    println!("User away change hooks called");
    
    // Simulate user disconnection
    extension_manager.on_user_disconnection(&user).await?;
    println!("User disconnection hooks called");
    
    println!();
    Ok(())
}

async fn demonstrate_message_extensions(server: &Server) -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Message Extension Integration");
    println!("================================");
    
    let extension_manager = server.extension_manager();
    
    // Create a test client
    let client = rustircd_core::Client::new(Uuid::new_v4());
    
    // Create a test message
    let message = Message::new(
        MessageType::PrivMsg,
        vec!["#general".to_string(), "Test message".to_string()],
    );
    
    // Simulate message preprocessing
    let processed_message = extension_manager.on_message_preprocess(&client, &message).await?;
    println!("Message preprocessing: {:?}", processed_message.is_some());
    
    // Simulate message postprocessing
    extension_manager.on_message_postprocess(&client, &message, &rustircd_core::module::ModuleResult::Handled).await?;
    println!("Message postprocessing: OK");
    
    // Create a test user for message sending
    let user = User::new(
        "TargetUser".to_string(),
        "target".to_string(),
        "Target User".to_string(),
        "target.example.com".to_string(),
        "localhost".to_string(),
    );
    
    // Simulate message sending
    let send_message = extension_manager.on_message_send(&user, &message).await?;
    println!("Message sending: {:?}", send_message.is_some());
    
    println!();
    Ok(())
}

// Helper function to demonstrate the complete integration flow
fn demonstrate_integration_flow() {
    println!("=== Complete IRCv3 Integration Flow ===");
    println!();
    println!("1. Server Initialization:");
    println!("   - Create ExtensionManager");
    println!("   - Register IRCv3 extensions");
    println!("   - Initialize capability negotiation");
    println!();
    println!("2. Client Connection:");
    println!("   - Client connects to server");
    println!("   - Capability negotiation begins");
    println!("   - Extensions register their capabilities");
    println!("   - Client requests specific capabilities");
    println!();
    println!("3. Message Processing:");
    println!("   - Message received from client");
    println!("   - Message preprocessing hooks called");
    println!("   - Message tags processed and validated");
    println!("   - Message sent to target(s)");
    println!("   - Message postprocessing hooks called");
    println!();
    println!("4. User Operations:");
    println!("   - User registration triggers hooks");
    println!("   - User property changes trigger hooks");
    println!("   - Channel operations trigger hooks");
    println!("   - User disconnection triggers hooks");
    println!();
    println!("5. Capability Features:");
    println!("   - Message tagging (server-time, account, bot, away)");
    println!("   - Echo message for message confirmation");
    println!("   - Batch processing for multiple messages");
    println!("   - Account tracking and user properties");
    println!("   - Away notification system");
    println!("   - Bot mode identification");
    println!();
    println!("Key Benefits:");
    println!("- Core remains simple and focused");
    println!("- IRCv3 capabilities are modular and optional");
    println!("- Easy to add new capabilities");
    println!("- Clean separation of concerns");
    println!("- Extensible architecture");
}
