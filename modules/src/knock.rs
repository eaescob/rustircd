//! Knock Module
//! 
//! Provides channel invitation request system.
//! Based on Ratbox's m_knock.c module.

use rustircd_core::{
    async_trait, Client, Error, Message, MessageType, Module, ModuleManager,
    ModuleNumericManager, module::{ModuleResult, ModuleStatsResponse, ModuleContext},
    NumericReply, Result, User
};
use tracing::{debug, info, warn};
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::help::{HelpProvider, HelpTopic};

/// Knock system module that handles channel invitation requests
pub struct KnockModule {
    /// Map of channel names to knock requests
    knock_requests: RwLock<HashMap<String, Vec<KnockRequest>>>,
    /// Configuration for knock system
    config: KnockConfig,
}

/// A knock request from a user to a channel
#[derive(Debug, Clone)]
pub struct KnockRequest {
    pub user_nick: String,
    pub user_ident: String,
    pub user_host: String,
    pub channel: String,
    pub reason: String,
    pub timestamp: u64,
}

/// Configuration for the knock system
#[derive(Debug, Clone)]
pub struct KnockConfig {
    /// Maximum number of knock requests per user per channel
    pub max_knocks_per_channel: usize,
    /// Time window for knock rate limiting (in seconds)
    pub knock_time_window: u64,
    /// Maximum knock requests per time window
    pub max_knocks_per_window: usize,
    /// Whether to allow knock requests to channels with +i mode
    pub allow_invite_only_knocks: bool,
    /// Whether to allow knock requests to channels with +k mode
    pub allow_key_knocks: bool,
}

impl Default for KnockConfig {
    fn default() -> Self {
        Self {
            max_knocks_per_channel: 3,
            knock_time_window: 300, // 5 minutes
            max_knocks_per_window: 5,
            allow_invite_only_knocks: true,
            allow_key_knocks: true,
        }
    }
}

impl KnockModule {
    /// Create a new knock module with default configuration
    pub fn new() -> Self {
        Self {
            knock_requests: RwLock::new(HashMap::new()),
            config: KnockConfig::default(),
        }
    }
    
