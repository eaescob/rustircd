//! Efficient message broadcasting system for IRC daemon

use crate::{Message, User, Error, Result, Client};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use dashmap::DashMap;

/// Broadcast target types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BroadcastTarget {
    /// All users on the server
    AllUsers,
    /// All users in a specific channel
    Channel(String),
    /// All users except the sender
    AllExcept(Uuid),
    /// Specific users by nickname
    Users(Vec<String>),
    /// All servers in the network
    AllServers,
    /// Specific servers
    Servers(Vec<String>),
    /// All operators
    Operators,
    /// Users matching a pattern
    Pattern(String),
}

/// Broadcast message with metadata
#[derive(Debug, Clone)]
pub struct BroadcastMessage {
    pub message: Message,
    pub target: BroadcastTarget,
    pub sender: Option<Uuid>,
    pub priority: BroadcastPriority,
}

/// Message priority for queuing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BroadcastPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Efficient broadcasting system
#[derive(Debug)]
pub struct BroadcastSystem {
    /// Message queue for different priorities
    message_queues: [VecDeque<BroadcastMessage>; 4],
    /// User subscriptions (user_id -> set of channels)
    user_subscriptions: DashMap<Uuid, HashSet<String>>,
    /// Channel subscriptions (channel -> set of user_ids)
    channel_subscriptions: DashMap<String, HashSet<Uuid>>,
    /// Server connections
    server_connections: DashMap<String, Uuid>,
    /// Client connections
    client_connections: DashMap<Uuid, Arc<Client>>,
    /// Broadcast statistics
    stats: Arc<RwLock<BroadcastStats>>,
}

/// Broadcasting statistics
#[derive(Debug, Default, Clone)]
pub struct BroadcastStats {
    pub messages_sent: u64,
    pub users_reached: u64,
    pub servers_reached: u64,
    pub channels_broadcasted: u64,
    pub errors: u64,
}

impl BroadcastSystem {
    /// Create a new broadcast system
    pub fn new() -> Self {
        Self {
            message_queues: [
                VecDeque::new(), // Low
                VecDeque::new(), // Normal
                VecDeque::new(), // High
                VecDeque::new(), // Critical
            ],
            user_subscriptions: DashMap::new(),
            channel_subscriptions: DashMap::new(),
            server_connections: DashMap::new(),
            client_connections: DashMap::new(),
            stats: Arc::new(RwLock::new(BroadcastStats::default())),
        }
    }

    /// Register a client connection
    pub fn register_client(&self, client_id: Uuid, client: Arc<Client>) {
        self.client_connections.insert(client_id, client);
    }

    /// Unregister a client connection
    pub fn unregister_client(&self, client_id: &Uuid) {
        self.client_connections.remove(client_id);
        self.user_subscriptions.remove(client_id);
    }

    /// Register a server connection
    pub fn register_server(&self, server_name: String, connection_id: Uuid) {
        self.server_connections.insert(server_name, connection_id);
    }

    /// Unregister a server connection
    pub fn unregister_server(&self, server_name: &str) {
        self.server_connections.remove(server_name);
    }

    /// Subscribe user to a channel
    pub fn subscribe_to_channel(&self, user_id: Uuid, channel: String) {
        self.user_subscriptions.entry(user_id).or_insert_with(HashSet::new)
            .insert(channel.clone());
        self.channel_subscriptions.entry(channel).or_insert_with(HashSet::new)
            .insert(user_id);
    }

    /// Unsubscribe user from a channel
    pub fn unsubscribe_from_channel(&self, user_id: &Uuid, channel: &str) {
        if let Some(mut channels) = self.user_subscriptions.get_mut(user_id) {
            channels.remove(channel);
        }
        if let Some(mut users) = self.channel_subscriptions.get_mut(channel) {
            users.remove(user_id);
        }
    }

    /// Queue a message for broadcasting
    pub fn queue_message(&mut self, broadcast: BroadcastMessage) -> Result<()> {
        let priority = broadcast.priority as usize;
        if priority < self.message_queues.len() {
            self.message_queues[priority].push_back(broadcast);
            Ok(())
        } else {
            Err(Error::User("Invalid priority".to_string()))
        }
    }

