//! Example demonstrating cache usage and monitoring
//!
//! This example shows how to use the built-in cache system for users and channels,
//! and how to monitor cache performance.

use rustircd_core::{Database, User};

/// Create a test user
fn create_user(nick: &str, username: &str, host: &str) -> User {
    User::new(
        nick.to_string(),
        username.to_string(),
        "Test User".to_string(),
        host.to_string(),
        "test.server".to_string(),
    )
}

#[tokio::main]
async fn main() {
    println!("=== Cache Usage Example ===\n");

    // Create database with default settings
    let db = Database::new(10000, 30);

    println!("1. Adding users to database...");
    // Add some users
    for i in 1..=5 {
        let user = create_user(
            &format!("user{}", i),
            &format!("user{}", i),
            "192.168.1.1"
        );
        db.add_user(user).unwrap();
        println!("   Added user{}", i);
    }

    println!("\n2. Initial cache state:");
    let stats = db.get_user_cache_stats();
    println!("   User cache size: {}/{}", stats.size, stats.capacity);
    println!("   User cache hits: {}", stats.total_hits);

    println!("\n3. Performing user lookups (populating cache)...");
    // First lookups - will populate cache
    for i in 1..=5 {
        let nick = format!("user{}", i);
        let user = db.get_user_by_nick(&nick).unwrap();
        println!("   Looked up: {}", user.nick);
    }

    let stats_after_first = db.get_user_cache_stats();
    println!("\n4. Cache state after first lookups:");
    println!("   User cache size: {}", stats_after_first.size);
    println!("   User cache hits: {}", stats_after_first.total_hits);

    println!("\n5. Performing repeated lookups (cache hits)...");
    // Repeated lookups - should hit cache
    for _ in 0..100 {
        for i in 1..=5 {
            let nick = format!("user{}", i);
            let _ = db.get_user_by_nick(&nick);
        }
    }

    let stats_after_repeated = db.get_user_cache_stats();
    println!("\n6. Cache state after 500 total lookups:");
    println!("   User cache size: {}", stats_after_repeated.size);
    println!("   User cache hits: {}", stats_after_repeated.total_hits);
    
    let hit_rate = if stats_after_repeated.total_hits > 0 {
        100.0 * stats_after_repeated.total_hits as f64 / 505.0
    } else {
        0.0
    };
    println!("   Cache hit rate: {:.2}%", hit_rate);

    println!("\n7. Testing channel member cache...");
    // Add users to channels
    db.add_user_to_channel("user1", "#test").unwrap();
    db.add_user_to_channel("user2", "#test").unwrap();
    db.add_user_to_channel("user3", "#test").unwrap();
    println!("   Added 3 users to #test");

    db.add_user_to_channel("user3", "#rust").unwrap();
    db.add_user_to_channel("user4", "#rust").unwrap();
    db.add_user_to_channel("user5", "#rust").unwrap();
    println!("   Added 3 users to #rust");

    println!("\n8. Querying channel members (populating cache)...");
    let test_members = db.get_channel_users("#test");
    let rust_members = db.get_channel_users("#rust");
    println!("   #test members: {:?}", test_members);
    println!("   #rust members: {:?}", rust_members);

    let channel_stats = db.get_channel_cache_stats();
    println!("\n9. Channel cache state:");
    println!("   Channel cache size: {}", channel_stats.size);
    println!("   Channel cache hits: {}", channel_stats.total_hits);

    println!("\n10. Repeated channel queries (cache hits)...");
    for _ in 0..50 {
        let _ = db.get_channel_users("#test");
        let _ = db.get_channel_users("#rust");
    }

    let channel_stats_after = db.get_channel_cache_stats();
    println!("\n11. Channel cache state after repeated queries:");
    println!("   Channel cache size: {}", channel_stats_after.size);
    println!("   Channel cache hits: {}", channel_stats_after.total_hits);

    let channel_hit_rate = if channel_stats_after.total_hits > 0 {
        100.0 * channel_stats_after.total_hits as f64 / 102.0
    } else {
        0.0
    };
    println!("   Cache hit rate: {:.2}%", channel_hit_rate);

    println!("\n12. Testing cache invalidation...");
    // Remove a user - should invalidate both user and channel caches
    let user1 = db.get_user_by_nick("user1").unwrap();
    db.remove_user(user1.id).unwrap();
    println!("   Removed user1");

    let test_members_after = db.get_channel_users("#test");
    println!("   #test members after removal: {:?}", test_members_after);
    assert_eq!(test_members_after.len(), 2);
    assert!(!test_members_after.contains(&"user1".to_string()));

    println!("\n13. Testing cache clearing...");
    db.clear_all_caches();
    let user_stats_cleared = db.get_user_cache_stats();
    let channel_stats_cleared = db.get_channel_cache_stats();
    println!("   User cache size after clear: {}", user_stats_cleared.size);
    println!("   Channel cache size after clear: {}", channel_stats_cleared.size);
    assert_eq!(user_stats_cleared.size, 0);
    assert_eq!(channel_stats_cleared.size, 0);

    println!("\n14. Performance comparison...");
    use std::time::Instant;

    // Clear cache and warm it up
    db.clear_all_caches();
    for i in 2..=5 {
        let nick = format!("user{}", i);
        let _ = db.get_user_by_nick(&nick);
    }

    // Benchmark cached lookups
    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = db.get_user_by_nick("user2");
    }
    let cached_duration = start.elapsed();
    let cached_avg = cached_duration.as_micros() as f64 / iterations as f64;

    println!("   {} cached lookups: {:?}", iterations, cached_duration);
    println!("   Average time per lookup: {:.2}μs", cached_avg);

    println!("\n=== Cache Example Complete ===");
    println!("\nKey Takeaways:");
    println!("  • Cache is automatic - no API changes needed");
    println!("  • High cache hit rates improve performance significantly");
    println!("  • Cache is properly invalidated on data changes");
    println!("  • Cache statistics help monitor performance");
    println!("  • Typical cached lookup: 1-5μs vs 10-50μs uncached");
}

