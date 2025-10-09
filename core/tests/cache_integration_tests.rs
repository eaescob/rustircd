//! Cache integration tests
//!
//! Tests to verify cache behavior in the Database for user lookups and channel members

use rustircd_core::{Database, User};
use tokio;

/// Helper function to create a test user
fn create_test_user(nick: &str, username: &str, host: &str) -> User {
    User::new(
        nick.to_string(),
        username.to_string(),
        "Test User".to_string(),
        host.to_string(),
        "test.server".to_string(),
    )
}

#[tokio::test]
async fn test_user_lookup_cache_basic() {
    let db = Database::new(1000, 30);
    
    // Add a user
    let user = create_test_user("alice", "alice", "192.168.1.1");
    let user_id = user.id;
    db.add_user(user).unwrap();
    
    // First lookup - should populate cache
    let stats_before = db.get_user_cache_stats();
    assert_eq!(stats_before.size, 1); // Entry added during add_user
    
    // Second lookup - should hit cache
    let cached_user = db.get_user_by_nick("alice").unwrap();
    assert_eq!(cached_user.id, user_id);
    
    // Check cache stats increased
    let stats_after = db.get_user_cache_stats();
    assert!(stats_after.total_hits > stats_before.total_hits);
}

#[tokio::test]
async fn test_user_lookup_cache_case_insensitive() {
    let db = Database::new(1000, 30);
    
    // Add a user with mixed case nickname
    let user = create_test_user("AlIcE", "alice", "192.168.1.1");
    let user_id = user.id;
    db.add_user(user).unwrap();
    
    // Lookup with different case variations
    let user1 = db.get_user_by_nick("alice").unwrap();
    let user2 = db.get_user_by_nick("ALICE").unwrap();
    let user3 = db.get_user_by_nick("AlIcE").unwrap();
    
    // All should return the same user
    assert_eq!(user1.id, user_id);
    assert_eq!(user2.id, user_id);
    assert_eq!(user3.id, user_id);
    
    // All should hit the cache (after first one populates it)
    let stats = db.get_user_cache_stats();
    assert!(stats.total_hits >= 2); // At least 2 cache hits
}

#[tokio::test]
async fn test_user_lookup_cache_invalidation_on_remove() {
    let db = Database::new(1000, 30);
    
    // Add a user
    let user = create_test_user("bob", "bob", "192.168.1.2");
    db.add_user(user).unwrap();
    
    // Lookup to populate cache
    let user1 = db.get_user_by_nick("bob").unwrap();
    assert_eq!(user1.nick, "bob");
    
    let cache_size_before = db.get_user_cache_stats().size;
    
    // Remove the user
    let removed_user = db.remove_user(user1.id).unwrap();
    assert!(removed_user.is_some());
    
    // Cache should be invalidated
    let cache_size_after = db.get_user_cache_stats().size;
    assert!(cache_size_after < cache_size_before);
    
    // Lookup should return None
    let user2 = db.get_user_by_nick("bob");
    assert!(user2.is_none());
}

#[tokio::test]
async fn test_user_lookup_cache_update_nickname() {
    let db = Database::new(1000, 30);
    
    // Add a user
    let mut user = create_test_user("charlie", "charlie", "192.168.1.3");
    let user_id = user.id;
    db.add_user(user.clone()).unwrap();
    
    // Lookup with old nickname
    let user1 = db.get_user_by_nick("charlie").unwrap();
    assert_eq!(user1.nick, "charlie");
    
    // Update nickname
    user.nick = "charles".to_string();
    db.update_user(&user_id, user).unwrap();
    
    // Old nickname should not be in cache
    let user_old = db.get_user_by_nick("charlie");
    assert!(user_old.is_none());
    
    // New nickname should be in cache
    let user_new = db.get_user_by_nick("charles").unwrap();
    assert_eq!(user_new.nick, "charles");
    assert_eq!(user_new.id, user_id);
}