    /// Process all queued messages
    pub async fn process_queues(&mut self) -> Result<()> {
        // Process queues in priority order (Critical -> High -> Normal -> Low)
        for i in (0..self.message_queues.len()).rev() {
            while let Some(broadcast) = self.message_queues[i].pop_front() {
                self.broadcast_message(broadcast).await?;
            }
        }
        Ok(())
    }

    /// Broadcast a message immediately
    pub async fn broadcast_message(&self, broadcast: BroadcastMessage) -> Result<()> {
        let targets = self.resolve_targets(&broadcast.target).await?;
        let mut success_count = 0;
        let mut error_count = 0;

        for target_id in targets {
            match self.send_to_target(target_id, &broadcast.message).await {
                Ok(_) => success_count += 1,
                Err(_) => error_count += 1,
            }
        }

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.messages_sent += 1;
        stats.users_reached += success_count;
        stats.errors += error_count;

        if matches!(broadcast.target, BroadcastTarget::Channel(_)) {
            stats.channels_broadcasted += 1;
        }

        Ok(())
    }

    /// Resolve broadcast targets to user IDs
    async fn resolve_targets(&self, target: &BroadcastTarget) -> Result<Vec<Uuid>> {
        match target {
            BroadcastTarget::AllUsers => {
                Ok(self.client_connections.iter().map(|entry| *entry.key()).collect())
            }
            BroadcastTarget::Channel(channel) => {
                Ok(self.channel_subscriptions.get(channel)
                    .map(|entry| entry.iter().cloned().collect())
                    .unwrap_or_default())
            }
            BroadcastTarget::AllExcept(sender_id) => {
                Ok(self.client_connections.iter()
                    .filter(|entry| *entry.key() != *sender_id)
                    .map(|entry| *entry.key())
                    .collect())
            }
            BroadcastTarget::Users(nicks) => {
                let mut user_ids = Vec::new();
                for nick in nicks {
                    // Find user by nickname (this would need access to the database)
                    // For now, we'll implement a simple lookup
                    if let Some(entry) = self.client_connections.iter().find(|entry| {
                        // This is a simplified lookup - in practice, you'd query the database
                        entry.value().nickname().map_or(false, |n| n == *nick)
                    }) {
                        user_ids.push(*entry.key());
                    }
                }
                Ok(user_ids)
            }
            BroadcastTarget::AllServers => {
                Ok(self.server_connections.iter().map(|entry| *entry.value()).collect())
            }
            BroadcastTarget::Servers(server_names) => {
                let mut server_ids = Vec::new();
                for server_name in server_names {
                    if let Some(entry) = self.server_connections.get(server_name) {
                        server_ids.push(*entry.value());
                    }
                }
                Ok(server_ids)
            }
            BroadcastTarget::Operators => {
                // Find all operator users
                Ok(self.client_connections.iter()
                    .filter(|entry| {
                        // This would need access to the database to check operator status
                        // For now, we'll return all users
                        true
                    })
                    .map(|entry| *entry.key())
                    .collect())
            }
            BroadcastTarget::Pattern(pattern) => {
                // Find users matching pattern
                Ok(self.client_connections.iter()
                    .filter(|entry| {
                        // This would need pattern matching against nicknames
                        // For now, we'll return all users
                        true
                    })
                    .map(|entry| *entry.key())
                    .collect())
            }
        }
    }

    /// Send message to a specific target
    async fn send_to_target(&self, target_id: Uuid, message: &Message) -> Result<()> {
        if let Some(client) = self.client_connections.get(&target_id) {
            client.send(message.clone())?;
        } else if self.server_connections.iter().any(|entry| *entry.value() == target_id) {
            // Send to server connection
            // This would need server-specific sending logic
            tracing::debug!("Sending to server: {:?}", message);
        }
        Ok(())
    }

