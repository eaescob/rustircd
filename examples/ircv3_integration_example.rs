//! Example showing how to integrate IRCv3 Extended Join and Multi-Prefix
//! into the actual JOIN and NAMES command handlers

use rustircd_core::{Client, Message, Result, Error, Module};
use rustircd_modules::ircv3::Ircv3Module;
use std::collections::HashSet;

/// Example JOIN command handler with Extended Join support
pub async fn handle_join_with_extended_join(
    client: &Client,
    message: &Message,
    ircv3_module: &Ircv3Module,
) -> Result<()> {
    if !client.is_registered() {
        let error_msg = rustircd_core::NumericReply::not_registered();
        let _ = client.send(error_msg);
        return Ok(());
    }
    
    if message.params.is_empty() {
        let error_msg = rustircd_core::NumericReply::need_more_params("JOIN");
        let _ = client.send(error_msg);
        return Ok(());
    }
    
    let channel_name = &message.params[0];
    
    // Check if client has extended-join capability
    if ircv3_module.is_extended_join_enabled(&client.id).await {
        // Create extended JOIN message with account name and real name
        let account_name = ircv3_module.get_account_name(client).await;
        let real_name = ircv3_module.get_real_name(client).await;
        
        let join_message = ircv3_module.create_extended_join_message(
            client,
            channel_name,
            account_name.as_deref(),
            real_name.as_deref(),
        ).await?;
        
        // Send the extended JOIN message to channel members
        // (In a real implementation, you would broadcast this to all channel members)
        println!("Extended JOIN message: {}", join_message);
        
        // Example of what the message would look like:
        // :testuser!testuser@localhost JOIN #test alice :Alice User
        // vs standard JOIN:
        // :testuser!testuser@localhost JOIN #test
    } else {
        // Create standard JOIN message
        let join_message = ircv3_module.create_standard_join_message(client, channel_name).await?;
        
        // Send the standard JOIN message to channel members
        println!("Standard JOIN message: {}", join_message);
    }
    
    Ok(())
}

/// Example NAMES command handler with Multi-Prefix support
pub async fn handle_names_with_multi_prefix(
    client: &Client,
    message: &Message,
    ircv3_module: &Ircv3Module,
) -> Result<()> {
    if !client.is_registered() {
        let error_msg = rustircd_core::NumericReply::not_registered();
        let _ = client.send(error_msg);
        return Ok(());
    }
    
    // Get channels to show names for
    let channels = if message.params.is_empty() {
        // Show names for all channels user is in
        // In a real implementation, you would get this from the user's channel list
        vec!["#test".to_string()]
    } else {
        message.params.clone()
    };
    
    for channel_name in channels {
        // Simulate channel members with different modes
        let members = create_mock_channel_members();
        
        // Process members based on client's multi-prefix capability
        let formatted_names = ircv3_module.process_channel_members(
            client,
            &members,
            &|user_id| {
                // In a real implementation, you would look up the user by ID
                Some(format!("user{}", user_id.as_u128() % 1000))
            }
        ).await;
        
        // Create NAMES reply
        let names_reply = ircv3_module.create_names_reply(
            client,
            &channel_name,
            &formatted_names,
        ).await?;
        
        // Send NAMES reply to client
        println!("NAMES reply for {}: {}", channel_name, names_reply);
        
        // Create end of NAMES reply
        let end_reply = ircv3_module.create_end_of_names_reply(&channel_name).await?;
        println!("End of NAMES for {}: {}", channel_name, end_reply);
    }
    
    Ok(())
}

/// Create mock channel members with different modes for testing
fn create_mock_channel_members() -> Vec<(uuid::Uuid, HashSet<char>)> {
    let mut members = Vec::new();
    
    // User 1: Founder + Admin + Operator + Voice (should show ~&@+)
    let mut modes1 = HashSet::new();
    modes1.insert('q'); // founder
    modes1.insert('a'); // admin
    modes1.insert('o'); // operator
    modes1.insert('v'); // voice
    members.push((uuid::Uuid::new_v4(), modes1));
    
    // User 2: Admin + Half-op (should show &%)
    let mut modes2 = HashSet::new();
    modes2.insert('a'); // admin
    modes2.insert('h'); // half-op
    members.push((uuid::Uuid::new_v4(), modes2));
    
    // User 3: Operator + Voice (should show @+)
    let mut modes3 = HashSet::new();
    modes3.insert('o'); // operator
    modes3.insert('v'); // voice
    members.push((uuid::Uuid::new_v4(), modes3));
    
    // User 4: Half-op only (should show %)
    let mut modes4 = HashSet::new();
    modes4.insert('h'); // half-op
    members.push((uuid::Uuid::new_v4(), modes4));
    
    // User 5: Voice only (should show +)
    let mut modes5 = HashSet::new();
    modes5.insert('v'); // voice
    members.push((uuid::Uuid::new_v4(), modes5));
    
    // User 6: No modes (should show no prefix)
    let modes6 = HashSet::new();
    members.push((uuid::Uuid::new_v4(), modes6));
    
    members
}

