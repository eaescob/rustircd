//! Message batching optimizer for improved network performance
//!
//! This module provides batching capabilities to reduce the number of network writes
//! by combining multiple messages to the same target into a single transmission.

use crate::{Message, Error, Result};
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Configuration for message batching
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum messages per batch
    pub max_batch_size: usize,
    /// Maximum time to wait before flushing batch (ms)
    pub max_batch_delay: Duration,
    /// Maximum total size of batched messages (bytes)
    pub max_batch_bytes: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 50,
            max_batch_delay: Duration::from_millis(10),
            max_batch_bytes: 4096,
        }
    }
}

/// A batch of messages for a single target
#[derive(Debug)]
pub struct MessageBatch {
    pub target_id: Uuid,
    pub messages: VecDeque<Message>,
    pub created_at: Instant,
    pub total_bytes: usize,
}

impl MessageBatch {
    pub fn new(target_id: Uuid) -> Self {
        Self {
            target_id,
            messages: VecDeque::new(),
            created_at: Instant::now(),
            total_bytes: 0,
        }
    }

    /// Add a message to the batch
    pub fn add_message(&mut self, message: Message) {
        let message_size = message.to_string().len();
        self.total_bytes += message_size;
        self.messages.push_back(message);
    }

    /// Check if batch should be flushed
    pub fn should_flush(&self, config: &BatchConfig) -> bool {
        self.messages.len() >= config.max_batch_size
            || self.total_bytes >= config.max_batch_bytes
            || self.created_at.elapsed() >= config.max_batch_delay
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the number of messages in the batch
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Combine all messages into a single string for transmission
    pub fn combine_messages(&self) -> String {
        self.messages.iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Message batching optimizer
pub struct BatchOptimizer {
    /// Pending batches per target
    batches: DashMap<Uuid, Arc<RwLock<MessageBatch>>>,
    /// Configuration
    config: BatchConfig,
    /// Statistics
    stats: Arc<RwLock<BatchStats>>,
}

/// Statistics for batching operations
#[derive(Debug, Default, Clone)]
pub struct BatchStats {
    pub total_messages_batched: u64,
    pub total_batches_sent: u64,
    pub total_bytes_saved: u64,
    pub average_batch_size: f64,
}

impl BatchOptimizer {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            batches: DashMap::new(),
            config,
            stats: Arc::new(RwLock::new(BatchStats::default())),
        }
    }

    /// Add a message to be batched
    pub async fn add_message(&self, target_id: Uuid, message: Message) -> Result<()> {
        // Get or create batch for this target
        let batch = self.batches.entry(target_id)
            .or_insert_with(|| Arc::new(RwLock::new(MessageBatch::new(target_id))))
            .clone();

        let mut batch_guard = batch.write().await;
        batch_guard.add_message(message);

        Ok(())
    }

    /// Check if any batches should be flushed
    pub async fn get_ready_batches(&self) -> Vec<(Uuid, String)> {
        let mut ready = Vec::new();

        // Check all batches
        for entry in self.batches.iter() {
            let target_id = *entry.key();
            let batch = entry.value().clone();
            
            let mut batch_guard = batch.write().await;
            if batch_guard.should_flush(&self.config) {
                let combined = batch_guard.combine_messages();
                let message_count = batch_guard.len();
                
                // Update statistics
                let mut stats = self.stats.write().await;
                stats.total_messages_batched += message_count as u64;
                stats.total_batches_sent += 1;
                
                // Calculate average batch size
                stats.average_batch_size = 
                    (stats.total_messages_batched as f64) / (stats.total_batches_sent as f64);

                ready.push((target_id, combined));
                
                // Clear the batch
                batch_guard.messages.clear();
                batch_guard.total_bytes = 0;
                batch_guard.created_at = Instant::now();
            }
        }

        ready
    }

    /// Force flush a specific target's batch
    pub async fn flush_target(&self, target_id: &Uuid) -> Option<String> {
        if let Some(batch) = self.batches.get(target_id) {
            let mut batch_guard = batch.write().await;
            if !batch_guard.is_empty() {
                let combined = batch_guard.combine_messages();
                let message_count = batch_guard.len();
                
                // Update statistics
                let mut stats = self.stats.write().await;
                stats.total_messages_batched += message_count as u64;
                stats.total_batches_sent += 1;
                stats.average_batch_size = 
                    (stats.total_messages_batched as f64) / (stats.total_batches_sent as f64);

                batch_guard.messages.clear();
                batch_guard.total_bytes = 0;
                batch_guard.created_at = Instant::now();
                
                return Some(combined);
            }
        }
        None
    }

    /// Force flush all batches
    pub async fn flush_all(&self) -> Vec<(Uuid, String)> {
        let mut flushed = Vec::new();

        for entry in self.batches.iter() {
            let target_id = *entry.key();
            let batch = entry.value().clone();
            
            let mut batch_guard = batch.write().await;
            if !batch_guard.is_empty() {
                let combined = batch_guard.combine_messages();
                let message_count = batch_guard.len();
                
                // Update statistics
                let mut stats = self.stats.write().await;
                stats.total_messages_batched += message_count as u64;
                stats.total_batches_sent += 1;
                stats.average_batch_size = 
                    (stats.total_messages_batched as f64) / (stats.total_batches_sent as f64);

                flushed.push((target_id, combined));
                
                batch_guard.messages.clear();
                batch_guard.total_bytes = 0;
                batch_guard.created_at = Instant::now();
            }
        }

        flushed
    }

    /// Remove a target's batch (e.g., when client disconnects)
    pub fn remove_target(&self, target_id: &Uuid) {
        self.batches.remove(target_id);
    }

    /// Get batching statistics
    pub async fn stats(&self) -> BatchStats {
        self.stats.read().await.clone()
    }

    /// Clear all statistics
    pub async fn clear_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = BatchStats::default();
    }
}

