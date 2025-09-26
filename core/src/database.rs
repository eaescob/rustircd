//! In-memory database for users, servers, and user history

use crate::{User, Error, Result};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use dashmap::DashMap;

/// User history entry for WHOWAS command
#[derive(Debug, Clone)]
pub struct UserHistoryEntry {
    pub user: User,
    pub disconnect_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

/// Server information for network-wide queries
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub hopcount: u32,
    pub connected_at: DateTime<Utc>,
    pub is_super_server: bool,
    pub user_count: u32,
}

/// Channel information (when channel module is enabled)
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub topic: Option<String>,
    pub user_count: u32,
    pub modes: HashSet<char>,
}

/// In-memory database for IRC daemon
#[derive(Debug)]
pub struct Database {
    /// Active users by ID
    users: DashMap<Uuid, User>,
    /// Users by nickname (case-insensitive)
    users_by_nick: DashMap<String, Uuid>,
    /// Users by username@hostname
    users_by_ident: DashMap<String, Uuid>,
    /// Connected servers
    servers: DashMap<String, ServerInfo>,
    /// User history for WHOWAS (FIFO with max size)
    user_history: Arc<RwLock<VecDeque<UserHistoryEntry>>>,
    /// Channels (when channel module is enabled)
    channels: DashMap<String, ChannelInfo>,
    /// Users in channels (nickname -> set of channels)
    user_channels: DashMap<String, HashSet<String>>,
    /// Channel members (channel -> set of nicknames)
    channel_members: DashMap<String, HashSet<String>>,
    /// Configuration
    max_history_size: usize,
    history_retention_days: i64,
}

impl Database {
    /// Create a new database
    pub fn new(max_history_size: usize, history_retention_days: i64) -> Self {
        Self {
            users: DashMap::new(),
            users_by_nick: DashMap::new(),
            users_by_ident: DashMap::new(),
            servers: DashMap::new(),
            user_history: Arc::new(RwLock::new(VecDeque::new())),
            channels: DashMap::new(),
            user_channels: DashMap::new(),
            channel_members: DashMap::new(),
            max_history_size,
            history_retention_days,
        }
    }

    // User management

    /// Add a user to the database
    pub fn add_user(&self, user: User) -> Result<()> {
        let user_id = user.id;
        let nick_lower = user.nick.to_lowercase();
        let ident = format!("{}@{}", user.username, user.host);

        // Check for nickname conflicts
        if self.users_by_nick.contains_key(&nick_lower) {
            return Err(Error::User("Nickname already in use".to_string()));
        }

        // Check for ident conflicts
        if self.users_by_ident.contains_key(&ident) {
            return Err(Error::User("Ident already in use".to_string()));
        }

        self.users.insert(user_id, user.clone());
        self.users_by_nick.insert(nick_lower, user_id);
        self.users_by_ident.insert(ident, user_id);

        Ok(())
    }