#[tokio::test]
async fn test_channel_member_cache_basic() {
    let db = Database::new(1000, 30);
    
    // Add users
    let user1 = create_test_user("alice", "alice", "192.168.1.1");
    let user2 = create_test_user("bob", "bob", "192.168.1.2");
    db.add_user(user1).unwrap();
    db.add_user(user2).unwrap();
    
    // Add users to channel
    db.add_user_to_channel("alice", "#test").unwrap();
    db.add_user_to_channel("bob", "#test").unwrap();
    
    // First lookup - should populate cache
    let members1 = db.get_channel_users("#test");
    assert_eq!(members1.len(), 2);
    assert!(members1.contains(&"alice".to_string()));
    assert!(members1.contains(&"bob".to_string()));
    
    // Second lookup - should hit cache
    let cache_stats_before = db.get_channel_cache_stats();
    let members2 = db.get_channel_users("#test");
    let cache_stats_after = db.get_channel_cache_stats();
    
    assert_eq!(members2.len(), 2);
    assert!(cache_stats_after.total_hits > cache_stats_before.total_hits);
}

#[tokio::test]
async fn test_channel_member_cache_invalidation_on_join() {
    let db = Database::new(1000, 30);
    
    // Add users
    let user1 = create_test_user("alice", "alice", "192.168.1.1");
    let user2 = create_test_user("bob", "bob", "192.168.1.2");
    db.add_user(user1).unwrap();
    db.add_user(user2).unwrap();
    
    // Add first user to channel
    db.add_user_to_channel("alice", "#test").unwrap();
    
    // Lookup to populate cache
    let members1 = db.get_channel_users("#test");
    assert_eq!(members1.len(), 1);
    
    // Add second user - should invalidate cache
    db.add_user_to_channel("bob", "#test").unwrap();
    
    // Lookup should return updated list
    let members2 = db.get_channel_users("#test");
    assert_eq!(members2.len(), 2);
    assert!(members2.contains(&"alice".to_string()));
    assert!(members2.contains(&"bob".to_string()));
}

#[tokio::test]
async fn test_channel_member_cache_invalidation_on_part() {
    let db = Database::new(1000, 30);
    
    // Add users
    let user1 = create_test_user("alice", "alice", "192.168.1.1");
    let user2 = create_test_user("bob", "bob", "192.168.1.2");
    db.add_user(user1).unwrap();
    db.add_user(user2).unwrap();
    
    // Add users to channel
    db.add_user_to_channel("alice", "#test").unwrap();
    db.add_user_to_channel("bob", "#test").unwrap();
    
    // Lookup to populate cache
    let members1 = db.get_channel_users("#test");
    assert_eq!(members1.len(), 2);
    
    // Remove user - should invalidate cache
    db.remove_user_from_channel("alice", "#test").unwrap();
    
    // Lookup should return updated list
    let members2 = db.get_channel_users("#test");
    assert_eq!(members2.len(), 1);
    assert!(members2.contains(&"bob".to_string()));
}

#[tokio::test]
async fn test_channel_member_cache_multiple_channels() {
    let db = Database::new(1000, 30);
    
    // Add users
    let user1 = create_test_user("alice", "alice", "192.168.1.1");
    let user2 = create_test_user("bob", "bob", "192.168.1.2");
    let user3 = create_test_user("charlie", "charlie", "192.168.1.3");
    db.add_user(user1).unwrap();
    db.add_user(user2).unwrap();
    db.add_user(user3).unwrap();
    
    // Add users to different channels
    db.add_user_to_channel("alice", "#channel1").unwrap();
    db.add_user_to_channel("bob", "#channel1").unwrap();
    db.add_user_to_channel("bob", "#channel2").unwrap();
    db.add_user_to_channel("charlie", "#channel2").unwrap();
    
    // Lookup both channels
    let members1 = db.get_channel_users("#channel1");
    let members2 = db.get_channel_users("#channel2");
    
    assert_eq!(members1.len(), 2);
    assert!(members1.contains(&"alice".to_string()));
    assert!(members1.contains(&"bob".to_string()));
    
    assert_eq!(members2.len(), 2);
    assert!(members2.contains(&"bob".to_string()));
    assert!(members2.contains(&"charlie".to_string()));
    
    // Remove from one channel shouldn't affect the other
    db.remove_user_from_channel("bob", "#channel1").unwrap();
    
    let members1_updated = db.get_channel_users("#channel1");
    let members2_same = db.get_channel_users("#channel2");
    
    assert_eq!(members1_updated.len(), 1);
    assert_eq!(members2_same.len(), 2);
}

