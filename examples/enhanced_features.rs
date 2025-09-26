//! Enhanced IRC Daemon Features Example
//! 
//! This example demonstrates the new database, broadcasting, and network features
//! of the Rust IRC daemon.

use rustircd_core::{
    Config, Server, Database, BroadcastSystem, NetworkQueryManager,
    BroadcastTarget, BroadcastMessage, BroadcastPriority, MessageBuilder,
    User, Message, MessageType,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load enhanced configuration
    let config = load_enhanced_config().await?;
    
    // Create server with enhanced features
    let mut server = Server::new(config);
    server.init().await?;

    // Demonstrate database features
    demonstrate_database_features(&server).await?;
    
    // Demonstrate broadcasting features
    demonstrate_broadcasting_features(&server).await?;
    
    // Demonstrate network query features
    demonstrate_network_features(&server).await?;
    
    // Start the server
    println!("Starting enhanced IRC daemon...");
    server.start().await?;
    
    Ok(())
}

async fn load_enhanced_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Load configuration from file
    let config_str = std::fs::read_to_string("config_enhanced.toml")?;
    let config: Config = toml::from_str(&config_str)?;
    
    // Validate configuration
    config.validate()?;
    
    Ok(config)
}

async fn demonstrate_database_features(server: &Server) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Database Features Demo ===");
    
    // Create some test users
    let user1 = create_test_user("alice", "alice@example.com", "Alice Smith");
    let user2 = create_test_user("bob", "bob@example.com", "Bob Johnson");
    let user3 = create_test_user("charlie", "charlie@example.com", "Charlie Brown");
    
    // Add users to database
    server.database.add_user(user1.clone())?;
    server.database.add_user(user2.clone())?;
    server.database.add_user(user3.clone())?;
    
    println!("Added 3 users to database");
    println!("Total users: {}", server.database.user_count());
    
    // Demonstrate user search
    let search_results = server.database.search_users("al*");
    println!("Users matching 'al*': {:?}", search_results.iter().map(|u| &u.nick).collect::<Vec<_>>());
    
    // Demonstrate channel tracking
    server.database.add_user_to_channel("alice", "#general")?;
    server.database.add_user_to_channel("bob", "#general")?;
    server.database.add_user_to_channel("charlie", "#dev")?;
    
    let general_users = server.database.get_channel_users("#general");
    println!("Users in #general: {:?}", general_users);
    
    let alice_channels = server.database.get_user_channels("alice");
    println!("Alice's channels: {:?}", alice_channels);
    
    // Simulate user disconnection and history tracking
    server.database.remove_user(user1.id).await?;
    println!("Removed Alice from active users");
    
    let alice_history = server.database.get_user_history("alice").await;
    println!("Alice's history entries: {}", alice_history.len());
    
    Ok(())
}

async fn demonstrate_broadcasting_features(server: &Server) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Broadcasting Features Demo ===");
    
    // Create a test message
    let message = Message::new(
        MessageType::PrivMsg,
        vec!["#general".to_string(), "Hello everyone!".to_string()],
    );
    
    // Demonstrate different broadcast targets
    let broadcast_all = BroadcastMessage {
        message: message.clone(),
        target: BroadcastTarget::AllUsers,
        sender: None,
        priority: BroadcastPriority::Normal,
    };
    
    let broadcast_channel = BroadcastMessage {
        message: message.clone(),
        target: BroadcastTarget::Channel("#general".to_string()),
        sender: None,
        priority: BroadcastPriority::Normal,
    };
    
    let broadcast_operators = BroadcastMessage {
        message: message.clone(),
        target: BroadcastTarget::Operators,
        sender: None,
        priority: BroadcastPriority::High,
    };
    
    // Queue messages
    server.broadcast_system.queue_message(broadcast_all)?;
    server.broadcast_system.queue_message(broadcast_channel)?;
    server.broadcast_system.queue_message(broadcast_operators)?;
    
    println!("Queued 3 broadcast messages");
    println!("Queue sizes: {:?}", server.broadcast_system.get_queue_sizes());
    
    // Process queues
    server.broadcast_system.process_queues().await?;
    
    // Get statistics
    let stats = server.broadcast_system.get_stats().await;
    println!("Broadcast stats: {:?}", stats);
    
    Ok(())
}

async fn demonstrate_network_features(server: &Server) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Network Features Demo ===");
    
    // Add some test servers
    let server1 = rustircd_core::ServerInfo {
        name: "server1.example.com".to_string(),
        description: "Test Server 1".to_string(),
        version: "1.0.0".to_string(),
        hopcount: 1,
        connected_at: chrono::Utc::now(),
        is_super_server: false,
        user_count: 50,
    };
    
    let server2 = rustircd_core::ServerInfo {
        name: "server2.example.com".to_string(),
        description: "Test Server 2".to_string(),
        version: "1.0.0".to_string(),
        hopcount: 1,
        connected_at: chrono::Utc::now(),
        is_super_server: true,
        user_count: 100,
    };
    
    server.database.add_server(server1)?;
    server.database.add_server(server2)?;
    
    println!("Added 2 servers to database");
    println!("Total servers: {}", server.database.server_count());
    
    // Demonstrate network queries
    let test_client_id = Uuid::new_v4();
    let servers = server.database.get_all_servers();
    let server_names: Vec<String> = servers.iter().map(|s| s.name.clone()).collect();
    
    // Submit WHOIS query
    if let Ok(request_id) = server.network_query_manager.query_whois(
        "alice".to_string(),
        test_client_id,
        server_names.clone(),
    ).await {
        println!("Submitted WHOIS query for 'alice': {}", request_id);
    }
    
    // Submit WHO query
    if let Ok(request_id) = server.network_query_manager.query_who(
        "*".to_string(),
        test_client_id,
        server_names,
    ).await {
        println!("Submitted WHO query for '*': {}", request_id);
    }
    
    println!("Pending queries: {}", server.network_query_manager.pending_query_count().await);
    
    // Clean up expired queries
    server.network_query_manager.cleanup_expired_queries().await?;
    
    Ok(())
}

fn create_test_user(nick: &str, ident: &str, realname: &str) -> User {
    let (username, host) = ident.split_once('@').unwrap_or((ident, "localhost"));
    
    User {
        id: Uuid::new_v4(),
        nick: nick.to_string(),
        username: username.to_string(),
        host: host.to_string(),
        realname: realname.to_string(),
        server: "localhost".to_string(),
        signon_time: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
        modes: std::collections::HashSet::new(),
        is_operator: false,
        away_message: None,
    }
}

// Helper function to demonstrate message building
fn demonstrate_message_building() {
    println!("\n=== Message Building Demo ===");
    
    let user = create_test_user("alice", "alice@example.com", "Alice Smith");
    
    // Create different types of messages
    let privmsg = MessageBuilder::privmsg("#general", "Hello world!", &user);
    let notice = MessageBuilder::notice("#general", "This is a notice", &user);
    let join = MessageBuilder::join("#general", &user);
    let part = MessageBuilder::part("#general", Some("Leaving"), &user);
    let quit = MessageBuilder::quit(Some("Goodbye"), &user);
    
    println!("PRIVMSG: {:?}", privmsg);
    println!("NOTICE: {:?}", notice);
    println!("JOIN: {:?}", join);
    println!("PART: {:?}", part);
    println!("QUIT: {:?}", quit);
}
