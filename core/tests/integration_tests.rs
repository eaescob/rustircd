//! Integration tests for RustIRCd core functionality

use rustircd_core::*;
use std::sync::Arc;
use tokio;
use uuid::Uuid;

#[tokio::test]
async fn test_database_operations() {
    let db = Database::new(1000, 30);
    
    // Test user creation
    let user = User::new(
        "alice".to_string(),
        "alice_user".to_string(),
        "Alice Wonderland".to_string(),
        "host.example.com".to_string(),
        "server.example.com".to_string(),
    );
    
    let user_id = user.id;
    assert!(db.add_user(user.clone()).is_ok());
    
    // Test user retrieval
    let retrieved = db.get_user(&user_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().nick, "alice");
    
    // Test user by nickname lookup
    let by_nick = db.get_user_by_nick("alice");
    assert!(by_nick.is_some());
    assert_eq!(by_nick.unwrap().id, user_id);
    
    // Test duplicate nickname
    let duplicate_user = User::new(
        "alice".to_string(),
        "other_user".to_string(),
        "Other User".to_string(),
        "other.example.com".to_string(),
        "server.example.com".to_string(),
    );
    assert!(db.add_user(duplicate_user).is_err());
    
    // Test user update
    let mut updated_user = user.clone();
    updated_user.nick = "alice2".to_string();
    assert!(db.update_user(updated_user.clone()).is_ok());
    
    let by_new_nick = db.get_user_by_nick("alice2");
    assert!(by_new_nick.is_some());
    
    // Test user removal
    assert!(db.remove_user(&user_id).is_ok());
    assert!(db.get_user(&user_id).is_none());
}

#[tokio::test]
async fn test_server_info_management() {
    let db = Database::new(1000, 30);
    
    let server_info = database::ServerInfo {
        name: "test.server".to_string(),
        description: "Test Server".to_string(),
        version: "1.0.0".to_string(),
        hopcount: 1,
        connected_at: chrono::Utc::now(),
        is_super_server: false,
        user_count: 0,
    };
    
    assert!(db.add_server(server_info.clone()).is_ok());
    
    let retrieved = db.get_server("test.server");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().description, "Test Server");
    
    assert!(db.remove_server("test.server").is_ok());
    assert!(db.get_server("test.server").is_none());
}

#[tokio::test]
async fn test_channel_operations() {
    let db = Database::new(1000, 30);
    
    // Create a channel
    let channel_info = ChannelInfo {
        name: "#test".to_string(),
        topic: Some("Test Topic".to_string()),
        user_count: 0,
        modes: std::collections::HashSet::new(),
    };
    
    assert!(db.add_channel(channel_info.clone()).is_ok());
    
    let retrieved = db.get_channel("#test");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().topic, Some("Test Topic".to_string()));
    
    // Add user to channel
    assert!(db.add_user_to_channel("alice", "#test").is_ok());
    
    let user_channels = db.get_user_channels("alice");
    assert!(user_channels.contains("#test"));
    
    let channel_members = db.get_channel_members("#test");
    assert!(channel_members.contains("alice"));
    
    // Remove user from channel
    assert!(db.remove_user_from_channel("alice", "#test").is_ok());
    assert!(!db.get_user_channels("alice").contains("#test"));
}

#[tokio::test]
async fn test_message_parsing() {
    // Test simple command
    let msg = Message::parse("NICK alice").unwrap();
    assert_eq!(msg.command, MessageType::Nick);
    assert_eq!(msg.params.len(), 1);
    assert_eq!(msg.params[0], "alice");
    
    // Test command with prefix
    let msg = Message::parse(":alice!user@host PRIVMSG #channel :Hello world").unwrap();
    assert!(msg.prefix.is_some());
    match msg.prefix.unwrap() {
        Prefix::User { nick, user, host } => {
            assert_eq!(nick, "alice");
            assert_eq!(user, "user");
            assert_eq!(host, "host");
        }
        _ => panic!("Expected user prefix"),
    }
    assert_eq!(msg.command, MessageType::PrivMsg);
    assert_eq!(msg.params.len(), 2);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "Hello world");
    
    // Test server prefix
    let msg = Message::parse(":server.example.com PING :server").unwrap();
    assert!(msg.prefix.is_some());
    match msg.prefix.unwrap() {
        Prefix::Server(server) => assert_eq!(server, "server.example.com"),
        _ => panic!("Expected server prefix"),
    }
    
    // Test numeric reply
    let msg = Message::parse(":server 001 alice :Welcome").unwrap();
    assert_eq!(msg.command, MessageType::Custom("001".to_string()));
}

