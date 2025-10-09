//! Cache and Burst Integration Tests
//!
//! Tests to validate that cache operations work correctly with server bursting mechanisms.
//! Bursting is when servers synchronize state by sending batches of users, channels, and servers.

use rustircd_core::{Database, User};
use tokio;

/// Helper function to create a test user
fn create_test_user(nick: &str, username: &str, host: &str, server: &str) -> User {
    User::new(
        nick.to_string(),
        username.to_string(),
        "Test User".to_string(),
        host.to_string(),
        server.to_string(),
    )
}

#[tokio::test]
async fn test_user_burst_cache_population() {
    let db = Database::new(1000, 30);
    
    // Simulate a burst of users from a remote server
    let burst_users = vec![
        ("alice", "alice", "192.168.1.1", "server1.net"),
        ("bob", "bob", "192.168.1.2", "server1.net"),
        ("charlie", "charlie", "192.168.1.3", "server1.net"),
        ("dave", "dave", "192.168.1.4", "server1.net"),
        ("eve", "eve", "192.168.1.5", "server1.net"),
    ];
    
    // Add burst users to database (simulating user burst)
    let mut user_ids = Vec::new();
    for (nick, username, host, server) in burst_users {
        let user = create_test_user(nick, username, host, server);
        user_ids.push(user.id);
        db.add_user(user).unwrap();
    }
    
    // Check cache was populated during burst
    let cache_stats = db.get_user_cache_stats();
    assert_eq!(cache_stats.size, 5, "Cache should contain all burst users");
    
    // Verify all users can be looked up from cache
    for (nick, _, _, _) in &[
        ("alice", "alice", "192.168.1.1", "server1.net"),
        ("bob", "bob", "192.168.1.2", "server1.net"),
        ("charlie", "charlie", "192.168.1.3", "server1.net"),
        ("dave", "dave", "192.168.1.4", "server1.net"),
        ("eve", "eve", "192.168.1.5", "server1.net"),
    ] {
        let user = db.get_user_by_nick(nick);
        assert!(user.is_some(), "User {} should be in cache", nick);
    }
    
    // All lookups should have hit the cache
    let final_stats = db.get_user_cache_stats();
    assert!(final_stats.total_hits >= 5, "Should have at least 5 cache hits");
}

#[tokio::test]
async fn test_large_user_burst() {
    let db = Database::new(1000, 30);
    
    // Simulate a large burst (100 users)
    for i in 0..100 {
        let nick = format!("user{}", i);
        let username = format!("user{}", i);
        let user = create_test_user(&nick, &username, "192.168.1.1", "server1.net");
        db.add_user(user).unwrap();
    }
    
    let cache_stats = db.get_user_cache_stats();
    assert_eq!(cache_stats.size, 100, "Cache should contain all 100 burst users");
    
    // Verify random lookups are cached
    for i in [0, 25, 50, 75, 99] {
        let nick = format!("user{}", i);
        let user = db.get_user_by_nick(&nick);
        assert!(user.is_some(), "User {} should be found", nick);
    }
    
    let final_stats = db.get_user_cache_stats();
    assert!(final_stats.total_hits >= 5, "Should have cache hits for lookups");
}

#[tokio::test]
async fn test_channel_burst_cache_invalidation() {
    let db = Database::new(1000, 30);
    
    // Add users first
    for i in 0..10 {
        let nick = format!("user{}", i);
        let username = format!("user{}", i);
        let user = create_test_user(&nick, &username, "192.168.1.1", "server1.net");
        db.add_user(user).unwrap();
    }
    
    // Simulate channel burst - adding users to channels rapidly
    for i in 0..5 {
        let nick = format!("user{}", i);
        db.add_user_to_channel(&nick, "#channel1").unwrap();
    }
    
    for i in 5..10 {
        let nick = format!("user{}", i);
        db.add_user_to_channel(&nick, "#channel2").unwrap();
    }
    
    // Query channel members (should populate cache)
    let channel1_members = db.get_channel_users("#channel1");
    let channel2_members = db.get_channel_users("#channel2");
    
    assert_eq!(channel1_members.len(), 5);
    assert_eq!(channel2_members.len(), 5);
    
    // Verify cache was populated
    let channel_stats = db.get_channel_cache_stats();
    assert_eq!(channel_stats.size, 2, "Both channels should be cached");
    
    // Repeated queries should hit cache
    let _ = db.get_channel_users("#channel1");
    let _ = db.get_channel_users("#channel2");
    
    let final_stats = db.get_channel_cache_stats();
    assert!(final_stats.total_hits >= 2, "Should have cache hits on repeated queries");
}