/// Example of capability negotiation handling
pub async fn handle_capability_negotiation(
    client: &Client,
    message: &Message,
    ircv3_module: &mut Ircv3Module,
) -> Result<()> {
    if message.params.len() < 2 {
        return Err(Error::User("No capabilities specified".to_string()));
    }
    
    let requested_caps: Vec<&str> = message.params[1].split_whitespace().collect();
    let mut acked_caps = Vec::new();
    
    for cap in requested_caps {
        match cap {
            "extended-join" => {
                ircv3_module.enable_extended_join(client.id);
                acked_caps.push("extended-join");
            }
            "multi-prefix" => {
                ircv3_module.enable_multi_prefix(client.id);
                acked_caps.push("multi-prefix");
            }
            _ => {
                // Handle other capabilities or reject unsupported ones
            }
        }
    }
    
    if !acked_caps.is_empty() {
        let ack_msg = Message::new(
            rustircd_core::MessageType::Custom("CAP".to_string()),
            vec!["*".to_string(), "ACK".to_string(), acked_caps.join(" ")],
        );
        let _ = client.send(ack_msg);
        println!("ACK capabilities for client {}: {}", client.id, acked_caps.join(" "));
    }
    
    Ok(())
}

/// Demonstrate the difference between standard and extended capabilities
pub async fn demonstrate_capabilities() -> Result<()> {
    println!("=== IRCv3 Extended Join and Multi-Prefix Demonstration ===\n");
    
    // Create IRCv3 module
    let mut ircv3_module = Ircv3Module::new();
    ircv3_module.init().await?;
    
    // Create mock client
    let client = create_mock_client();
    
    println!("1. Standard JOIN (without extended-join capability):");
    let standard_join = ircv3_module.create_standard_join_message(&client, "#test").await?;
    println!("   {}", standard_join);
    println!();
    
    println!("2. Extended JOIN (with extended-join capability):");
    ircv3_module.enable_extended_join(client.id);
    let extended_join = ircv3_module.create_extended_join_message(
        &client,
        "#test",
        Some("alice"),
        Some("Alice User"),
    ).await?;
    println!("   {}", extended_join);
    println!();
    
    println!("3. Standard NAMES (without multi-prefix capability):");
    let members = create_mock_channel_members();
    let standard_names = ircv3_module.process_channel_members(
        &client,
        &members,
        &|user_id| Some(format!("user{}", user_id.as_u128() % 1000)),
    ).await;
    println!("   Names: {:?}", standard_names);
    println!();
    
    println!("4. Multi-Prefix NAMES (with multi-prefix capability):");
    ircv3_module.enable_multi_prefix(client.id);
    let multi_prefix_names = ircv3_module.process_channel_members(
        &client,
        &members,
        &|user_id| Some(format!("user{}", user_id.as_u128() % 1000)),
    ).await;
    println!("   Names: {:?}", multi_prefix_names);
    println!();
    
    println!("5. NAMES Reply with Multi-Prefix:");
    let names_reply = ircv3_module.create_names_reply(
        &client,
        "#test",
        &multi_prefix_names,
    ).await?;
    println!("   {}", names_reply);
    
    Ok(())
}

fn create_mock_client() -> Client {
    use rustircd_core::{Client, User};
    use rustircd_core::client::{ConnectionType, ClientState};
    use tokio::sync::mpsc;
    
    let client_id = uuid::Uuid::new_v4();
    let (sender, _receiver) = mpsc::unbounded_channel();
    
    let mut client = Client::new_with_type(
        client_id,
        "127.0.0.1:12345".to_string(),
        "127.0.0.1:6667".to_string(),
        sender,
        ConnectionType::Client,
    );
    
    client.set_state(ClientState::Registered);
    
    let user = User::new(
        "testuser".to_string(),
        "testuser".to_string(),
        "Test User".to_string(),
        "localhost".to_string(),
        "test.server".to_string(),
    );
    
    client.set_user(user);
    client
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    demonstrate_capabilities().await?;
    
    Ok(())
}
