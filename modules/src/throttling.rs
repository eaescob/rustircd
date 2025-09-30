//! Connection throttling module for rate limiting client connections

use rustircd_core::{Module, module::{ModuleResult, ModuleStatsResponse}, Client, Message, User, Error, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, warn, debug, error};

/// Throttling entry for tracking connection attempts from an IP
#[derive(Debug, Clone)]
struct ThrottleEntry {
    /// List of connection timestamps within the time window
    connection_times: Vec<Instant>,
    /// Current throttling stage (0 = no throttling, 1-10 = throttling stages)
    stage: u8,
    /// When the current throttle expires
    throttle_until: Option<Instant>,
}

impl ThrottleEntry {
    fn new() -> Self {
        Self {
            connection_times: Vec::new(),
            stage: 0,
            throttle_until: None,
        }
    }

    /// Add a connection attempt and check if throttling should be applied
    fn add_connection(&mut self, config: &rustircd_core::config::ThrottlingConfig) -> bool {
        let now = Instant::now();
        
        // Clean old connection times outside the time window
        let cutoff = now - Duration::from_secs(config.time_window_seconds);
        self.connection_times.retain(|&time| time > cutoff);
        
        // Check if currently throttled
        if let Some(throttle_until) = self.throttle_until {
            if now < throttle_until {
                return false; // Still throttled
            } else {
                // Throttle expired, reset stage
                self.stage = 0;
                self.throttle_until = None;
            }
        }
        
        // Add current connection
        self.connection_times.push(now);
        
        // Check if we've exceeded the limit
        if self.connection_times.len() > config.max_connections_per_ip {
            // Move to next stage or max stage
            self.stage = (self.stage + 1).min(config.max_stages);
            
            // Calculate throttle duration: initial_seconds * (stage_factor ^ (stage-1))
            let throttle_duration = config.initial_throttle_seconds * 
                config.stage_factor.pow(self.stage.saturating_sub(1) as u32);
            
            self.throttle_until = Some(now + Duration::from_secs(throttle_duration));
            
            warn!(
                "IP throttled at stage {} for {} seconds ({} connections in {}s window)",
                self.stage,
                throttle_duration,
                self.connection_times.len(),
                config.time_window_seconds
            );
            
            return false; // Throttled
        }
        
        true // Connection allowed
    }

    /// Check if currently throttled without adding a connection
    fn is_throttled(&self) -> bool {
        if let Some(throttle_until) = self.throttle_until {
            Instant::now() < throttle_until
        } else {
            false
        }
    }

    /// Get remaining throttle time in seconds
    fn remaining_throttle_seconds(&self) -> u64 {
        if let Some(throttle_until) = self.throttle_until {
            let now = Instant::now();
            if now < throttle_until {
                (throttle_until - now).as_secs()
            } else {
                0
            }
        } else {
            0
        }
    }
}

/// Connection throttling module
pub struct ThrottlingModule {
    name: String,
    version: String,
    description: String,
    /// IP address to throttle entry mapping
    throttle_map: Arc<RwLock<HashMap<IpAddr, ThrottleEntry>>>,
    /// Throttling configuration
    config: Arc<rustircd_core::config::ThrottlingConfig>,
}

impl ThrottlingModule {
    pub fn new(config: rustircd_core::config::ThrottlingConfig) -> Self {
        Self {
            name: "throttling".to_string(),
            version: "1.0.0".to_string(),
            description: "Connection throttling and rate limiting module".to_string(),
            throttle_map: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(config),
        }
    }

    /// Check if an IP address is allowed to connect
    pub async fn check_connection_allowed(&self, ip_addr: IpAddr) -> Result<bool> {
        if !self.config.enabled {
            return Ok(true);
        }

        let mut throttle_map = self.throttle_map.write().await;
        let entry = throttle_map.entry(ip_addr).or_insert_with(ThrottleEntry::new);
        
        let allowed = entry.add_connection(&self.config);
        
        if !allowed {
            debug!(
                "Connection from {} blocked - stage {}, remaining: {}s",
                ip_addr,
                entry.stage,
                entry.remaining_throttle_seconds()
            );
        }
        
        Ok(allowed)
    }