#[tokio::test]
async fn test_message_serialization() {
    // Test simple message
    let msg = Message::new(MessageType::Nick, vec!["alice".to_string()]);
    assert_eq!(msg.to_string().trim(), "NICK alice");
    
    // Test message with prefix
    let msg = Message::with_prefix(
        Prefix::User {
            nick: "alice".to_string(),
            user: "user".to_string(),
            host: "host".to_string(),
        },
        MessageType::PrivMsg,
        vec!["#channel".to_string(), "Hello world".to_string()],
    );
    let serialized = msg.to_string().trim();
    assert!(serialized.starts_with(":alice!user@host PRIVMSG #channel"));
    assert!(serialized.contains("Hello world"));
    
    // Test server prefix
    let msg = Message::with_prefix(
        Prefix::Server("server.example.com".to_string()),
        MessageType::Ping,
        vec!["test".to_string()],
    );
    assert_eq!(msg.to_string().trim(), ":server.example.com PING test");
}

#[tokio::test]
async fn test_user_modes() {
    let mut user = User::new(
        "alice".to_string(),
        "user".to_string(),
        "Alice User".to_string(),
        "host.example.com".to_string(),
        "server.example.com".to_string(),
    );
    
    // Test mode setting
    user.set_mode(UserMode::Invisible, true);
    assert!(user.has_mode(&UserMode::Invisible));
    
    user.set_mode(UserMode::Wallops, true);
    assert!(user.has_mode(&UserMode::Wallops));
    
    // Test mode unsetting
    user.set_mode(UserMode::Invisible, false);
    assert!(!user.has_mode(&UserMode::Invisible));
    
    // Test operator mode
    assert!(!user.is_operator);
    user.set_mode(UserMode::Operator, true);
    assert!(user.has_mode(&UserMode::Operator));
}

#[tokio::test]
async fn test_broadcast_system() {
    let system = BroadcastSystem::new();
    
    // Register clients
    let client1_id = Uuid::new_v4();
    let client2_id = Uuid::new_v4();
    
    // Subscribe to channel
    system.subscribe_to_channel(client1_id, "#test".to_string());
    system.subscribe_to_channel(client2_id, "#test".to_string());
    
    // Test channel subscription
    let target = BroadcastTarget::Channel("#test".to_string());
    
    // Unsubscribe
    system.unsubscribe_from_channel(&client1_id, "#test");
    
    // Cleanup
    system.unregister_client(&client1_id);
    system.unregister_client(&client2_id);
}

#[tokio::test]
async fn test_cache_operations() {
    use std::time::Duration;
    
    // Test LRU cache
    let cache = LruCache::<String, String>::new(2, Duration::from_secs(60));
    
    cache.insert("key1".to_string(), "value1".to_string());
    cache.insert("key2".to_string(), "value2".to_string());
    
    assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    
    // This should evict key2 (not accessed recently)
    cache.insert("key3".to_string(), "value3".to_string());
    
    // Test message cache
    let msg_cache = MessageCache::new(100, Duration::from_secs(60));
    msg_cache.insert("PING :test".to_string(), "PONG :test\r\n".to_string());
    assert_eq!(msg_cache.get("PING :test"), Some("PONG :test\r\n".to_string()));
    
    // Test DNS cache
    let dns_cache = DnsCache::new(Duration::from_secs(300));
    dns_cache.cache_hostname("192.168.1.1".to_string(), "example.com".to_string());
    assert_eq!(dns_cache.get_hostname("192.168.1.1"), Some("example.com".to_string()));
    assert_eq!(dns_cache.get_ip("example.com"), Some("192.168.1.1".to_string()));
}

#[tokio::test]
async fn test_batch_optimizer() {
    use std::time::Duration;
    
    let config = BatchConfig {
        max_batch_size: 3,
        max_batch_delay: Duration::from_millis(100),
        max_batch_bytes: 1000,
    };
    
    let optimizer = BatchOptimizer::new(config);
    let target_id = Uuid::new_v4();
    
    // Add messages
    let msg1 = Message::new(MessageType::PrivMsg, vec!["#test".to_string(), "Hello".to_string()]);
    let msg2 = Message::new(MessageType::PrivMsg, vec!["#test".to_string(), "World".to_string()]);
    let msg3 = Message::new(MessageType::PrivMsg, vec!["#test".to_string(), "!".to_string()]);
    
    optimizer.add_message(target_id, msg1).await.unwrap();
    optimizer.add_message(target_id, msg2).await.unwrap();
    optimizer.add_message(target_id, msg3).await.unwrap();
    
    // Should have a ready batch now
    let ready = optimizer.get_ready_batches().await;
    assert_eq!(ready.len(), 1);
    
    let stats = optimizer.stats().await;
    assert_eq!(stats.total_messages_batched, 3);
    assert_eq!(stats.total_batches_sent, 1);
}

