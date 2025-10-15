//! IRCv3 Multi-Prefix capability
//! 
//! This module implements the multi-prefix capability which allows NAMES messages
//! to show multiple prefixes for users who have multiple channel modes.

use rustircd_core::{Client, Message, Result};
use std::collections::HashMap;

/// Multi-Prefix handler
pub struct MultiPrefix {
    /// Track which clients have the multi-prefix capability enabled
    enabled_clients: HashMap<uuid::Uuid, bool>,
}

impl MultiPrefix {
    pub fn new() -> Self {
        Self {
            enabled_clients: HashMap::new(),
        }
    }
    
    /// Enable multi-prefix for a client
    pub fn enable_for_client(&mut self, client_id: uuid::Uuid) {
        self.enabled_clients.insert(client_id, true);
        tracing::debug!("Multi-prefix enabled for client {}", client_id);
    }
    
    /// Disable multi-prefix for a client
    pub fn disable_for_client(&mut self, client_id: uuid::Uuid) {
        self.enabled_clients.remove(&client_id);
        tracing::debug!("Multi-prefix disabled for client {}", client_id);
    }
    
    /// Check if multi-prefix is enabled for a client
    pub fn is_enabled_for_client(&self, client_id: &uuid::Uuid) -> bool {
        self.enabled_clients.get(client_id).copied().unwrap_or(false)
    }
    
    /// Create a NAMES reply with multiple prefixes
    pub fn create_names_reply(
        &self,
        _client: &Client,
        channel: &str,
        names: &[String],
    ) -> Result<Message> {
        let names_str = names.join(" ");
        let message = Message::new(
            rustircd_core::MessageType::Custom("353".to_string()),
            vec!["*".to_string(), "=".to_string(), channel.to_string(), names_str],
        );
        Ok(message)
    }
    
    /// Create an end of NAMES reply
    pub fn create_end_of_names_reply(&self, channel: &str) -> Result<Message> {
        let message = Message::new(
            rustircd_core::MessageType::Custom("366".to_string()),
            vec!["*".to_string(), channel.to_string(), "End of /NAMES list".to_string()],
        );
        Ok(message)
    }
    
    /// Format user name with multiple prefixes based on channel modes
    pub fn format_user_with_prefixes(
        &self,
        nick: &str,
        modes: &std::collections::HashSet<char>,
    ) -> String {
        let mut result = String::new();
        
        // Add prefixes in order of precedence (highest to lowest)
        // @ = operator, + = voice, % = half-op, & = admin, ~ = founder
        if modes.contains(&'o') || modes.contains(&'O') {
            result.push('@');
        }
        if modes.contains(&'h') || modes.contains(&'H') {
            result.push('%');
        }
        if modes.contains(&'a') || modes.contains(&'A') {
            result.push('&');
        }
        if modes.contains(&'q') || modes.contains(&'Q') {
            result.push('~');
        }
        if modes.contains(&'v') || modes.contains(&'V') {
            result.push('+');
        }
        
        result.push_str(nick);
        result
    }
    
    /// Format user name with single prefix (standard behavior)
    pub fn format_user_with_single_prefix(
        &self,
        nick: &str,
        modes: &std::collections::HashSet<char>,
    ) -> String {
        let mut result = String::new();
        
        // Only show the highest priority prefix
        if modes.contains(&'o') || modes.contains(&'O') {
            result.push('@');
        } else if modes.contains(&'h') || modes.contains(&'H') {
            result.push('%');
        } else if modes.contains(&'a') || modes.contains(&'A') {
            result.push('&');
        } else if modes.contains(&'q') || modes.contains(&'Q') {
            result.push('~');
        } else if modes.contains(&'v') || modes.contains(&'V') {
            result.push('+');
        }
        
        result.push_str(nick);
        result
    }
    
    /// Process channel members and format their names based on capability
    pub fn process_channel_members(
        &self,
        client: &Client,
        members: &[(uuid::Uuid, std::collections::HashSet<char>)],
        user_database: &dyn Fn(&uuid::Uuid) -> Option<String>, // Function to get nick from user ID
    ) -> Vec<String> {
        let mut names = Vec::new();
        
        for (user_id, modes) in members {
            if let Some(nick) = user_database(user_id) {
                let formatted_name = if self.is_enabled_for_client(&client.id) {
                    self.format_user_with_prefixes(&nick, modes)
                } else {
                    self.format_user_with_single_prefix(&nick, modes)
                };
                names.push(formatted_name);
            }
        }
        
        // Sort names by prefix priority, then alphabetically
        names.sort_by(|a, b| {
            let a_prefix = self.get_prefix_priority(a);
            let b_prefix = self.get_prefix_priority(b);
            
            match a_prefix.cmp(&b_prefix) {
                std::cmp::Ordering::Equal => a.cmp(b),
                other => other,
            }
        });
        
        names
    }
    
    /// Get prefix priority for sorting (lower number = higher priority)
    fn get_prefix_priority(&self, name: &str) -> u8 {
        if name.starts_with('~') { 1 }  // founder
        else if name.starts_with('&') { 2 }  // admin
        else if name.starts_with('@') { 3 }  // operator
        else if name.starts_with('%') { 4 }  // half-op
        else if name.starts_with('+') { 5 }  // voice
        else { 6 }  // regular user
    }
    
    /// Initialize the module
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing Multi-Prefix module");
        Ok(())
    }
    
    /// Cleanup the module
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up Multi-Prefix module");
        self.enabled_clients.clear();
        Ok(())
    }
}

impl Default for MultiPrefix {
    fn default() -> Self {
        Self::new()
    }
}