    /// Create a new knock module with custom configuration
    pub fn with_config(config: KnockConfig) -> Self {
        Self {
            knock_requests: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// Handle KNOCK command
    async fn handle_knock(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if args.len() < 2 {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["KNOCK", "Not enough parameters"])?;
            return Ok(());
        }
        
        let channel = &args[0];
        let reason = if args.len() > 2 {
            args[1..].join(" ")
        } else {
            args[1].clone()
        };
        
        // Validate channel name
        if !self.is_valid_channel_name(channel) {
            client.send_numeric(NumericReply::ErrNoSuchChannel, &[channel, "Invalid channel name"])?;
            return Ok(());
        }
        
        // Check if user is already in the channel
        if self.is_user_in_channel(user, channel).await? {
            client.send_numeric(NumericReply::ErrUserOnChannel, &[user.nickname(), channel, "You are already on that channel"])?;
            return Ok(());
        }
        
        // Check rate limiting
        if !self.check_knock_rate_limit(user, channel).await? {
            client.send_numeric(NumericReply::ErrTooManyTargets, &[channel, "You have knocked too many times recently"])?;
            return Ok(());
        }
        
        // Create knock request
        let knock_request = KnockRequest {
            user_nick: user.nickname().to_string(),
            user_ident: user.username().to_string(),
            user_host: user.hostname().to_string(),
            channel: channel.to_string(),
            reason,
            timestamp: self.get_current_timestamp(),
        };
        
        // Store knock request
        self.store_knock_request(&knock_request).await?;
        
        // Send knock notification to channel operators
        self.notify_channel_operators(&knock_request).await?;
        
        // Send confirmation to user
        client.send_numeric(NumericReply::RplKnock, &[channel, "Your knock has been delivered"])?;
        
        info!("Knock request from {} to {}: {}", user.nickname(), channel, knock_request.reason);
        
        Ok(())
    }
    
    /// Check if a channel name is valid
    fn is_valid_channel_name(&self, channel: &str) -> bool {
        if channel.is_empty() || channel.len() > 50 {
            return false;
        }
        
        // Check for valid channel prefix
        if !channel.starts_with('#') && !channel.starts_with('&') && !channel.starts_with('+') && !channel.starts_with('!') {
            return false;
        }
        
        // Check for invalid characters
        for ch in channel.chars() {
            if ch == ' ' || ch == ',' || ch == 7 as char { // space, comma, or bell
                return false;
            }
        }
        
        true
    }
    
    /// Check if user is already in the channel
    async fn is_user_in_channel(&self, user: &User, channel: &str) -> Result<bool> {
        // TODO: Implement channel membership checking
        // This would need access to the channel module or database
        // For now, return false (user not in channel)
        Ok(false)
    }
    
    /// Check knock rate limiting
    async fn check_knock_rate_limit(&self, user: &User, channel: &str) -> Result<bool> {
        let knock_requests = self.knock_requests.read().await;
        let current_time = self.get_current_timestamp();
        
        // Count knocks to this specific channel
        if let Some(requests) = knock_requests.get(channel) {
            let user_knocks: Vec<&KnockRequest> = requests
                .iter()
                .filter(|req| req.user_nick == user.nickname())
                .collect();
            
            if user_knocks.len() >= self.config.max_knocks_per_channel {
                return Ok(false);
            }
        }
        
        // Count total knocks in time window
        let mut total_knocks = 0;
        for requests in knock_requests.values() {
            let user_knocks: Vec<&KnockRequest> = requests
                .iter()
                .filter(|req| {
                    req.user_nick == user.nickname() && 
                    (current_time - req.timestamp) <= self.config.knock_time_window
                })
                .collect();
            
            total_knocks += user_knocks.len();
        }
        
        Ok(total_knocks < self.config.max_knocks_per_window)
    }
    
    /// Store a knock request
    async fn store_knock_request(&self, request: &KnockRequest) -> Result<()> {
        let mut knock_requests = self.knock_requests.write().await;
        
        knock_requests
            .entry(request.channel.clone())
            .or_insert_with(Vec::new)
            .push(request.clone());
        
        // Clean up old requests
        self.cleanup_old_requests(&mut knock_requests).await;
        
        Ok(())
    }
    
    /// Clean up old knock requests
    async fn cleanup_old_requests(&self, knock_requests: &mut HashMap<String, Vec<KnockRequest>>) {
        let current_time = self.get_current_timestamp();
        
        for requests in knock_requests.values_mut() {
            requests.retain(|req| (current_time - req.timestamp) <= self.config.knock_time_window);
        }
        
        // Remove empty channel entries
        knock_requests.retain(|_, requests| !requests.is_empty());
    }
    
    /// Notify channel operators about knock request
    async fn notify_channel_operators(&self, request: &KnockRequest) -> Result<()> {
        // TODO: Implement channel operator notification
        // This would need access to the channel module to get operator list
        // and the broadcast system to send messages
        
        debug!("Knock notification for {}: {} ({}) wants to join {}: {}", 
               request.channel, request.user_nick, request.user_host, 
               request.channel, request.reason);
        
        // For now, just log the knock request
        // In a real implementation, this would send a message to channel operators
        
        Ok(())
    }
    
    /// Get current timestamp
    fn get_current_timestamp(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Get knock requests for a channel
    pub async fn get_knock_requests(&self, channel: &str) -> Vec<KnockRequest> {
        let knock_requests = self.knock_requests.read().await;
        knock_requests
            .get(channel)
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }
    
    /// Clear knock requests for a channel
    pub async fn clear_knock_requests(&self, channel: &str) -> Result<()> {
        let mut knock_requests = self.knock_requests.write().await;
        knock_requests.remove(channel);
        Ok(())
    }
    
    /// Clear knock requests for a specific user
    pub async fn clear_user_knock_requests(&self, user_nick: &str) -> Result<()> {
        let mut knock_requests = self.knock_requests.write().await;
        
        for requests in knock_requests.values_mut() {
            requests.retain(|req| req.user_nick != user_nick);
        }
        
        // Remove empty channel entries
        knock_requests.retain(|_, requests| !requests.is_empty());
        
        Ok(())
    }
}

#[async_trait]
impl Module for KnockModule {
    fn name(&self) -> &str {
        "knock"
    }
    
    fn description(&self) -> &str {
        "Provides channel invitation request system"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("{} module initialized", self.name());
        Ok(())
    }

    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled),
        };