/// Connection pool for server-to-server connections
pub struct ConnectionPool {
    /// Active connections
    connections: DashMap<String, Vec<Uuid>>,
    /// Connection statistics
    stats: Arc<RwLock<ConnectionPoolStats>>,
    /// Maximum connections per server
    max_connections_per_server: usize,
}

#[derive(Debug, Default, Clone)]
pub struct ConnectionPoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub connections_created: u64,
    pub connections_reused: u64,
}

impl ConnectionPool {
    pub fn new(max_connections_per_server: usize) -> Self {
        Self {
            connections: DashMap::new(),
            stats: Arc::new(RwLock::new(ConnectionPoolStats::default())),
            max_connections_per_server,
        }
    }

    /// Get a connection for a server (or indicate a new one is needed)
    pub async fn get_connection(&self, server: &str) -> Option<Uuid> {
        if let Some(connections) = self.connections.get(server) {
            if let Some(&conn_id) = connections.first() {
                let mut stats = self.stats.write().await;
                stats.connections_reused += 1;
                return Some(conn_id);
            }
        }
        None
    }

    /// Add a connection to the pool
    pub async fn add_connection(&self, server: String, connection_id: Uuid) -> Result<()> {
        let mut connections = self.connections.entry(server.clone())
            .or_insert_with(Vec::new);
        
        if connections.len() >= self.max_connections_per_server {
            return Err(Error::User(format!(
                "Maximum connections reached for server {}",
                server
            )));
        }

        connections.push(connection_id);
        
        let mut stats = self.stats.write().await;
        stats.total_connections += 1;
        stats.active_connections += 1;
        stats.connections_created += 1;

        Ok(())
    }

    /// Remove a connection from the pool
    pub async fn remove_connection(&self, server: &str, connection_id: &Uuid) {
        if let Some(mut connections) = self.connections.get_mut(server) {
            connections.retain(|id| id != connection_id);
            
            let mut stats = self.stats.write().await;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }
    }

    /// Get pool statistics
    pub async fn stats(&self) -> ConnectionPoolStats {
        let mut stats = self.stats.read().await.clone();
        stats.total_connections = self.connections.iter()
            .map(|entry| entry.value().len())
            .sum();
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageType;

    #[tokio::test]
    async fn test_message_batching() {
        let config = BatchConfig {
            max_batch_size: 3,
            max_batch_delay: Duration::from_millis(100),
            max_batch_bytes: 1000,
        };
        let optimizer = BatchOptimizer::new(config);
        
        let target_id = Uuid::new_v4();
        let msg1 = Message::new(MessageType::PrivMsg, vec!["#test".to_string(), "Hello".to_string()]);
        let msg2 = Message::new(MessageType::PrivMsg, vec!["#test".to_string(), "World".to_string()]);
        let msg3 = Message::new(MessageType::PrivMsg, vec!["#test".to_string(), "!".to_string()]);
        
        optimizer.add_message(target_id, msg1).await.unwrap();
        optimizer.add_message(target_id, msg2).await.unwrap();
        optimizer.add_message(target_id, msg3).await.unwrap();
        
        let ready = optimizer.get_ready_batches().await;
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, target_id);
    }

    #[tokio::test]
    async fn test_connection_pool() {
        let pool = ConnectionPool::new(5);
        let server = "test.server".to_string();
        let conn_id = Uuid::new_v4();
        
        pool.add_connection(server.clone(), conn_id).await.unwrap();
        
        let retrieved = pool.get_connection(&server).await;
        assert_eq!(retrieved, Some(conn_id));
        
        let stats = pool.stats().await;
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.connections_created, 1);
    }

    #[test]
    fn test_message_batch() {
        let mut batch = MessageBatch::new(Uuid::new_v4());
        let msg = Message::new(MessageType::Ping, vec!["server".to_string()]);
        
        batch.add_message(msg);
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
        
        let combined = batch.combine_messages();
        assert!(combined.contains("PING"));
    }
}






