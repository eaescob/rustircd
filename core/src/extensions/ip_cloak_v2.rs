//! IP Cloaking Extension (Solanum 4.0c Style)
//! 
//! This extension provides IP address cloaking functionality based on Solanum's
//! ip_cloaking_4.0.c implementation. It uses CIDR masking, SHA3 MAC computation,
//! and base32 encoding to create secure cloaked hostnames.
//! 
//! Based on: https://github.com/solanum-ircd/solanum/blob/main/extensions/ip_cloaking_4.0.c

use crate::{User, Result, Error};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use async_trait::async_trait;
use sha3::{Sha3_256, Digest};
use base32::{Alphabet, encode};

/// IP cloaking configuration based on Solanum's approach
#[derive(Debug, Clone)]
pub struct IpCloakConfig {
    /// Whether IP cloaking is enabled
    pub enabled: bool,
    /// Secret key for MAC computation
    pub secret_key: String,
    /// Suffix for cloaked hostnames
    pub suffix: String,
    /// IPv4 CIDR length for masking
    pub ipv4_cidr: u8,
    /// IPv6 CIDR length for masking
    pub ipv6_cidr: u8,
    /// Number of bits for MAC computation
    pub mac_bits: u8,
    /// Whether to preserve certain hostname patterns
    pub preserve_patterns: Vec<String>,
}

impl Default for IpCloakConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            secret_key: "change_this_secret_key".to_string(),
            suffix: ".cloaked".to_string(),
            ipv4_cidr: 16,  // /16 for IPv4
            ipv6_cidr: 32,  // /32 for IPv6
            mac_bits: 64,   // 64 bits for MAC
            preserve_patterns: vec![
                "*.example.com".to_string(),
                "*.localhost".to_string(),
            ],
        }
    }
}

/// IP cloaking extension based on Solanum's implementation
pub struct IpCloakExtension {
    /// Configuration
    config: IpCloakConfig,
    /// Cloaked IP cache
    cloaked_cache: Arc<RwLock<HashMap<IpAddr, String>>>,
    /// Reverse lookup cache
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
    
    /// Cloak an IP address using Solanum's algorithm
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
        
        // Apply CIDR masking
        let masked_ip = self.apply_cidr_mask(ip);
        
        // Compute MAC using SHA3
        let mac = self.compute_mac(&masked_ip).await?;
        
        // Encode in base32
        let encoded = self.encode_base32(&mac);
        
        // Create cloaked hostname
        let cloaked = format!("{}{}", encoded, self.config.suffix);
        
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
    
    /// Apply CIDR masking to an IP address
    fn apply_cidr_mask(&self, ip: IpAddr) -> IpAddr {
        match ip {
            IpAddr::V4(ipv4) => {
                let cidr = self.config.ipv4_cidr.min(32);
                let mask = if cidr == 0 {
                    0
                } else {
                    !((1u32 << (32 - cidr)) - 1)
                };
                let masked = u32::from(ipv4) & mask;
                IpAddr::V4(Ipv4Addr::from(masked))
            }
            IpAddr::V6(ipv6) => {
                let cidr = self.config.ipv6_cidr.min(128);
                let mut octets = ipv6.octets();
                
                for i in 0..16 {
                    let bit_offset = i * 8;
                    if bit_offset >= cidr as usize {
                        octets[i] = 0;
                    } else if bit_offset + 8 > cidr as usize {
                        let remaining_bits = cidr as usize - bit_offset;
                        let mask = !((1u8 << (8 - remaining_bits)) - 1);
                        octets[i] &= mask;
                    }
                }
                
                IpAddr::V6(Ipv6Addr::from(octets))
            }
        }
    }
    
    /// Compute MAC using SHA3 (similar to Solanum's approach)
    async fn compute_mac(&self, ip: &IpAddr) -> Result<Vec<u8>> {
        let ip_bytes = match ip {
            IpAddr::V4(ipv4) => ipv4.octets().to_vec(),
            IpAddr::V6(ipv6) => ipv6.octets().to_vec(),
        };
        
        let mut hasher = Sha3_256::new();
        hasher.update(self.config.secret_key.as_bytes());
        hasher.update(&ip_bytes);
        let hash = hasher.finalize();
        
        // Take the specified number of bits
        let bits = self.config.mac_bits.min(256);
        let bytes = (bits + 7) / 8; // Round up to nearest byte
        Ok(hash[..bytes].to_vec())
    }
    
    /// Encode bytes in base32
    fn encode_base32(&self, data: &[u8]) -> String {
        encode(Alphabet::RFC4648 { padding: false }, data)
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
        
        false
    }
    
    /// Check if a hostname should be preserved
    pub fn should_preserve_hostname(&self, hostname: &str) -> bool {
        for pattern in &self.config.preserve_patterns {
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
    
    /// Get the real IP from a cloaked hostname
    pub async fn uncloak_ip(&self, cloaked_hostname: &str) -> Option<IpAddr> {
        let reverse_cache = self.reverse_cache.read().await;
        reverse_cache.get(cloaked_hostname).cloned()
    }
    
    /// Get cloaking statistics
    pub async fn get_statistics(&self) -> IpCloakStats {
        let cloaked_cache = self.cloaked_cache.read().await;
        let reverse_cache = self.reverse_cache.read().await;
        
        IpCloakStats {
            total_cloaked_ips: cloaked_cache.len(),
            total_reverse_entries: reverse_cache.len(),
            enabled: self.config.enabled,
            ipv4_cidr: self.config.ipv4_cidr,
            ipv6_cidr: self.config.ipv6_cidr,
            mac_bits: self.config.mac_bits,
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
    /// Whether cloaking is enabled
    pub enabled: bool,
    /// IPv4 CIDR length
    pub ipv4_cidr: u8,
    /// IPv6 CIDR length
    pub ipv6_cidr: u8,
    /// MAC bits
    pub mac_bits: u8,
}

#[async_trait]
impl crate::extensions::UserExtension for IpCloakExtension {
    /// Called when a user registers
    async fn on_user_registration(&self, user: &User) -> Result<()> {
        // Cloak the user's IP address
        if let Ok(ip) = user.host.parse::<IpAddr>() {
            let cloaked_ip = self.cloak_ip(ip).await?;
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
    
    /// Set secret key
    pub fn secret_key(mut self, secret_key: String) -> Self {
        self.config.secret_key = secret_key;
        self
    }
    
    /// Set suffix
    pub fn suffix(mut self, suffix: String) -> Self {
        self.config.suffix = suffix;
        self
    }
    
    /// Set IPv4 CIDR length
    pub fn ipv4_cidr(mut self, cidr: u8) -> Self {
        self.config.ipv4_cidr = cidr;
        self
    }
    
    /// Set IPv6 CIDR length
    pub fn ipv6_cidr(mut self, cidr: u8) -> Self {
        self.config.ipv6_cidr = cidr;
        self
    }
    
    /// Set MAC bits
    pub fn mac_bits(mut self, bits: u8) -> Self {
        self.config.mac_bits = bits;
        self
    }
    
    /// Add preserve pattern
    pub fn preserve_pattern(mut self, pattern: String) -> Self {
        self.config.preserve_patterns.push(pattern);
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
