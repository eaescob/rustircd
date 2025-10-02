//! Statistics tracking system for IRC server

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
// Remove unused tracing import when not needed

/// Statistics data for the server
#[derive(Debug, Clone)]
pub struct ServerStatistics {
    /// Server start time
    pub start_time: Instant,
    /// Total connections accepted
    pub total_connections: u64,
    /// Total messages received
    pub total_messages_received: u64,
    /// Total messages sent
    pub total_messages_sent: u64,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Command usage statistics
    pub command_usage: HashMap<String, u64>,
    /// Current number of connected clients
    pub current_clients: u32,
    /// Current number of connected servers
    pub current_servers: u32,
    /// Current number of channels
    pub current_channels: u32,
}

impl Default for ServerStatistics {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            total_connections: 0,
            total_messages_received: 0,
            total_messages_sent: 0,
            total_bytes_received: 0,
            total_bytes_sent: 0,
            command_usage: HashMap::new(),
            current_clients: 0,
            current_servers: 0,
            current_channels: 0,
        }
    }
}

impl ServerStatistics {
    /// Create new statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Get server uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Record a new connection
    pub fn record_connection(&mut self) {
        self.total_connections += 1;
        self.current_clients += 1;
    }

    /// Record a connection disconnect
    pub fn record_disconnection(&mut self) {
        if self.current_clients > 0 {
            self.current_clients -= 1;
        }
    }

    /// Record a server connection
    pub fn record_server_connection(&mut self) {
        self.current_servers += 1;
    }

    /// Record a server disconnection
    pub fn record_server_disconnection(&mut self) {
        if self.current_servers > 0 {
            self.current_servers -= 1;
        }
    }

    /// Record a message received
    pub fn record_message_received(&mut self, command: &str, bytes: usize) {
        self.total_messages_received += 1;
        self.total_bytes_received += bytes as u64;
        
        // Track command usage
        *self.command_usage.entry(command.to_string()).or_insert(0) += 1;
    }

    /// Record a message sent
    pub fn record_message_sent(&mut self, bytes: usize) {
        self.total_messages_sent += 1;
        self.total_bytes_sent += bytes as u64;
    }

    /// Update channel count
    pub fn set_channel_count(&mut self, count: u32) {
        self.current_channels = count;
    }

    /// Get command usage statistics
    pub fn get_command_stats(&self) -> &HashMap<String, u64> {
        &self.command_usage
    }

    /// Get top commands by usage
    pub fn get_top_commands(&self, limit: usize) -> Vec<(String, u64)> {
        let mut commands: Vec<_> = self.command_usage.iter().collect();
        commands.sort_by(|a, b| b.1.cmp(a.1));
        commands.truncate(limit);
        commands.into_iter().map(|(k, v)| (k.clone(), *v)).collect()
    }
}

/// Statistics manager for tracking server statistics
pub struct StatisticsManager {
    /// Main server statistics
    statistics: Arc<RwLock<ServerStatistics>>,
    /// Module-specific statistics
    module_statistics: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
}