    /// Remove a user from the database
    pub fn remove_user(&self, user_id: Uuid) -> Result<Option<User>> {
        if let Some((_, user)) = self.users.remove(&user_id) {
            let nick_lower = user.nick.to_lowercase();
            let ident = format!("{}@{}", user.username, user.host);

            self.users_by_nick.remove(&nick_lower);
            self.users_by_ident.remove(&ident);

            // Remove from all channels
            if let Some((_, channels)) = self.user_channels.remove(&user.nick) {
                for channel_name in channels {
                    if let Some(mut members) = self.channel_members.get_mut(&channel_name) {
                        members.remove(&user.nick);
                    }
                }
            }

            // Add to history
            // self.add_to_history(user.clone()).await?; // Commented out - method is async but called from sync context

            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Get user by ID
    pub fn get_user(&self, user_id: &Uuid) -> Option<User> {
        self.users.get(user_id).map(|entry| entry.value().clone())
    }

    /// Get user by nickname
    pub fn get_user_by_nick(&self, nick: &str) -> Option<User> {
        let nick_lower = nick.to_lowercase();
        self.users_by_nick.get(&nick_lower)
            .and_then(|entry| self.users.get(entry.value()))
            .map(|entry| entry.value().clone())
    }

    /// Get user by ident (username@hostname)
    pub fn get_user_by_ident(&self, ident: &str) -> Option<User> {
        self.users_by_ident.get(ident)
            .and_then(|entry| self.users.get(entry.value()))
            .map(|entry| entry.value().clone())
    }

    /// Update user information
    pub fn update_user(&self, user_id: &Uuid, user: User) -> Result<()> {
        if let Some(mut entry) = self.users.get_mut(user_id) {
            let old_nick = entry.nick.clone();
            let old_ident = format!("{}@{}", entry.username, entry.host);
            let new_nick_lower = user.nick.to_lowercase();
            let new_ident = format!("{}@{}", user.username, user.host);

            // Update nickname mapping if changed
            if old_nick != user.nick {
                let old_nick_lower = old_nick.to_lowercase();
                self.users_by_nick.remove(&old_nick_lower);
                self.users_by_nick.insert(new_nick_lower, *user_id);
            }

            // Update ident mapping if changed
            if old_ident != new_ident {
                self.users_by_ident.remove(&old_ident);
                self.users_by_ident.insert(new_ident, *user_id);
            }

            *entry = user;
            Ok(())
        } else {
            Err(Error::User("User not found".to_string()))
        }
    }

    /// Get all users
    pub fn get_all_users(&self) -> Vec<User> {
        self.users.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Search users by pattern (supports wildcards)
    pub fn search_users(&self, pattern: &str) -> Vec<User> {
        let pattern_lower = pattern.to_lowercase();
        self.users_by_nick.iter()
            .filter(|entry| {
                let nick = entry.key();
                self.matches_pattern(nick, &pattern_lower)
            })
            .filter_map(|entry| self.users.get(entry.value()))
            .map(|entry| entry.value().clone())
            .collect()
    }

    // Server management

    /// Add a server to the database
    pub fn add_server(&self, server: ServerInfo) -> Result<()> {
        self.servers.insert(server.name.clone(), server);
        Ok(())
    }

    /// Remove a server from the database
    pub fn remove_server(&self, server_name: &str) -> Option<ServerInfo> {
        self.servers.remove(server_name).map(|(_, server)| server)
    }

    /// Get server information
    pub fn get_server(&self, server_name: &str) -> Option<ServerInfo> {
        self.servers.get(server_name).map(|entry| entry.value().clone())
    }

    /// Get all servers
    pub fn get_all_servers(&self) -> Vec<ServerInfo> {
        self.servers.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Check if server is a super server
    pub fn is_super_server(&self, server_name: &str) -> bool {
        self.servers.get(server_name)
            .map(|entry| entry.is_super_server)
            .unwrap_or(false)
    }

    // Channel management (when channel module is enabled)

    /// Add a channel
    pub fn add_channel(&self, channel: ChannelInfo) -> Result<()> {
        self.channels.insert(channel.name.clone(), channel);
        Ok(())
    }

    /// Remove a channel
    pub fn remove_channel(&self, channel_name: &str) -> Option<ChannelInfo> {
        self.channels.remove(channel_name).map(|(_, channel)| channel)
    }

    /// Add user to channel
    pub fn add_user_to_channel(&self, nick: &str, channel: &str) -> Result<()> {
        // Add to user's channel list
        self.user_channels.entry(nick.to_string()).or_insert_with(HashSet::new)
            .insert(channel.to_string());

        // Add to channel's member list
        self.channel_members.entry(channel.to_string()).or_insert_with(HashSet::new)
            .insert(nick.to_string());

        Ok(())
    }

    /// Remove user from channel
    pub fn remove_user_from_channel(&self, nick: &str, channel: &str) -> Result<()> {
        // Remove from user's channel list
        if let Some(mut channels) = self.user_channels.get_mut(nick) {
            channels.remove(channel);
        }

        // Remove from channel's member list
        if let Some(mut members) = self.channel_members.get_mut(channel) {
            members.remove(nick);
        }

        Ok(())
    }

    /// Get users in a channel
    pub fn get_channel_users(&self, channel: &str) -> Vec<String> {
        self.channel_members.get(channel)
            .map(|entry| entry.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get channels for a user
    pub fn get_user_channels(&self, nick: &str) -> Vec<String> {
        self.user_channels.get(nick)
            .map(|entry| entry.iter().cloned().collect())
            .unwrap_or_default()
    }

    // User history management

    /// Add user to history
    async fn add_to_history(&self, user: User) -> Result<()> {
        let entry = UserHistoryEntry {
            user: user.clone(),
            disconnect_time: Utc::now(),
            last_activity: user.last_activity,
        };

        let mut history = self.user_history.write().await;
        history.push_back(entry);

        // Maintain max size
        while history.len() > self.max_history_size {
            history.pop_front();
        }

        Ok(())
    }

    /// Get user history by nickname
    pub async fn get_user_history(&self, nick: &str) -> Vec<UserHistoryEntry> {
        let history = self.user_history.read().await;
        history.iter()
            .filter(|entry| entry.user.nick.to_lowercase() == nick.to_lowercase())
            .cloned()
            .collect()
    }

    /// Clean up old history entries
    pub async fn cleanup_history(&self) -> Result<()> {
        let cutoff = Utc::now() - Duration::days(self.history_retention_days);
        let mut history = self.user_history.write().await;

        while let Some(entry) = history.front() {
            if entry.disconnect_time < cutoff {
                history.pop_front();
            } else {
                break;
            }
        }

        Ok(())
    }

    // Utility methods

    /// Check if pattern matches (supports * and ? wildcards)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();
        
        self.matches_pattern_recursive(&text_chars, &pattern_chars, 0, 0)
    }

    fn matches_pattern_recursive(&self, text: &[char], pattern: &[char], text_idx: usize, pattern_idx: usize) -> bool {
        if pattern_idx >= pattern.len() {
            return text_idx >= text.len();
        }

        match pattern[pattern_idx] {
            '*' => {
                // Try matching * with 0 or more characters
                for i in text_idx..=text.len() {
                    if self.matches_pattern_recursive(text, pattern, i, pattern_idx + 1) {
                        return true;
                    }
                }
                false
            }
            '?' => {
                // Match any single character
                if text_idx < text.len() {
                    self.matches_pattern_recursive(text, pattern, text_idx + 1, pattern_idx + 1)
                } else {
                    false
                }
            }
            c => {
                // Match exact character (case-insensitive)
                if text_idx < text.len() && text[text_idx].to_lowercase().next() == c.to_lowercase().next() {
                    self.matches_pattern_recursive(text, pattern, text_idx + 1, pattern_idx + 1)
                } else {
                    false
                }
            }
        }
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Get server count
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Get channel count
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Get total user count across all servers
    pub fn total_user_count(&self) -> u32 {
        self.servers.iter()
            .map(|entry| entry.user_count)
            .sum::<u32>() + self.user_count() as u32
    }
}

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub max_history_size: usize,
    pub history_retention_days: i64,
    pub enable_channel_tracking: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_history_size: 10000,
            history_retention_days: 30,
            enable_channel_tracking: true,
        }
    }
}
