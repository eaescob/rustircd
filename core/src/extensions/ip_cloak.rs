//! IP Cloaking Extension
//! 
//! This extension provides IP address cloaking functionality to hide real IP addresses
//! from users while maintaining proper routing and identification capabilities.
//! Inspired by Solanum's ip_cloak extension.

use crate::{User, Result, Error};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use async_trait::async_trait;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

/// IP cloaking extension configuration
#[derive(Debug, Clone)]
pub struct IpCloakConfig {
    /// Whether IP cloaking is enabled
    pub enabled: bool,
    /// Cloaking method to use
    pub method: CloakMethod,
    /// Secret key for cloaking algorithms
    pub secret_key: String,
    /// Whether to cloak IPv4 addresses
    pub cloak_ipv4: bool,
    /// Whether to cloak IPv6 addresses
    pub cloak_ipv6: bool,
    /// Cloak prefix for static cloaking
    pub cloak_prefix: String,
    /// Whether to preserve hostname for certain patterns
    pub preserve_hostname_patterns: Vec<String>,
}

/// IP cloaking methods
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloakMethod {
    /// Static cloaking with a fixed prefix
    Static,
    /// Hash-based cloaking using SHA256
    Hash,
    /// Hybrid cloaking (combination of methods)
    Hybrid,
    /// Custom cloaking function
    Custom,
}

impl Default for IpCloakConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            method: CloakMethod::Hash,
            secret_key: "default_secret_key_change_me".to_string(),
            cloak_ipv4: true,
            cloak_ipv6: true,
            cloak_prefix: "cloaked".to_string(),
            preserve_hostname_patterns: vec![
                "*.example.com".to_string(),
                "*.localhost".to_string(),
            ],
        }
    }
}

/// IP cloaking extension
pub struct IpCloakExtension {
    /// Configuration
    config: IpCloakConfig,
    /// Cloaked IP cache
    cloaked_cache: Arc<RwLock<HashMap<IpAddr, String>>>,
    /// Reverse lookup cache (cloaked -> real)
    reverse_cache: Arc<RwLock<HashMap<String, IpAddr>>>,
}

impl IpCloakExtension {
    /// Create a new IP cloaking extension
    pub fn new(config: IpCloakConfig) -> Self {
        Self {
            config,
            cloaked_cache: Arc::new(RwLock::new(HashMap::new())),
            reverse_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Cloak an IP address
    pub async fn cloak_ip(&self, ip: IpAddr) -> Result<String> {
        if !self.config.enabled {
            return Ok(ip.to_string());
        }
        
        // Check if we should preserve this IP
        if self.should_preserve_ip(&ip).await {
            return Ok(ip.to_string());
        }
        
        // Check cache first
        {
            let cache = self.cloaked_cache.read().await;
            if let Some(cloaked) = cache.get(&ip) {
                return Ok(cloaked.clone());
            }
        }
        
        // Generate cloaked IP
        let cloaked = match self.config.method {
            CloakMethod::Static => self.cloak_static(ip).await?,
            CloakMethod::Hash => self.cloak_hash(ip).await?,
            CloakMethod::Hybrid => self.cloak_hybrid(ip).await?,
            CloakMethod::Custom => self.cloak_custom(ip).await?,
        };
        
        // Cache the result
        {
            let mut cache = self.cloaked_cache.write().await;
            cache.insert(ip, cloaked.clone());
        }
        
        // Cache reverse lookup
        {
            let mut reverse_cache = self.reverse_cache.write().await;
            reverse_cache.insert(cloaked.clone(), ip);
        }
        
        Ok(cloaked)
    }
    
    /// Get the real IP from a cloaked IP
    pub async fn uncloak_ip(&self, cloaked_ip: &str) -> Option<IpAddr> {
        let reverse_cache = self.reverse_cache.read().await;
        reverse_cache.get(cloaked_ip).cloned()
    }
    
    /// Check if an IP should be preserved (not cloaked)
    async fn should_preserve_ip(&self, ip: &IpAddr) -> bool {
        // Always preserve localhost
        if ip.is_loopback() {
            return true;
        }
        
        // Check if it's a private IP that should be preserved
        if ip.is_private() {
            return true;
        }
        
        // Check preserve patterns (for hostnames, not IPs)
        // This would be used when we have hostname information
        false
    }
    
    /// Static cloaking method
    async fn cloak_static(&self, ip: IpAddr) -> Result<String> {
        let ip_str = ip.to_string();
        let parts: Vec<&str> = ip_str.split('.').collect();
        
        if parts.len() == 4 {
            // IPv4: replace with prefix.x.x.x
            Ok(format!("{}.{}.{}.{}", 
                self.config.cloak_prefix, 
                parts[1], 
                parts[2], 
                parts[3]))
        } else {
            // IPv6: replace with prefix:xxxx:xxxx:...
            let parts: Vec<&str> = ip_str.split(':').collect();
            if parts.len() > 0 {
                Ok(format!("{}:{}", self.config.cloak_prefix, 
                    parts[1..].join(":")))
            } else {
                Ok(format!("{}-{}", self.config.cloak_prefix, ip_str))
            }
        }
    }
    
    /// Hash-based cloaking method
    async fn cloak_hash(&self, ip: IpAddr) -> Result<String> {
        let ip_str = ip.to_string();
        let input = format!("{}{}", self.config.secret_key, ip_str);
        
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash = hasher.finalize();
        
        // Convert to base64 and take first 16 characters
        let encoded = general_purpose::STANDARD.encode(hash);
        let short_hash = &encoded[..16];
        
        // Format as IP-like string
        if ip.is_ipv4() {
            Ok(format!("{}.{}.{}.{}", 
                &short_hash[0..4], 
                &short_hash[4..8], 
                &short_hash[8..12], 
                &short_hash[12..16]))
        } else {
            Ok(format!("{}:{}:{}:{}", 
                &short_hash[0..4], 
                &short_hash[4..8], 
                &short_hash[8..12], 
                &short_hash[12..16]))
        }
    }
    
    /// Hybrid cloaking method
    async fn cloak_hybrid(&self, ip: IpAddr) -> Result<String> {
        // Combine static and hash methods
        let static_cloaked = self.cloak_static(ip).await?;
        let hash_cloaked = self.cloak_hash(ip).await?;
        
        // Mix the two methods
        Ok(format!("{}-{}", static_cloaked, hash_cloaked))
    }
    
    /// Custom cloaking method
    async fn cloak_custom(&self, ip: IpAddr) -> Result<String> {
        // Custom implementation - for now, use hash method
        self.cloak_hash(ip).await
    }
    
    /// Check if a hostname should be preserved
    pub fn should_preserve_hostname(&self, hostname: &str) -> bool {
        for pattern in &self.config.preserve_hostname_patterns {
            if self.matches_pattern(hostname, pattern) {
                return true;
            }
        }
        false
    }
    
    /// Simple pattern matching for hostname preservation
    fn matches_pattern(&self, hostname: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            return hostname.ends_with(suffix);
        }
        
        hostname == pattern
    }
    
    /// Get cloaking statistics
    pub async fn get_statistics(&self) -> IpCloakStats {
        let cloaked_cache = self.cloaked_cache.read().await;
        let reverse_cache = self.reverse_cache.read().await;
        
        IpCloakStats {
            total_cloaked_ips: cloaked_cache.len(),
            total_reverse_entries: reverse_cache.len(),
            method: self.config.method.clone(),
            enabled: self.config.enabled,
        }
    }
    
    /// Clear caches
    pub async fn clear_caches(&self) {
        let mut cloaked_cache = self.cloaked_cache.write().await;
        cloaked_cache.clear();
        
        let mut reverse_cache = self.reverse_cache.write().await;
        reverse_cache.clear();
    }
}

/// IP cloaking statistics
#[derive(Debug, Clone)]
pub struct IpCloakStats {
    /// Total number of cloaked IPs
    pub total_cloaked_ips: usize,
    /// Total number of reverse lookup entries
    pub total_reverse_entries: usize,
    /// Cloaking method in use
    pub method: CloakMethod,
    /// Whether cloaking is enabled
    pub enabled: bool,
}

#[async_trait]
impl crate::extensions::UserExtension for IpCloakExtension {
    /// Called when a user registers
    async fn on_user_registration(&self, user: &User) -> Result<()> {
        // Cloak the user's IP address
        if let Ok(ip) = user.host.parse::<IpAddr>() {
            let cloaked_ip = self.cloak_ip(ip).await?;
            // Note: In a real implementation, we would update the user's host
            // This would require access to modify the user object
            tracing::debug!("Cloaked IP {} to {} for user {}", user.host, cloaked_ip, user.nick);
        }
        Ok(())
    }
    