    /// Get broadcasting statistics
    pub async fn get_stats(&self) -> BroadcastStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = BroadcastStats::default();
    }

    /// Get queue sizes
    pub fn get_queue_sizes(&self) -> [usize; 4] {
        [
            self.message_queues[0].len(), // Low
            self.message_queues[1].len(), // Normal
            self.message_queues[2].len(), // High
            self.message_queues[3].len(), // Critical
        ]
    }

    /// Check if queues are empty
    pub fn queues_empty(&self) -> bool {
        self.message_queues.iter().all(|queue| queue.is_empty())
    }
}

/// Helper functions for common broadcast patterns

impl BroadcastSystem {
    /// Broadcast a message to all users
    pub async fn broadcast_to_all(&self, message: Message, sender: Option<Uuid>) -> Result<()> {
        let broadcast = BroadcastMessage {
            message,
            target: BroadcastTarget::AllUsers,
            sender,
            priority: BroadcastPriority::Normal,
        };
        self.broadcast_message(broadcast).await
    }

    /// Broadcast a message to a channel
    pub async fn broadcast_to_channel(&self, channel: &str, message: Message, sender: Option<Uuid>) -> Result<()> {
        let broadcast = BroadcastMessage {
            message,
            target: BroadcastTarget::Channel(channel.to_string()),
            sender,
            priority: BroadcastPriority::Normal,
        };
        self.broadcast_message(broadcast).await
    }

    /// Broadcast a message to all users except sender
    pub async fn broadcast_to_all_except(&self, message: Message, sender: Uuid) -> Result<()> {
        let broadcast = BroadcastMessage {
            message,
            target: BroadcastTarget::AllExcept(sender),
            sender: Some(sender),
            priority: BroadcastPriority::Normal,
        };
        self.broadcast_message(broadcast).await
    }

    /// Broadcast a message to specific users
    pub async fn broadcast_to_users(&self, nicks: Vec<String>, message: Message, sender: Option<Uuid>) -> Result<()> {
        let broadcast = BroadcastMessage {
            message,
            target: BroadcastTarget::Users(nicks),
            sender,
            priority: BroadcastPriority::Normal,
        };
        self.broadcast_message(broadcast).await
    }

    /// Broadcast a message to all operators
    pub async fn broadcast_to_operators(&self, message: Message, sender: Option<Uuid>) -> Result<()> {
        let broadcast = BroadcastMessage {
            message,
            target: BroadcastTarget::Operators,
            sender,
            priority: BroadcastPriority::High,
        };
        self.broadcast_message(broadcast).await
    }

    /// Broadcast a server message to all servers
    pub async fn broadcast_to_servers(&self, message: Message) -> Result<()> {
        let broadcast = BroadcastMessage {
            message,
            target: BroadcastTarget::AllServers,
            sender: None,
            priority: BroadcastPriority::High,
        };
        self.broadcast_message(broadcast).await
    }
}

/// Message builder for common IRC messages
pub struct MessageBuilder;

impl MessageBuilder {
    /// Create a PRIVMSG message
    pub fn privmsg(target: &str, message: &str, sender: &User) -> Message {
        use crate::MessageType;
        Message::new(
            MessageType::PrivMsg,
            vec![target.to_string(), message.to_string()],
        )
    }

    /// Create a NOTICE message
    pub fn notice(target: &str, message: &str, sender: &User) -> Message {
        use crate::MessageType;
        Message::new(
            MessageType::Notice,
            vec![target.to_string(), message.to_string()],
        )
    }

    /// Create a JOIN message
    pub fn join(channel: &str, sender: &User) -> Message {
        use crate::MessageType;
        Message::new(
            MessageType::Join,
            vec![channel.to_string()],
        )
    }

    /// Create a PART message
    pub fn part(channel: &str, reason: Option<&str>, sender: &User) -> Message {
        use crate::MessageType;
        let mut params = vec![channel.to_string()];
        if let Some(reason) = reason {
            params.push(reason.to_string());
        }
        Message::new(MessageType::Part, params)
    }

    /// Create a QUIT message
    pub fn quit(reason: Option<&str>, sender: &User) -> Message {
        use crate::MessageType;
        let params = if let Some(reason) = reason {
            vec![reason.to_string()]
        } else {
            vec![]
        };
        Message::new(MessageType::Quit, params)
    }
}