#[tokio::test]
async fn test_cache_clear_operations() {
    let db = Database::new(1000, 30);
    
    // Add some users
    let user1 = create_test_user("alice", "alice", "192.168.1.1");
    let user2 = create_test_user("bob", "bob", "192.168.1.2");
    db.add_user(user1).unwrap();
    db.add_user(user2).unwrap();
    
    // Add to channel
    db.add_user_to_channel("alice", "#test").unwrap();
    db.add_user_to_channel("bob", "#test").unwrap();
    
    // Populate caches
    let _ = db.get_user_by_nick("alice");
    let _ = db.get_channel_users("#test");
    
    // Check caches are populated
    let user_stats = db.get_user_cache_stats();
    let channel_stats = db.get_channel_cache_stats();
    assert!(user_stats.size > 0);
    assert!(channel_stats.size > 0);
    
    // Clear user cache
    db.clear_user_cache();
    let user_stats_after = db.get_user_cache_stats();
    assert_eq!(user_stats_after.size, 0);
    
    // Clear channel cache
    db.clear_channel_cache();
    let channel_stats_after = db.get_channel_cache_stats();
    assert_eq!(channel_stats_after.size, 0);
}

#[tokio::test]
async fn test_cache_clear_all() {
    let db = Database::new(1000, 30);
    
    // Add some users and populate caches
    let user = create_test_user("alice", "alice", "192.168.1.1");
    db.add_user(user).unwrap();
    db.add_user_to_channel("alice", "#test").unwrap();
    
    let _ = db.get_user_by_nick("alice");
    let _ = db.get_channel_users("#test");
    
    // Clear all caches
    db.clear_all_caches();
    
    // Check all caches are empty
    let user_stats = db.get_user_cache_stats();
    let channel_stats = db.get_channel_cache_stats();
    assert_eq!(user_stats.size, 0);
    assert_eq!(channel_stats.size, 0);
}

#[tokio::test]
async fn test_cache_stats_tracking() {
    let db = Database::new(1000, 30);
    
    // Add a user
    let user = create_test_user("alice", "alice", "192.168.1.1");
    db.add_user(user).unwrap();
    
    // Get initial stats
    let stats_initial = db.get_user_cache_stats();
    let initial_hits = stats_initial.total_hits;
    
    // Perform multiple lookups
    for _ in 0..10 {
        let _ = db.get_user_by_nick("alice");
    }
    
    // Check stats increased
    let stats_after = db.get_user_cache_stats();
    assert!(stats_after.total_hits > initial_hits);
    assert!(stats_after.total_hits >= initial_hits + 10);
}

#[tokio::test]
async fn test_concurrent_cache_access() {
    use std::sync::Arc;
    use tokio::task;
    
    let db = Arc::new(Database::new(1000, 30));
    
    // Add users
    for i in 0..10 {
        let user = create_test_user(&format!("user{}", i), &format!("user{}", i), "192.168.1.1");
        db.add_user(user).unwrap();
    }
    
    // Spawn concurrent tasks to access cache
    let mut handles = vec![];
    for i in 0..100 {
        let db_clone = Arc::clone(&db);
        let handle = task::spawn(async move {
            let nick = format!("user{}", i % 10);
            db_clone.get_user_by_nick(&nick)
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_some());
    }
    
    // Check cache stats
    let stats = db.get_user_cache_stats();
    assert!(stats.total_hits > 0);
}

#[tokio::test]
async fn test_user_removal_invalidates_channel_cache() {
    let db = Database::new(1000, 30);
    
    // Add users
    let user1 = create_test_user("alice", "alice", "192.168.1.1");
    let user2 = create_test_user("bob", "bob", "192.168.1.2");
    let alice_id = user1.id;
    db.add_user(user1).unwrap();
    db.add_user(user2).unwrap();
    
    // Add to channel
    db.add_user_to_channel("alice", "#test").unwrap();
    db.add_user_to_channel("bob", "#test").unwrap();
    
    // Populate channel cache
    let members1 = db.get_channel_users("#test");
    assert_eq!(members1.len(), 2);
    
    // Remove user from database (should invalidate channel cache)
    db.remove_user(alice_id).unwrap();
    
    // Channel member list should be updated
    let members2 = db.get_channel_users("#test");
    assert_eq!(members2.len(), 1);
    assert!(members2.contains(&"bob".to_string()));
}

#[tokio::test]
async fn test_empty_channel_not_cached() {
    let db = Database::new(1000, 30);
    
    // Query non-existent channel
    let members = db.get_channel_users("#nonexistent");
    assert_eq!(members.len(), 0);
    
    // Cache should not store empty results
    let stats = db.get_channel_cache_stats();
    assert_eq!(stats.size, 0);
}