impl StatisticsManager {
    /// Create a new statistics manager
    pub fn new() -> Self {
        Self {
            statistics: Arc::new(RwLock::new(ServerStatistics::new())),
            module_statistics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a reference to the statistics
    pub fn statistics(&self) -> Arc<RwLock<ServerStatistics>> {
        self.statistics.clone()
    }

    /// Record a new connection
    pub async fn record_connection(&self) {
        let mut stats = self.statistics.write().await;
        stats.record_connection();
    }

    /// Record a connection disconnect
    pub async fn record_disconnection(&self) {
        let mut stats = self.statistics.write().await;
        stats.record_disconnection();
    }

    /// Record a server connection
    pub async fn record_server_connection(&self) {
        let mut stats = self.statistics.write().await;
        stats.record_server_connection();
    }

    /// Record a server disconnection
    pub async fn record_server_disconnection(&self) {
        let mut stats = self.statistics.write().await;
        stats.record_server_disconnection();
    }

    /// Record a message received
    pub async fn record_message_received(&self, command: &str, bytes: usize) {
        let mut stats = self.statistics.write().await;
        stats.record_message_received(command, bytes);
    }

    /// Record a message sent
    pub async fn record_message_sent(&self, bytes: usize) {
        let mut stats = self.statistics.write().await;
        stats.record_message_sent(bytes);
    }

    /// Update channel count
    pub async fn set_channel_count(&self, count: u32) {
        let mut stats = self.statistics.write().await;
        stats.set_channel_count(count);
    }

    /// Set module statistics
    pub async fn set_module_stats(&self, module: &str, stats: HashMap<String, String>) {
        let mut module_stats = self.module_statistics.write().await;
        module_stats.insert(module.to_string(), stats);
    }

    /// Get module statistics
    pub async fn get_module_stats(&self, module: &str) -> Option<HashMap<String, String>> {
        let module_stats = self.module_statistics.read().await;
        module_stats.get(module).cloned()
    }

    /// Get all module statistics
    pub async fn get_all_module_stats(&self) -> HashMap<String, HashMap<String, String>> {
        let module_stats = self.module_statistics.read().await;
        module_stats.clone()
    }

    /// Clear module statistics
    pub async fn clear_module_stats(&self, module: &str) {
        let mut module_stats = self.module_statistics.write().await;
        module_stats.remove(module);
    }
}

impl Default for StatisticsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_statistics_creation() {
        let stats = ServerStatistics::new();
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.total_messages_received, 0);
        assert_eq!(stats.total_messages_sent, 0);
        assert_eq!(stats.current_clients, 0);
        assert_eq!(stats.current_servers, 0);
        assert_eq!(stats.current_channels, 0);
    }

    #[test]
    fn test_connection_tracking() {
        let mut stats = ServerStatistics::new();
        
        // Record connections
        stats.record_connection();
        stats.record_connection();
        assert_eq!(stats.total_connections, 2);
        assert_eq!(stats.current_clients, 2);
        
        // Record disconnections
        stats.record_disconnection();
        assert_eq!(stats.current_clients, 1);
    }

    #[test]
    fn test_message_tracking() {
        let mut stats = ServerStatistics::new();
        
        stats.record_message_received("PRIVMSG", 100);
        stats.record_message_received("PRIVMSG", 50);
        stats.record_message_received("JOIN", 30);
        
        assert_eq!(stats.total_messages_received, 3);
        assert_eq!(stats.total_bytes_received, 180);
        assert_eq!(stats.command_usage.get("PRIVMSG"), Some(&2));
        assert_eq!(stats.command_usage.get("JOIN"), Some(&1));
    }

    #[test]
    fn test_command_stats() {
        let mut stats = ServerStatistics::new();
        
        stats.record_message_received("PRIVMSG", 100);
        stats.record_message_received("PRIVMSG", 100);
        stats.record_message_received("JOIN", 50);
        stats.record_message_received("PART", 50);
        
        let top_commands = stats.get_top_commands(2);
        assert_eq!(top_commands.len(), 2);
        assert_eq!(top_commands[0].0, "PRIVMSG");
        assert_eq!(top_commands[0].1, 2);
    }

    #[tokio::test]
    async fn test_statistics_manager() {
        let manager = StatisticsManager::new();
        
        manager.record_connection().await;
        manager.record_message_received("TEST", 100).await;
        
        let stats = manager.statistics().read().await;
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.total_messages_received, 1);
        assert_eq!(stats.total_bytes_received, 100);
    }

    #[tokio::test]
    async fn test_module_statistics() {
        let manager = StatisticsManager::new();
        
        let mut module_stats = HashMap::new();
        module_stats.insert("key1".to_string(), "value1".to_string());
        module_stats.insert("key2".to_string(), "value2".to_string());
        
        manager.set_module_stats("test_module", module_stats).await;
        
        let retrieved = manager.get_module_stats("test_module").await.unwrap();
        assert_eq!(retrieved.get("key1"), Some(&"value1".to_string()));
        assert_eq!(retrieved.get("key2"), Some(&"value2".to_string()));
    }
}