#[tokio::test]
async fn test_connection_pool() {
    let pool = ConnectionPool::new(5);
    let server = "test.server".to_string();
    let conn_id = Uuid::new_v4();
    
    // Add connection
    pool.add_connection(server.clone(), conn_id).await.unwrap();
    
    // Get connection
    let retrieved = pool.get_connection(&server).await;
    assert_eq!(retrieved, Some(conn_id));
    
    // Check stats
    let stats = pool.stats().await;
    assert_eq!(stats.active_connections, 1);
    assert_eq!(stats.connections_created, 1);
    
    // Remove connection
    pool.remove_connection(&server, &conn_id).await;
    let stats = pool.stats().await;
    assert_eq!(stats.active_connections, 0);
}

#[tokio::test]
async fn test_numeric_replies() {
    let welcome = NumericReply::welcome("server", "alice", "user", "host");
    assert_eq!(welcome.command, MessageType::Custom("001".to_string()));
    assert_eq!(welcome.params[0], "alice");
    
    let no_nick = NumericReply::no_nickname_given();
    assert_eq!(no_nick.command, MessageType::Custom("431".to_string()));
    
    let nick_in_use = NumericReply::nickname_in_use("alice");
    assert_eq!(nick_in_use.command, MessageType::Custom("433".to_string()));
}

#[tokio::test]
async fn test_validation_functions() {
    use crate::utils::string;
    
    // Channel name validation
    assert!(string::is_valid_channel_name("#channel"));
    assert!(string::is_valid_channel_name("&channel"));
    assert!(string::is_valid_channel_name("+channel"));
    assert!(string::is_valid_channel_name("!channel"));
    assert!(!string::is_valid_channel_name("channel"));
    assert!(!string::is_valid_channel_name(""));
    
    // Nickname validation
    assert!(string::is_valid_nickname("alice", 9));
    assert!(string::is_valid_nickname("alice123", 9));
    assert!(string::is_valid_nickname("alice_", 9));
    assert!(!string::is_valid_nickname("", 9));
    assert!(!string::is_valid_nickname("123alice", 9));
    assert!(!string::is_valid_nickname("alice#", 9));
    
    // Username validation
    assert!(string::is_valid_username("user"));
    assert!(string::is_valid_username("user123"));
    assert!(!string::is_valid_username(""));
    assert!(!string::is_valid_username("user name"));
}

#[tokio::test]
async fn test_throttling_manager() {
    let throttling = ThrottlingManager::new(3, 60, 10);
    let ip = "192.168.1.1".to_string();
    
    // First connections should be allowed
    assert!(throttling.check_connection(&ip).is_ok());
    assert!(throttling.check_connection(&ip).is_ok());
    assert!(throttling.check_connection(&ip).is_ok());
    
    // Fourth connection should be throttled
    assert!(throttling.check_connection(&ip).is_err());
}

#[tokio::test]
async fn test_class_tracker() {
    use crate::config::ConnectionClass;
    
    let class = ConnectionClass {
        name: "default".to_string(),
        max_sendq: 1048576,
        max_recvq: 8192,
        ping_frequency: 120,
        connection_timeout: 300,
        max_clients: Some(100),
        max_connections_per_ip: Some(3),
        max_connections_per_host: Some(5),
        throttle: true,
    };
    
    let tracker = ClassTracker::new();
    tracker.register_class(class);
    
    // Test connection tracking
    let ip = "192.168.1.1".to_string();
    let host = "host.example.com".to_string();
    
    assert!(tracker.can_accept_connection("default", &ip, &host).is_ok());
    
    let client_id = Uuid::new_v4();
    tracker.track_connection(client_id, "default".to_string(), ip.clone(), host.clone());
    
    // Get stats
    let stats = tracker.get_stats("default");
    assert!(stats.is_some());
}

#[test]
fn test_user_creation() {
    let user = User::new(
        "alice".to_string(),
        "user".to_string(),
        "Alice User".to_string(),
        "host.example.com".to_string(),
        "server.example.com".to_string(),
    );
    
    assert_eq!(user.nick, "alice");
    assert_eq!(user.username, "user");
    assert_eq!(user.realname, "Alice User");
    assert_eq!(user.host, "host.example.com");
    assert_eq!(user.server, "server.example.com");
    assert!(!user.registered);
    assert!(!user.is_operator);
}






