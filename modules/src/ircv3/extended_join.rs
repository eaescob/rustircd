//! IRCv3 Extended Join capability
//! 
//! This module implements the extended-join capability which allows JOIN messages
//! to include the account name and real name of the joining user.

use rustircd_core::{Client, Message, Error, Result};
use std::collections::HashMap;

/// Extended Join handler
pub struct ExtendedJoin {
    /// Track which clients have the extended-join capability enabled
    enabled_clients: HashMap<uuid::Uuid, bool>,
}

impl ExtendedJoin {
    pub fn new() -> Self {
        Self {
            enabled_clients: HashMap::new(),
        }
    }
    
    /// Enable extended join for a client
    pub fn enable_for_client(&mut self, client_id: uuid::Uuid) {
        self.enabled_clients.insert(client_id, true);
        tracing::debug!("Extended join enabled for client {}", client_id);
    }
    
    /// Disable extended join for a client
    pub fn disable_for_client(&mut self, client_id: uuid::Uuid) {
        self.enabled_clients.remove(&client_id);
        tracing::debug!("Extended join disabled for client {}", client_id);
    }
    
    /// Check if extended join is enabled for a client
    pub fn is_enabled_for_client(&self, client_id: &uuid::Uuid) -> bool {
        self.enabled_clients.get(client_id).copied().unwrap_or(false)
    }
    
    /// Create an extended JOIN message with account name and real name
    pub fn create_extended_join_message(
        &self,
        client: &Client,
        channel: &str,
        account_name: Option<&str>,
        real_name: Option<&str>,
    ) -> Result<Message> {
        let mut params = vec![channel.to_string()];
        
        // Add account name (or * if not available)
        let account = account_name.unwrap_or("*");
        params.push(account.to_string());
        
        // Add real name (or * if not available)
        let realname = real_name.unwrap_or("*");
        params.push(realname.to_string());
        
        let message = Message::with_prefix(
            rustircd_core::Prefix::User {
                nick: client.nickname().unwrap_or("unknown").to_string(),
                user: client.username().unwrap_or("unknown").to_string(),
                host: client.hostname().unwrap_or("unknown").to_string(),
            },
            rustircd_core::MessageType::Join,
            params,
        );
        
        Ok(message)
    }
    
    /// Create a standard JOIN message (fallback for clients without extended-join)
    pub fn create_standard_join_message(
        &self,
        client: &Client,
        channel: &str,
    ) -> Result<Message> {
        let message = Message::with_prefix(
            rustircd_core::Prefix::User {
                nick: client.nickname().unwrap_or("unknown").to_string(),
                user: client.username().unwrap_or("unknown").to_string(),
                host: client.hostname().unwrap_or("unknown").to_string(),
            },
            rustircd_core::MessageType::Join,
            vec![channel.to_string()],
        );
        
        Ok(message)
    }
    
    /// Handle JOIN command with extended join support
    pub async fn handle_join(
        &self,
        client: &Client,
        channel: &str,
        account_name: Option<&str>,
        real_name: Option<&str>,
    ) -> Result<Message> {
        if self.is_enabled_for_client(&client.id) {
            self.create_extended_join_message(client, channel, account_name, real_name)
        } else {
            self.create_standard_join_message(client, channel)
        }
    }
    
    /// Get account name from user data (placeholder implementation)
    /// In a real implementation, this would query the account system
    pub fn get_account_name(&self, _client: &Client) -> Option<String> {
        // TODO: Implement account lookup
        // For now, return None to indicate no account
        None
    }
    
    /// Get real name from user data
    pub fn get_real_name(&self, client: &Client) -> Option<String> {
        client.realname().map(|s| s.to_string())
    }
    
    /// Initialize the module
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing Extended Join module");
        Ok(())
    }
    
    /// Cleanup the module
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up Extended Join module");
        self.enabled_clients.clear();
        Ok(())
    }
}

impl Default for ExtendedJoin {
    fn default() -> Self {
        Self::new()
    }
}