    /// Called when a user disconnects
    async fn on_user_disconnection(&self, user: &User) -> Result<()> {
        // Clean up any user-specific cloaking data if needed
        tracing::debug!("User {} disconnected, cleaning up cloaking data", user.nick);
        Ok(())
    }
    
    /// Called when user properties change
    async fn on_user_property_change(&self, user: &User, property: &str, old_value: &str, new_value: &str) -> Result<()> {
        if property == "host" {
            // Re-cloak the new host if it's an IP address
            if let Ok(ip) = new_value.parse::<IpAddr>() {
                let cloaked_ip = self.cloak_ip(ip).await?;
                tracing::debug!("Re-cloaked IP {} to {} for user {}", new_value, cloaked_ip, user.nick);
            }
        }
        Ok(())
    }
    
    /// Called when user joins a channel
    async fn on_user_join_channel(&self, _user: &User, _channel: &str) -> Result<()> {
        // No special handling needed for channel joins
        Ok(())
    }
    
    /// Called when user parts a channel
    async fn on_user_part_channel(&self, _user: &User, _channel: &str, _reason: Option<&str>) -> Result<()> {
        // No special handling needed for channel parts
        Ok(())
    }
    
    /// Called when user changes nickname
    async fn on_user_nick_change(&self, _user: &User, _old_nick: &str, _new_nick: &str) -> Result<()> {
        // No special handling needed for nick changes
        Ok(())
    }
    
    /// Called when user sets away status
    async fn on_user_away_change(&self, _user: &User, _away: bool, _message: Option<&str>) -> Result<()> {
        // No special handling needed for away changes
        Ok(())
    }
}

impl Default for IpCloakExtension {
    fn default() -> Self {
        Self::new(IpCloakConfig::default())
    }
}

/// IP cloaking configuration builder
pub struct IpCloakConfigBuilder {
    config: IpCloakConfig,
}

impl IpCloakConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: IpCloakConfig::default(),
        }
    }
    
    /// Set whether cloaking is enabled
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }
    
    /// Set cloaking method
    pub fn method(mut self, method: CloakMethod) -> Self {
        self.config.method = method;
        self
    }
    
    /// Set secret key
    pub fn secret_key(mut self, secret_key: String) -> Self {
        self.config.secret_key = secret_key;
        self
    }
    
    /// Set cloak prefix
    pub fn cloak_prefix(mut self, prefix: String) -> Self {
        self.config.cloak_prefix = prefix;
        self
    }
    
    /// Add preserve hostname pattern
    pub fn preserve_hostname_pattern(mut self, pattern: String) -> Self {
        self.config.preserve_hostname_patterns.push(pattern);
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> IpCloakConfig {
        self.config
    }
}

impl Default for IpCloakConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