#[tokio::test]
async fn test_burst_with_duplicate_users() {
    let db = Database::new(1000, 30);
    
    // Add initial users
    let user1 = create_test_user("alice", "alice", "192.168.1.1", "server1.net");
    let user1_id = user1.id;
    db.add_user(user1).unwrap();
    
    // Try to add duplicate (simulating burst with duplicate)
    let user1_dup = create_test_user("alice", "alice", "192.168.1.1", "server1.net");
    let result = db.add_user(user1_dup);
    assert!(result.is_err(), "Duplicate user should fail");
    
    // Original user should still be in cache
    let user = db.get_user_by_nick("alice").unwrap();
    assert_eq!(user.id, user1_id);
    
    let cache_stats = db.get_user_cache_stats();
    assert!(cache_stats.total_hits > 0, "Cache should still work after duplicate attempt");
}

#[tokio::test]
async fn test_multiple_server_burst() {
    let db = Database::new(1000, 30);
    
    // Simulate burst from server1
    for i in 0..10 {
        let nick = format!("s1user{}", i);
        let user = create_test_user(&nick, &nick, "192.168.1.1", "server1.net");
        db.add_user(user).unwrap();
    }
    
    // Simulate burst from server2
    for i in 0..10 {
        let nick = format!("s2user{}", i);
        let user = create_test_user(&nick, &nick, "192.168.2.1", "server2.net");
        db.add_user(user).unwrap();
    }
    
    // Simulate burst from server3
    for i in 0..10 {
        let nick = format!("s3user{}", i);
        let user = create_test_user(&nick, &nick, "192.168.3.1", "server3.net");
        db.add_user(user).unwrap();
    }
    
    let cache_stats = db.get_user_cache_stats();
    assert_eq!(cache_stats.size, 30, "Cache should contain all users from all servers");
    
    // Verify we can look up users from any server
    assert!(db.get_user_by_nick("s1user5").is_some());
    assert!(db.get_user_by_nick("s2user5").is_some());
    assert!(db.get_user_by_nick("s3user5").is_some());
    
    let final_stats = db.get_user_cache_stats();
    assert!(final_stats.total_hits >= 3, "Should have cache hits");
}

#[tokio::test]
async fn test_burst_cache_performance_under_load() {
    use std::time::Instant;
    
    let db = Database::new(10000, 30);
    
    // Simulate a large burst (1000 users)
    let start = Instant::now();
    for i in 0..1000 {
        let nick = format!("user{}", i);
        let user = create_test_user(&nick, &nick, "192.168.1.1", "server1.net");
        db.add_user(user).unwrap();
    }
    let burst_duration = start.elapsed();
    
    println!("Burst insertion time for 1000 users: {:?}", burst_duration);
    
    // Now test lookup performance
    let start = Instant::now();
    for i in 0..1000 {
        let nick = format!("user{}", i);
        let _ = db.get_user_by_nick(&nick);
    }
    let lookup_duration = start.elapsed();
    
    println!("Lookup time for 1000 users (cached): {:?}", lookup_duration);
    println!("Average lookup time: {:?}", lookup_duration / 1000);
    
    // Cache should make lookups significantly faster
    let cache_stats = db.get_user_cache_stats();
    assert_eq!(cache_stats.size, 1000, "All users should be cached");
    assert_eq!(cache_stats.total_hits, 1000, "All lookups should hit cache");
}

#[tokio::test]
async fn test_channel_burst_with_many_members() {
    let db = Database::new(1000, 30);
    
    // Add 100 users
    for i in 0..100 {
        let nick = format!("user{}", i);
        let user = create_test_user(&nick, &nick, "192.168.1.1", "server1.net");
        db.add_user(user).unwrap();
    }
    
    // Simulate channel burst - add all 100 users to a channel
    for i in 0..100 {
        let nick = format!("user{}", i);
        db.add_user_to_channel(&nick, "#bigchannel").unwrap();
    }
    
    // Query channel members
    let members = db.get_channel_users("#bigchannel");
    assert_eq!(members.len(), 100, "All users should be in channel");
    
    // Repeated queries should be fast (cached)
    use std::time::Instant;
    let start = Instant::now();
    for _ in 0..100 {
        let _ = db.get_channel_users("#bigchannel");
    }
    let duration = start.elapsed();
    
    println!("100 channel member queries (cached): {:?}", duration);
    println!("Average query time: {:?}", duration / 100);
    
    let channel_stats = db.get_channel_cache_stats();
    assert!(channel_stats.total_hits >= 100, "Should have high cache hit count");
}

#[tokio::test]
async fn test_burst_then_user_removal() {
    let db = Database::new(1000, 30);
    
    // Simulate burst
    let mut user_ids = Vec::new();
    for i in 0..20 {
        let nick = format!("user{}", i);
        let user = create_test_user(&nick, &nick, "192.168.1.1", "server1.net");
        user_ids.push((user.id, nick.clone()));
        db.add_user(user).unwrap();
    }
    
    // Add users to channels
    for i in 0..20 {
        let nick = format!("user{}", i);
        db.add_user_to_channel(&nick, "#test").unwrap();
    }
    
    // Populate cache
    let _ = db.get_channel_users("#test");
    let cache_stats_before = db.get_channel_cache_stats();
    assert!(cache_stats_before.size > 0);
    
    // Remove some users (simulating QUIT)
    for (user_id, nick) in user_ids.iter().take(5) {
        db.remove_user(*user_id).unwrap();
        // Cache should be invalidated
        let members = db.get_channel_users("#test");
        assert!(!members.contains(nick), "Removed user should not be in channel");
    }
    
    // Verify final state
    let members = db.get_channel_users("#test");
    assert_eq!(members.len(), 15, "Should have 15 users remaining");
}

