//! Performance-optimized caching system for frequently accessed data
//!
//! This module provides various caching mechanisms to improve performance:
//! - Message serialization cache to avoid repeated string formatting
//! - User lookup cache with LRU eviction
//! - Channel member cache for fast membership checks
//! - DNS result caching to reduce lookup overhead

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Cache entry with TTL
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub expires_at: Instant,
    pub hit_count: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
            hit_count: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    pub fn refresh(&mut self, ttl: Duration) {
        self.expires_at = Instant::now() + ttl;
    }

    pub fn increment_hit(&mut self) {
        self.hit_count += 1;
    }
}

/// LRU cache with configurable size and TTL
pub struct LruCache<K, V> {
    cache: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    access_order: Arc<RwLock<Vec<K>>>,
    max_size: usize,
    default_ttl: Duration,
}

impl<K: Clone + Eq + std::hash::Hash, V: Clone> LruCache<K, V> {
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(Vec::new())),
            max_size,
            default_ttl,
        }
    }

    /// Get a value from cache
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write();
        if let Some(entry) = cache.get_mut(key) {
            if entry.is_expired() {
                cache.remove(key);
                return None;
            }
            entry.increment_hit();
            
            // Update access order
            let mut access = self.access_order.write();
            access.retain(|k| k != key);
            access.push(key.clone());
            
            return Some(entry.value.clone());
        }
        None
    }

    /// Insert a value into cache
    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.cache.write();
        let mut access = self.access_order.write();

        // Evict if at capacity
        if cache.len() >= self.max_size && !cache.contains_key(&key) {
            if let Some(lru_key) = access.first().cloned() {
                cache.remove(&lru_key);
                access.remove(0);
            }
        }

        cache.insert(key.clone(), CacheEntry::new(value, self.default_ttl));
        access.retain(|k| k != &key);
        access.push(key);
    }

    /// Remove a value from cache
    pub fn remove(&self, key: &K) {
        let mut cache = self.cache.write();
        let mut access = self.access_order.write();
        cache.remove(key);
        access.retain(|k| k != key);
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.write().clear();
        self.access_order.write().clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let total_hits: u64 = cache.values().map(|e| e.hit_count).sum();
        CacheStats {
            size: cache.len(),
            capacity: self.max_size,
            total_hits,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub total_hits: u64,
}

/// Message serialization cache
/// Caches pre-formatted IRC messages to avoid repeated string formatting
pub struct MessageCache {
    cache: DashMap<String, CacheEntry<String>>,
    max_size: usize,
    default_ttl: Duration,
}

impl MessageCache {
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            max_size,
            default_ttl,
        }
    }

    /// Get a cached message
    pub fn get(&self, key: &str) -> Option<String> {
        if let Some(mut entry) = self.cache.get_mut(key) {
            if !entry.is_expired() {
                entry.increment_hit();
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Cache a message
    pub fn insert(&self, key: String, message: String) {
        // Simple size limiting by removing random entries if over capacity
        if self.cache.len() >= self.max_size {
            if let Some(entry) = self.cache.iter().next() {
                let key_to_remove = entry.key().clone();
                drop(entry);
                self.cache.remove(&key_to_remove);
            }
        }
        self.cache.insert(key, CacheEntry::new(message, self.default_ttl));
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_hits: u64 = self.cache.iter().map(|e| e.hit_count).sum();
        CacheStats {
            size: self.cache.len(),
            capacity: self.max_size,
            total_hits,
        }
    }
}

/// DNS result cache
pub struct DnsCache {
    hostname_cache: DashMap<String, CacheEntry<String>>,
    ip_cache: DashMap<String, CacheEntry<String>>,
    default_ttl: Duration,
}

impl DnsCache {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            hostname_cache: DashMap::new(),
            ip_cache: DashMap::new(),
            default_ttl,
        }
    }

    /// Get hostname for IP
    pub fn get_hostname(&self, ip: &str) -> Option<String> {
        if let Some(mut entry) = self.ip_cache.get_mut(ip) {
            if !entry.is_expired() {
                entry.increment_hit();
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Cache hostname for IP
    pub fn cache_hostname(&self, ip: String, hostname: String) {
        self.ip_cache.insert(ip.clone(), CacheEntry::new(hostname.clone(), self.default_ttl));
        self.hostname_cache.insert(hostname, CacheEntry::new(ip, self.default_ttl));
    }

    /// Get IP for hostname
    pub fn get_ip(&self, hostname: &str) -> Option<String> {
        if let Some(mut entry) = self.hostname_cache.get_mut(hostname) {
            if !entry.is_expired() {
                entry.increment_hit();
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Clear DNS cache
    pub fn clear(&self) {
        self.hostname_cache.clear();
        self.ip_cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> (CacheStats, CacheStats) {
        let hostname_hits: u64 = self.hostname_cache.iter().map(|e| e.hit_count).sum();
        let ip_hits: u64 = self.ip_cache.iter().map(|e| e.hit_count).sum();
        
        (
            CacheStats {
                size: self.hostname_cache.len(),
                capacity: usize::MAX,
                total_hits: hostname_hits,
            },
            CacheStats {
                size: self.ip_cache.len(),
                capacity: usize::MAX,
                total_hits: ip_hits,
            },
        )
    }
}

/// User lookup cache for fast access to frequently accessed users
pub type UserLookupCache = LruCache<String, Uuid>;

/// Channel member cache for fast membership checks
pub struct ChannelMemberCache {
    cache: DashMap<String, CacheEntry<Vec<String>>>,
    default_ttl: Duration,
}

impl ChannelMemberCache {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            default_ttl,
        }
    }

    /// Get cached channel members
    pub fn get(&self, channel: &str) -> Option<Vec<String>> {
        if let Some(mut entry) = self.cache.get_mut(channel) {
            if !entry.is_expired() {
                entry.increment_hit();
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Cache channel members
    pub fn cache(&self, channel: String, members: Vec<String>) {
        self.cache.insert(channel, CacheEntry::new(members, self.default_ttl));
    }

    /// Invalidate channel cache
    pub fn invalidate(&self, channel: &str) {
        self.cache.remove(channel);
    }

    /// Clear all cache
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_hits: u64 = self.cache.iter().map(|e| e.hit_count).sum();
        CacheStats {
            size: self.cache.len(),
            capacity: usize::MAX,
            total_hits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache() {
        let cache = LruCache::new(2, Duration::from_secs(60));
        
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");
        
        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));
        
        // This should evict key1 (least recently used)
        cache.insert("key3", "value3");
        assert_eq!(cache.get(&"key1"), None);
        assert_eq!(cache.get(&"key3"), Some("value3"));
    }

    #[test]
    fn test_message_cache() {
        let cache = MessageCache::new(100, Duration::from_secs(60));
        
        cache.insert("PING :server".to_string(), "PONG :server\r\n".to_string());
        assert_eq!(cache.get("PING :server"), Some("PONG :server\r\n".to_string()));
        
        cache.clear();
        assert_eq!(cache.get("PING :server"), None);
    }

    #[test]
    fn test_dns_cache() {
        let cache = DnsCache::new(Duration::from_secs(300));
        
        cache.cache_hostname("192.168.1.1".to_string(), "example.com".to_string());
        
        assert_eq!(cache.get_hostname("192.168.1.1"), Some("example.com".to_string()));
        assert_eq!(cache.get_ip("example.com"), Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_channel_member_cache() {
        let cache = ChannelMemberCache::new(Duration::from_secs(30));
        
        let members = vec!["alice".to_string(), "bob".to_string()];
        cache.cache("#test".to_string(), members.clone());
        
        assert_eq!(cache.get("#test"), Some(members));
        
        cache.invalidate("#test");
        assert_eq!(cache.get("#test"), None);
    }
}