    /// Get throttling status for an IP address
    pub async fn get_throttle_status(&self, ip_addr: IpAddr) -> (bool, u8, u64) {
        let throttle_map = self.throttle_map.read().await;
        if let Some(entry) = throttle_map.get(&ip_addr) {
            let is_throttled = entry.is_throttled();
            let stage = entry.stage;
            let remaining = entry.remaining_throttle_seconds();
            (is_throttled, stage, remaining)
        } else {
            (false, 0, 0)
        }
    }

    /// Start the cleanup task to remove expired entries
    async fn start_cleanup_task(&self) {
        let throttle_map = self.throttle_map.clone();
        let cleanup_interval = self.config.cleanup_interval_seconds;
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(cleanup_interval));
            
            loop {
                interval.tick().await;
                
                let now = Instant::now();
                let mut throttle_map = throttle_map.write().await;
                let initial_count = throttle_map.len();
                
                // Remove entries that are not throttled and have no recent connections
                throttle_map.retain(|_, entry| {
                    entry.is_throttled() || 
                    !entry.connection_times.is_empty()
                });
                
                let final_count = throttle_map.len();
                if final_count < initial_count {
                    debug!("Cleaned up {} expired throttle entries", initial_count - final_count);
                }
            }
        });
    }
}

#[async_trait]
impl Module for ThrottlingModule {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn init(&mut self) -> Result<()> {
        if !self.config.enabled {
            info!("Throttling module disabled");
            return Ok(());
        }
        
        info!(
            "Initializing throttling module - max {} connections per IP in {}s window, {} stages, {}s initial throttle",
            self.config.max_connections_per_ip,
            self.config.time_window_seconds,
            self.config.max_stages,
            self.config.initial_throttle_seconds
        );
        
        // Start cleanup task
        self.start_cleanup_task().await;
        
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up throttling module");
        Ok(())
    }
    
    async fn handle_message(&mut self, _client: &Client, _message: &Message) -> Result<ModuleResult> {
        // Throttling module doesn't handle specific messages
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, _user: &User) -> Result<()> {
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, _user: &User) -> Result<()> {
        Ok(())
    }
    
    fn get_capabilities(&self) -> Vec<String> {
        vec!["connection_throttling".to_string()]
    }
    
    fn supports_capability(&self, capability: &str) -> bool {
        capability == "connection_throttling"
    }
    
    fn get_numeric_replies(&self) -> Vec<u16> {
        vec![]
    }
    
    fn handles_numeric_reply(&self, _numeric: u16) -> bool {
        false
    }
    
    async fn handle_numeric_reply(&mut self, _numeric: u16, _params: Vec<String>) -> Result<()> {
        Ok(())
    }
    