        match message.command {
            MessageType::Custom(ref cmd) if cmd == "KNOCK" => {
                self.handle_knock(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_server_message(&mut self, _server: &str, _message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }

    async fn handle_user_registration(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }

    async fn handle_user_disconnection(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["message_handler".to_string()]
    }

    fn supports_capability(&self, capability: &str) -> bool {
        capability == "message_handler"
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

    async fn handle_stats_query(&mut self, _query: &str, _client_id: uuid::Uuid, _server: Option<&rustircd_core::Server>) -> Result<Vec<ModuleStatsResponse>> {
        Ok(vec![])
    }

    fn get_stats_queries(&self) -> Vec<String> {
        vec![]
    }

    fn register_numerics(&self, _manager: &mut ModuleNumericManager) -> Result<()> {
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("Knock module cleaned up");
        Ok(())
    }
}

impl Default for KnockModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpProvider for KnockModule {
    fn get_help_topics(&self) -> Vec<HelpTopic> {
        vec![
            HelpTopic {
                command: "KNOCK".to_string(),
                syntax: "KNOCK <channel> <reason>".to_string(),
                description: "Request invitation to a channel".to_string(),
                oper_only: false,
                examples: vec![
                    "KNOCK #rust Please let me in".to_string(),
                    "KNOCK #secret I'm a friend of alice".to_string(),
                ],
                module_name: Some("knock".to_string()),
            },
        ]
    }
    
    fn get_command_help(&self, command: &str) -> Option<HelpTopic> {
        if command == "KNOCK" {
            Some(HelpTopic {
                command: "KNOCK".to_string(),
                syntax: "KNOCK <channel> <reason>".to_string(),
                description: "Request invitation to a channel".to_string(),
                oper_only: false,
                examples: vec![
                    "KNOCK #rust Please let me in".to_string(),
                    "KNOCK #secret I'm a friend of alice".to_string(),
                ],
                module_name: Some("knock".to_string()),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_knock_config_default() {
        let config = KnockConfig::default();
        assert_eq!(config.max_knocks_per_channel, 3);
        assert_eq!(config.knock_time_window, 300);
        assert_eq!(config.max_knocks_per_window, 5);
        assert!(config.allow_invite_only_knocks);
        assert!(config.allow_key_knocks);
    }
    
    #[test]
    fn test_knock_module_creation() {
        let module = KnockModule::new();
        assert_eq!(module.config.max_knocks_per_channel, 3);
    }
    
    #[test]
    fn test_valid_channel_names() {
        let module = KnockModule::new();
        
        assert!(module.is_valid_channel_name("#rust"));
        assert!(module.is_valid_channel_name("&local"));
        assert!(module.is_valid_channel_name("+public"));
        assert!(module.is_valid_channel_name("!secure"));
        
        assert!(!module.is_valid_channel_name(""));
        assert!(!module.is_valid_channel_name("rust")); // No prefix
        assert!(!module.is_valid_channel_name("#rust,programming")); // Contains comma
        assert!(!module.is_valid_channel_name("#rust programming")); // Contains space
    }
    
    #[tokio::test]
    async fn test_knock_request_storage() {
        let module = KnockModule::new();
        
        let request = KnockRequest {
            user_nick: "alice".to_string(),
            user_ident: "alice".to_string(),
            user_host: "user@host.com".to_string(),
            channel: "#rust".to_string(),
            reason: "Please let me in".to_string(),
            timestamp: module.get_current_timestamp(),
        };
        
        module.store_knock_request(&request).await.unwrap();
        
        let requests = module.get_knock_requests("#rust").await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].user_nick, "alice");
        assert_eq!(requests[0].reason, "Please let me in");
    }
    
    #[tokio::test]
    async fn test_clear_knock_requests() {
        let module = KnockModule::new();
        
        let request = KnockRequest {
            user_nick: "alice".to_string(),
            user_ident: "alice".to_string(),
            user_host: "user@host.com".to_string(),
            channel: "#rust".to_string(),
            reason: "Please let me in".to_string(),
            timestamp: module.get_current_timestamp(),
        };
        
        module.store_knock_request(&request).await.unwrap();
        assert!(!module.get_knock_requests("#rust").await.is_empty());
        
        module.clear_knock_requests("#rust").await.unwrap();
        assert!(module.get_knock_requests("#rust").await.is_empty());
    }
}