#[tokio::test]
async fn test_concurrent_burst_and_queries() {
    use std::sync::Arc;
    use tokio::task;
    
    let db = Arc::new(Database::new(10000, 30));
    
    // Spawn task to add users (simulating burst)
    let db_write = Arc::clone(&db);
    let writer = task::spawn(async move {
        for i in 0..100 {
            let nick = format!("user{}", i);
            let user = create_test_user(&nick, &nick, "192.168.1.1", "server1.net");
            let _ = db_write.add_user(user);
            tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
        }
    });
    
    // Spawn tasks to query users concurrently
    let mut readers = vec![];
    for _ in 0..5 {
        let db_read = Arc::clone(&db);
        let reader = task::spawn(async move {
            for i in 0..100 {
                let nick = format!("user{}", i);
                let _ = db_read.get_user_by_nick(&nick);
                tokio::time::sleep(tokio::time::Duration::from_micros(50)).await;
            }
        });
        readers.push(reader);
    }
    
    // Wait for all tasks
    writer.await.unwrap();
    for reader in readers {
        reader.await.unwrap();
    }
    
    // Verify cache worked correctly under concurrent access
    let cache_stats = db.get_user_cache_stats();
    assert!(cache_stats.size > 0, "Cache should have entries");
    assert!(cache_stats.total_hits > 0, "Cache should have hits");
}

#[tokio::test]
async fn test_burst_cache_lru_eviction() {
    let db = Database::new(1000, 30);
    
    // Fill cache to capacity (10000 entries)
    for i in 0..10100 {
        let nick = format!("user{:05}", i);
        let user = create_test_user(&nick, &nick, "192.168.1.1", "server1.net");
        let _ = db.add_user(user);
    }
    
    let cache_stats = db.get_user_cache_stats();
    assert!(cache_stats.size <= 10000, "Cache should not exceed capacity");
    
    // Oldest entries should have been evicted
    // Most recent entries should still be accessible
    let recent_user = db.get_user_by_nick("user10099");
    assert!(recent_user.is_some(), "Recent user should be in cache");
}

#[tokio::test]
async fn test_channel_burst_multiple_channels() {
    let db = Database::new(1000, 30);
    
    // Add users
    for i in 0..50 {
        let nick = format!("user{}", i);
        let user = create_test_user(&nick, &nick, "192.168.1.1", "server1.net");
        db.add_user(user).unwrap();
    }
    
    // Simulate burst of multiple channels
    let channels = ["#channel1", "#channel2", "#channel3", "#channel4", "#channel5"];
    
    for (idx, channel) in channels.iter().enumerate() {
        // Add 10 users to each channel
        for i in (idx * 10)..((idx + 1) * 10) {
            let nick = format!("user{}", i);
            db.add_user_to_channel(&nick, channel).unwrap();
        }
    }
    
    // Query all channels (populate cache)
    for channel in &channels {
        let members = db.get_channel_users(channel);
        assert_eq!(members.len(), 10, "Each channel should have 10 members");
    }
    
    let channel_stats = db.get_channel_cache_stats();
    assert_eq!(channel_stats.size, 5, "All 5 channels should be cached");
    
    // Repeated queries should hit cache
    for channel in &channels {
        let _ = db.get_channel_users(channel);
    }
    
    let final_stats = db.get_channel_cache_stats();
    assert!(final_stats.total_hits >= 5, "Should have cache hits on repeated queries");
}

#[tokio::test]
async fn test_burst_with_cache_statistics() {
    let db = Database::new(1000, 30);
    
    // Simulate burst
    for i in 0..50 {
        let nick = format!("user{}", i);
        let user = create_test_user(&nick, &nick, "192.168.1.1", "server1.net");
        db.add_user(user).unwrap();
    }
    
    // Initial cache state
    let initial_stats = db.get_user_cache_stats();
    assert_eq!(initial_stats.size, 50);
    let initial_hits = initial_stats.total_hits;
    
    // Perform lookups
    for i in 0..50 {
        let nick = format!("user{}", i);
        let _ = db.get_user_by_nick(&nick);
    }
    
    // Verify statistics updated
    let final_stats = db.get_user_cache_stats();
    assert_eq!(final_stats.size, 50, "Cache size should remain stable");
    assert!(final_stats.total_hits > initial_hits, "Hit count should increase");
    assert!(final_stats.total_hits >= 50, "Should have at least 50 hits");
}