    async fn handle_stats_query(&mut self, query: &str, client_id: uuid::Uuid, server: Option<&crate::Server>) -> Result<Vec<ModuleStatsResponse>> {
        let mut responses = Vec::new();
        
        if query == "T" {
            // Handle throttling statistics
            let throttle_map = self.throttle_map.read().await;
            
            if throttle_map.is_empty() {
                responses.push(ModuleStatsResponse::ModuleStats(
                    "THROTTLING".to_string(),
                    "No throttled IPs".to_string()
                ));
            } else {
                // Check if the requesting user is an operator and if server details are allowed
                let is_operator = if let Some(server) = server {
                    let users = server.users.read().await;
                    let requesting_user = users.get(&client_id);
                    requesting_user.map(|u| u.is_operator).unwrap_or(false)
                } else {
                    false
                };
                
                let show_details = is_operator && server.map(|s| s.config.server.show_server_details_in_stats).unwrap_or(false);
                
                if show_details {
                    // Show detailed information to operators (if configured)
                    for (ip, entry) in throttle_map.iter() {
                        let status = if entry.is_throttled() {
                            format!("THROTTLED stage={} remaining={}s", entry.stage, entry.remaining_throttle_seconds())
                        } else {
                            format!("ACTIVE connections={}", entry.connection_times.len())
                        };
                        
                        responses.push(ModuleStatsResponse::ModuleStats(
                            ip.to_string(),
                            status
                        ));
                    }
                } else {
                    // Show limited information to non-operators or when configured to hide details
                    let total_throttled = throttle_map.values().filter(|entry| entry.is_throttled()).count();
                    let total_active = throttle_map.len() - total_throttled;
                    
                    responses.push(ModuleStatsResponse::ModuleStats(
                        "THROTTLING".to_string(),
                        format!("{} throttled IPs, {} active IPs", total_throttled, total_active)
                    ));
                }
            }
        }
        
        Ok(responses)
    }
    
    fn get_stats_queries(&self) -> Vec<String> {
        vec!["T".to_string()]
    }
}

/// Helper function to extract IP address from socket address
pub fn extract_ip_from_socket_addr(addr: &std::net::SocketAddr) -> IpAddr {
    addr.ip()
}

/// Helper function to extract IP address from string representation
pub fn parse_ip_from_string(addr_str: &str) -> Result<IpAddr, Error> {
    addr_str.parse::<IpAddr>()
        .map_err(|e| Error::Connection(format!("Invalid IP address '{}': {}", addr_str, e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    fn create_test_config() -> rustircd_core::config::ThrottlingConfig {
        rustircd_core::config::ThrottlingConfig {
            enabled: true,
            max_connections_per_ip: 3,
            time_window_seconds: 60,
            initial_throttle_seconds: 5,
            max_stages: 3,
            stage_factor: 2,
            cleanup_interval_seconds: 300,
        }
    }

    #[tokio::test]
    async fn test_connection_allowed_within_limit() {
        let config = create_test_config();
        let module = ThrottlingModule::new(config);
        let ip = IpAddr::from_str("127.0.0.1").unwrap();

        // First few connections should be allowed
        assert!(module.check_connection_allowed(ip).await.unwrap());
        assert!(module.check_connection_allowed(ip).await.unwrap());
        assert!(module.check_connection_allowed(ip).await.unwrap());
    }

    #[tokio::test]
    async fn test_connection_blocked_over_limit() {
        let config = create_test_config();
        let module = ThrottlingModule::new(config);
        let ip = IpAddr::from_str("127.0.0.1").unwrap();

        // Fill up the limit
        for _ in 0..3 {
            assert!(module.check_connection_allowed(ip).await.unwrap());
        }

        // Next connection should be blocked
        assert!(!module.check_connection_allowed(ip).await.unwrap());
    }

    #[tokio::test]
    async fn test_throttle_stages() {
        let config = create_test_config();
        let module = ThrottlingModule::new(config);
        let ip = IpAddr::from_str("127.0.0.1").unwrap();

        // Trigger multiple throttling stages
        for stage in 0..3 {
            // Fill up the limit to trigger throttling
            for _ in 0..3 {
                module.check_connection_allowed(ip).await.unwrap();
            }
            
            // Check that we're in the expected stage
            let (is_throttled, current_stage, _) = module.get_throttle_status(ip).await;
            assert!(is_throttled);
            assert_eq!(current_stage, stage + 1);
        }
    }

    #[tokio::test]
    async fn test_disabled_module() {
        let mut config = create_test_config();
        config.enabled = false;
        let module = ThrottlingModule::new(config);
        let ip = IpAddr::from_str("127.0.0.1").unwrap();

        // All connections should be allowed when disabled
        for _ in 0..10 {
            assert!(module.check_connection_allowed(ip).await.unwrap());
        }
    }
}
