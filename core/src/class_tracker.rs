//! Connection class tracker for enforcing per-class limits
//!
//! This module tracks active connections per class and enforces limits like:
//! - max_clients per class
//! - max_connections_per_ip per class
//! - max_connections_per_host per class

use crate::{Config, Error, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};

/// Tracks active connections for each class
#[derive(Debug, Clone)]
pub struct ClassTracker {
    /// Shared state
    state: Arc<RwLock<ClassTrackerState>>,
}

#[derive(Debug)]
struct ClassTrackerState {
    /// Configuration reference
    config: Config,
    /// Count of clients per class
    clients_per_class: HashMap<String, usize>,
    /// Count of connections per IP per class
    connections_per_ip: HashMap<String, HashMap<IpAddr, usize>>,
    /// Count of connections per hostname per class
    connections_per_host: HashMap<String, HashMap<String, usize>>,
}

impl ClassTracker {
    /// Create a new class tracker with configuration
    pub fn new(config: Config) -> Self {
        Self {
            state: Arc::new(RwLock::new(ClassTrackerState {
                config,
                clients_per_class: HashMap::new(),
                connections_per_ip: HashMap::new(),
                connections_per_host: HashMap::new(),
            })),
        }
    }

    /// Check if a new connection can be accepted for the given class
    pub fn can_accept_connection(
        &self,
        class_name: &str,
        ip: IpAddr,
        hostname: &str,
    ) -> Result<()> {
        let state = self.state.read()
            .map_err(|_| Error::Generic("Failed to acquire read lock".to_string()))?;

        // Get the connection class
        let class = state.config.get_class(class_name)
            .ok_or_else(|| Error::Config(format!("Unknown class: {}", class_name)))?;

        // Check max_clients for this class
        if let Some(max_clients) = class.max_clients {
            let current_clients = state.clients_per_class.get(class_name).unwrap_or(&0);
            if *current_clients >= max_clients {
                return Err(Error::Connection(format!(
                    "Class {} has reached maximum clients ({}/{})",
                    class_name, current_clients, max_clients
                )));
            }
        }

        // Check max_connections_per_ip for this class
        let max_per_ip = class.max_connections_per_ip
            .or(Some(state.config.connection.max_connections_per_ip))
            .unwrap_or(5);

        if let Some(class_ips) = state.connections_per_ip.get(class_name) {
            let current_ip_count = class_ips.get(&ip).unwrap_or(&0);
            if *current_ip_count >= max_per_ip {
                return Err(Error::Connection(format!(
                    "IP {} has reached maximum connections for class {} ({}/{})",
                    ip, class_name, current_ip_count, max_per_ip
                )));
            }
        }

        // Check max_connections_per_host for this class
        let max_per_host = class.max_connections_per_host
            .or(Some(state.config.connection.max_connections_per_host))
            .unwrap_or(10);

        if let Some(class_hosts) = state.connections_per_host.get(class_name) {
            let current_host_count = class_hosts.get(hostname).unwrap_or(&0);
            if *current_host_count >= max_per_host {
                return Err(Error::Connection(format!(
                    "Host {} has reached maximum connections for class {} ({}/{})",
                    hostname, class_name, current_host_count, max_per_host
                )));
            }
        }

        Ok(())
    }

    /// Register a new connection for tracking
    pub fn register_connection(
        &self,
        class_name: &str,
        ip: IpAddr,
        hostname: &str,
    ) -> Result<()> {
        let mut state = self.state.write()
            .map_err(|_| Error::Generic("Failed to acquire write lock".to_string()))?;

        // Increment client count for class
        *state.clients_per_class.entry(class_name.to_string()).or_insert(0) += 1;

        // Increment IP count for class
        *state.connections_per_ip
            .entry(class_name.to_string())
            .or_insert_with(HashMap::new)
            .entry(ip)
            .or_insert(0) += 1;

        // Increment hostname count for class
        *state.connections_per_host
            .entry(class_name.to_string())
            .or_insert_with(HashMap::new)
            .entry(hostname.to_string())
            .or_insert(0) += 1;

        tracing::debug!(
            "Registered connection for class {}: IP={}, host={}, total_in_class={}",
            class_name,
            ip,
            hostname,
            state.clients_per_class.get(class_name).unwrap_or(&0)
        );

        Ok(())
    }

