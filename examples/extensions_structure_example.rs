//! Extensions Structure Example
//! 
//! This example demonstrates the new modular extensions structure
//! that follows Solanum's pattern with separate files per extension.

use rustircd_core::{Config, CoreExtensionManager, User, Message, Client};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Rust IRC Daemon - Extensions Structure Example");
    println!("==============================================");
    
    // Load configuration
    let config = Config::from_file("examples/configs/services.toml")?;
    
    // Initialize core extensions
    let core_extensions = CoreExtensionManager::new("services.example.org".to_string());
    core_extensions.initialize().await?;
    
    println!("✓ Core extensions initialized");
    println!();
    
    // Demonstrate individual extension access
    println!("Individual Extension Access:");
    println!("===========================");
    
    // Account tracking extension
    let account_tracking = core_extensions.get_account_tracking();
    println!("✓ Account tracking extension loaded");
    
    // Identify message extension
    let identify_msg = core_extensions.get_identify_message();
    println!("✓ Identify message extension loaded");
    
    // Server time extension
    let server_time = core_extensions.get_server_time();
    println!("✓ Server time extension loaded");
    
    // Batch extension
    let batch = core_extensions.get_batch();
    println!("✓ Batch extension loaded");
    
    println!();
    
    // Demonstrate extension functionality
    println!("Extension Functionality Demo:");
    println!("============================");
    
    // Create a test user
    let user = User::new(
        "testuser".to_string(),
        "testuser".to_string(),
        "localhost".to_string(),
        "Test User".to_string(),
        "test.ircd.org".to_string(),
    );
    
    println!("Test user: {}", user.nick);
    
    // Test account tracking
    println!("\nAccount Tracking:");
    account_tracking.set_account(user.id, "testaccount".to_string()).await?;
    let account_info = account_tracking.get_account(user.id).await;
    println!("  Account info: {:?}", account_info);
    
    let is_identified = account_tracking.is_identified(user.id).await;
    println!("  Is identified: {}", is_identified);
    
    // Test message tag generation
    println!("\nMessage Tag Generation:");
    let message = Message {
        prefix: Some("testuser!testuser@localhost".to_string()),
        command: "PRIVMSG".to_string(),
        params: vec!["#test".to_string(), "Hello, world!".to_string()],
    };
    
    let mut client = Client::new(Uuid::new_v4());
    client.user = Some(user.clone());
    
    // Test identify message tags
    let identify_tags = identify_msg.generate_outgoing_tags(&user, &message).await?;
    println!("  Identify tags: {:?}", identify_tags);
    
    // Test server time tags
    let time_tags = server_time.generate_outgoing_tags(&user, &message).await?;
    println!("  Time tags: {:?}", time_tags);
    
    // Test batch functionality
    println!("\nBatch Functionality:");
    let batch_id = "test-batch-123".to_string();
    batch.start_batch(batch_id.clone(), "test".to_string(), user.id).await?;
    println!("  Started batch: {}", batch_id);
    
    let has_batch = batch.has_batch(&batch_id).await;
    println!("  Batch exists: {}", has_batch);
    
    batch.add_to_batch(&batch_id, message.clone()).await?;
    println!("  Added message to batch");
    
    let batch_messages = batch.end_batch(&batch_id).await?;
    println!("  Ended batch, messages: {}", batch_messages.unwrap_or_default().len());
    
    println!();
    
    // Show extension structure
    println!("Extension Structure:");
    println!("===================");
    println!("core/src/extensions/");
    println!("├── mod.rs                 # Module definitions and manager");
    println!("├── identify_msg.rs        # Identify message extension");
    println!("├── account_tracking.rs    # Account tracking extension");
    println!("├── server_time.rs         # Server time extension");
    println!("├── batch.rs               # Batch extension");
    println!("└── README.md              # Documentation");
    println!();
    
    println!("Benefits of this structure:");
    println!("- Each extension is in its own file");
    println!("- Easy to add new extensions");
    println!("- Follows Solanum's modular pattern");
    println!("- Better maintainability and organization");
    println!("- Clear separation of concerns");
    
    println!("\nExample completed successfully!");
    
    Ok(())
}
