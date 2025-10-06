//! Example demonstrating IRCv3 Extended Join and Multi-Prefix capabilities
//! 
//! This example shows how to use the Extended Join and Multi-Prefix capabilities
//! in the rustircd IRC server.

use rustircd_core::{Server, Config, Result};
use rustircd_modules::ircv3::Ircv3Module;
use std::collections::HashSet;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Starting rustircd with IRCv3 Extended Join and Multi-Prefix capabilities...");
    
    // Create configuration
    let config = Config::from_file("config.toml")?;
    
    // Create IRCv3 module
    let mut ircv3_module = Ircv3Module::new();
    
    // Initialize the module
    ircv3_module.init().await?;
    
    // Create server
    let mut server = Server::new(config)?;
    
    // Add the IRCv3 module to the server
    server.add_module(Box::new(ircv3_module))?;
    
    // Start the server
    println!("Server starting on port 6667...");
    println!("Connect with an IRC client and try these commands:");
    println!();
    println!("1. Enable Extended Join capability:");
    println!("   CAP REQ :extended-join");
    println!();
    println!("2. Enable Multi-Prefix capability:");
    println!("   CAP REQ :multi-prefix");
    println!();
    println!("3. Join a channel to see extended join information:");
    println!("   JOIN #test");
    println!();
    println!("4. Use NAMES to see multiple prefixes:");
    println!("   NAMES #test");
    println!();
    println!("5. End capability negotiation:");
    println!("   CAP END");
    println!();
    
    // Run the server
    server.run().await?;
    
    Ok(())
}

/// Example of how to manually test the capabilities
#[allow(dead_code)]
async fn test_capabilities() -> Result<()> {
    let mut ircv3_module = Ircv3Module::new();
    ircv3_module.init().await?;
    
    // Simulate a client ID
    let client_id = uuid::Uuid::new_v4();
    
    // Test Extended Join capability
    println!("Testing Extended Join capability...");
    ircv3_module.enable_extended_join(client_id);
    
    // Create a mock client
    let client = create_mock_client(client_id);
    
    // Test creating extended join message
    let extended_join_msg = ircv3_module.create_extended_join_message(
        &client,
        "#test",
        Some("alice"),
        Some("Alice User")
    )?;
    
    println!("Extended JOIN message: {}", extended_join_msg);
    
    // Test Multi-Prefix capability
    println!("\nTesting Multi-Prefix capability...");
    ircv3_module.enable_multi_prefix(client_id);
    
    // Create mock channel members with different modes
    let mut members = Vec::new();
    let mut modes1 = HashSet::new();
    modes1.insert('o'); // operator
    modes1.insert('v'); // voice
    members.push((uuid::Uuid::new_v4(), modes1));
    
    let mut modes2 = HashSet::new();
    modes2.insert('h'); // half-op
    members.push((uuid::Uuid::new_v4(), modes2));
    
    let mut modes3 = HashSet::new();
    modes3.insert('v'); // voice only
    members.push((uuid::Uuid::new_v4(), modes3));
    
    // Process members with multi-prefix
    let formatted_names = ircv3_module.process_channel_members(
        &client,
        &members,
        &|_| Some("testuser".to_string())
    );
    
    println!("Formatted names with multi-prefix: {:?}", formatted_names);
    
    // Test NAMES reply
    let names_reply = ircv3_module.create_names_reply(
        &client,
        "#test",
        &formatted_names
    )?;
    
    println!("NAMES reply: {}", names_reply);
    
    Ok(())
}

/// Create a mock client for testing
fn create_mock_client(id: uuid::Uuid) -> rustircd_core::Client {
    use rustircd_core::{Client, ConnectionType, ClientState, User};
    use tokio::sync::mpsc;
    
    let (sender, _receiver) = mpsc::unbounded_channel();
    
    let mut client = Client::new_with_type(
        id,
        "127.0.0.1:12345".to_string(),
        "127.0.0.1:6667".to_string(),
        sender,
        ConnectionType::Client,
    );
    
    client.set_state(ClientState::Registered);
    
    let user = User {
        id,
        nick: "testuser".to_string(),
        username: "testuser".to_string(),
        host: "localhost".to_string(),
        realname: "Test User".to_string(),
        channels: HashSet::new(),
        modes: HashSet::new(),
        away_message: None,
        last_activity: std::time::SystemTime::now(),
    };
    
    client.set_user(user);
    client
}