    /// Unregister a connection
    pub fn unregister_connection(
        &self,
        class_name: &str,
        ip: IpAddr,
        hostname: &str,
    ) -> Result<()> {
        let mut state = self.state.write()
            .map_err(|_| Error::Generic("Failed to acquire write lock".to_string()))?;

        // Decrement client count for class
        if let Some(count) = state.clients_per_class.get_mut(class_name) {
            *count = count.saturating_sub(1);
        }

        // Decrement IP count for class
        if let Some(class_ips) = state.connections_per_ip.get_mut(class_name) {
            if let Some(count) = class_ips.get_mut(&ip) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    class_ips.remove(&ip);
                }
            }
        }

        // Decrement hostname count for class
        if let Some(class_hosts) = state.connections_per_host.get_mut(class_name) {
            if let Some(count) = class_hosts.get_mut(hostname) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    class_hosts.remove(hostname);
                }
            }
        }

        tracing::debug!(
            "Unregistered connection for class {}: IP={}, host={}, remaining_in_class={}",
            class_name,
            ip,
            hostname,
            state.clients_per_class.get(class_name).unwrap_or(&0)
        );

        Ok(())
    }

    /// Get statistics for a class
    pub fn get_class_stats(&self, class_name: &str) -> Option<ClassStats> {
        let state = self.state.read().ok()?;

        Some(ClassStats {
            class_name: class_name.to_string(),
            total_clients: *state.clients_per_class.get(class_name).unwrap_or(&0),
            unique_ips: state.connections_per_ip
                .get(class_name)
                .map(|m| m.len())
                .unwrap_or(0),
            unique_hosts: state.connections_per_host
                .get(class_name)
                .map(|m| m.len())
                .unwrap_or(0),
        })
    }

    /// Get all class statistics
    pub fn get_all_stats(&self) -> Vec<ClassStats> {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        state.config.classes.iter().map(|class| {
            ClassStats {
                class_name: class.name.clone(),
                total_clients: *state.clients_per_class.get(&class.name).unwrap_or(&0),
                unique_ips: state.connections_per_ip
                    .get(&class.name)
                    .map(|m| m.len())
                    .unwrap_or(0),
                unique_hosts: state.connections_per_host
                    .get(&class.name)
                    .map(|m| m.len())
                    .unwrap_or(0),
            }
        }).collect()
    }

    /// Update configuration (useful for rehash)
    pub fn update_config(&self, config: Config) -> Result<()> {
        let mut state = self.state.write()
            .map_err(|_| Error::Generic("Failed to acquire write lock".to_string()))?;
        state.config = config;
        Ok(())
    }

    /// Get connection class for a host/IP combination
    pub fn get_class_for_connection(&self, host: &str, ip: &str) -> Option<String> {
        let state = self.state.read().ok()?;

        // Check if there are any allow blocks defined
        if !state.config.security.allow_blocks.is_empty() {
            // Use allow blocks to determine class
            if let Some(allow_block) = state.config.find_allow_block(host, ip) {
                return Some(allow_block.class.clone());
            }
            // If allow blocks are defined but no match, deny connection
            return None;
        }

        // Fallback: use default class if host is in allowed_hosts
        for allowed_host in &state.config.security.allowed_hosts {
            if state.config.matches_host_pattern(host, allowed_host) {
                return Some("default".to_string());
            }
        }

        None
    }

    /// Check if throttling is disabled for a class
    pub fn is_throttling_disabled(&self, class_name: &str) -> bool {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return false,
        };

        state.config.get_class(class_name)
            .map(|class| class.disable_throttling)
            .unwrap_or(false)
    }
}

/// Statistics for a connection class
#[derive(Debug, Clone)]
pub struct ClassStats {
    /// Class name
    pub class_name: String,
    /// Total number of clients in this class
    pub total_clients: usize,
    /// Number of unique IP addresses
    pub unique_ips: usize,
    /// Number of unique hostnames
    pub unique_hosts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn create_test_config() -> Config {
        let mut config = Config::default();
        
        // Add a test class with limits
        config.classes = vec![
            ConnectionClass {
                name: "test".to_string(),
                max_clients: Some(5),
                ping_frequency: Some(120),
                connection_timeout: Some(300),
                max_sendq: Some(1048576),
                max_recvq: Some(8192),
                disable_throttling: false,
                max_connections_per_ip: Some(2),
                max_connections_per_host: Some(3),
                description: Some("Test class".to_string()),
            },
        ];
        
        config
    }

    #[test]
    fn test_can_accept_connection() {
        let config = create_test_config();
        let tracker = ClassTracker::new(config);
        
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        let hostname = "test.example.com";
        
        // First connection should be accepted
        assert!(tracker.can_accept_connection("test", ip, hostname).is_ok());
    }

    #[test]
    fn test_max_clients_per_class() {
        let config = create_test_config();
        let tracker = ClassTracker::new(config);
        
        // Register 5 connections (the max)
        for i in 0..5 {
            let ip: IpAddr = format!("192.168.1.{}", i + 100).parse().unwrap();
            let hostname = format!("host{}.example.com", i);
            tracker.register_connection("test", ip, &hostname).unwrap();
        }
        
        // 6th connection should be rejected
        let ip: IpAddr = "192.168.1.200".parse().unwrap();
        assert!(tracker.can_accept_connection("test", ip, "host6.example.com").is_err());
    }

    #[test]
    fn test_max_connections_per_ip() {
        let config = create_test_config();
        let tracker = ClassTracker::new(config);
        
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        
        // Register 2 connections from same IP (the max for this class)
        tracker.register_connection("test", ip, "host1.example.com").unwrap();
        tracker.register_connection("test", ip, "host2.example.com").unwrap();
        
        // 3rd connection from same IP should be rejected
        assert!(tracker.can_accept_connection("test", ip, "host3.example.com").is_err());
    }
}

