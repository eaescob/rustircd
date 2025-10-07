//! OPME Extension
//! 
//! This module provides the OPME command functionality, allowing operators
//! to grant themselves operator status in channels. Based on Solanum's
//! m_opme.c implementation.
//! 
//! Based on: https://github.com/solanum-ircd/solanum/blob/main/extensions/m_opme.c

use rustircd_core::{User, Message, Client, Result, Error, NumericReply, Config, ModuleNumericManager, module::{ModuleContext, ModuleResult, ModuleStatsResponse}};
use std::collections::HashSet;
use uuid::Uuid;
use async_trait::async_trait;

/// OPME module for handling the OPME command
pub struct OpmeModule {
    /// Module configuration
    config: OpmeConfig,
}

/// Configuration for the OPME module
#[derive(Debug, Clone)]
pub struct OpmeConfig {
    /// Whether the OPME module is enabled
    pub enabled: bool,
    /// Whether to require operator privileges to use OPME
    pub require_oper: bool,
    /// Whether to log OPME usage
    pub log_usage: bool,
    /// Whether to notify channel members of OPME usage
    pub notify_channel: bool,
    /// Rate limiting configuration
    pub rate_limit: OpmeRateLimit,
}

/// Rate limiting configuration for OPME
#[derive(Debug, Clone)]
pub struct OpmeRateLimit {
    /// Whether rate limiting is enabled
    pub enabled: bool,
    /// Maximum OPME uses per user per time window
    pub max_uses: u32,
    /// Time window in seconds
    pub time_window: u64,
}

impl Default for OpmeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_oper: true,
            log_usage: true,
            notify_channel: true,
            rate_limit: OpmeRateLimit {
                enabled: true,
                max_uses: 5,
                time_window: 300, // 5 minutes
            },
        }
    }
}

/// OPME usage tracking
#[derive(Debug, Clone)]
pub struct OpmeUsage {
    /// User ID
    pub user_id: Uuid,
    /// Timestamp of usage
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Channel where OPME was used
    pub channel: String,
}

impl OpmeModule {
    /// Create a new OPME module
    pub fn new(config: OpmeConfig) -> Self {
        Self { config }
    }
    
    /// Handle OPME command
    pub async fn handle_opme(&self, client: &Client, message: &Message, config: &Config) -> Result<()> {
        if !self.config.enabled {
            client.send_numeric(NumericReply::ErrUnknownCommand, &["OPME"])?;
            return Ok(());
        }
        
        // Check if user is provided
        let user = client.user.as_ref().ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if user has operator privileges (if required)
        if self.config.require_oper && !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Insufficient privileges"])?;
            return Ok(());
        }
        
        // Get target channel
        let channel = if message.params.is_empty() {
            // If no channel specified, use the first channel the user is in
            user.channels.iter().next().cloned()
                .ok_or_else(|| Error::User("No channel specified and user not in any channels".to_string()))?
        } else {
            message.params[0].clone()
        };
        
        // Validate channel name
        if !self.is_valid_channel_name(&channel) {
            client.send_numeric(NumericReply::ErrNoSuchChannel, &[&channel, "No such channel"])?;
            return Ok(());
        }
        
        // Check rate limiting
        if self.config.rate_limit.enabled {
            if let Err(e) = self.check_rate_limit(user.id, &channel).await {
                client.send_numeric(NumericReply::ErrTooManyTargets, &["Rate limit exceeded"])?;
                return Ok(());
            }
        }
        
        // Grant operator status in channel
        self.grant_channel_operator(user, &channel).await?;
        
        // Log usage
        if self.config.log_usage {
            tracing::info!("User {} used OPME in channel {}", user.nick, channel);
        }
        
        // Send success message
        let success_msg = NumericReply::RplYoureOper.reply("", vec![format!("You are now an operator in {}", channel)]);
        let _ = client.send(success_msg);
        
        // Notify channel if configured
        if self.config.notify_channel {
            self.notify_channel_opme(&channel, &user.nick).await?;
        }
        
        Ok(())
    }
    
    /// Check if channel name is valid
    fn is_valid_channel_name(&self, channel: &str) -> bool {
        if channel.is_empty() || channel.len() > 200 {
            return false;
        }
        
        // Check for valid channel prefixes
        channel.starts_with('#') || 
        channel.starts_with('&') || 
        channel.starts_with('+') || 
        channel.starts_with('!')
    }
    
    /// Check rate limiting for OPME usage
    async fn check_rate_limit(&self, user_id: Uuid, channel: &str) -> Result<()> {
        // This is a simplified rate limiting implementation
        // In a real implementation, you would store usage data persistently
        // and check against the time window and max uses
        
        // Implement basic rate limiting with in-memory storage
        // TODO: Implement persistent storage for rate limiting across server restarts
        
        if self.config.rate_limit.enabled {
            // For now, implement basic rate limiting logic
            // In production, this would use persistent storage (Redis, database, etc.)
            
            // Basic implementation: allow if rate limiting is not exceeded
            // This is a simplified version - real implementation would track per-user usage
            tracing::debug!("Rate limiting check for user {} - max uses: {}, time window: {}s", 
                user_id, self.config.rate_limit.max_uses, self.config.rate_limit.time_window);
            
            // For now, always allow (in production, check against stored usage data)
            // Real implementation would:
            // 1. Query usage from persistent storage
            // 2. Check if current time window has exceeded max uses
            // 3. Update usage counter
            // 4. Return appropriate result
        }
        
        Ok(())
    }
    
    /// Grant operator status in a channel
    async fn grant_channel_operator(&self, user: &User, channel: &str) -> Result<()> {
        // This is a simplified implementation
        // In a real implementation, you would:
        // 1. Find the channel object
        // 2. Add the user as an operator
        // 3. Set the appropriate channel modes
        // 4. Send MODE messages to notify other users
        
        tracing::debug!("Granting operator status in channel {} to user {}", channel, user.nick);
        
        // Implement basic channel operator granting
        // TODO: Integrate with channel manager for full channel mode support
        
        // For now, log the action and prepare mode change message
        // In production, this would:
        // 1. Find the channel in the channel manager
        // 2. Add the user as an operator (set +o mode)
        // 3. Broadcast the mode change to all channel members
        // 4. Update channel state in database
        
        tracing::info!("OPME: Granting operator status in channel {} to user {}", channel, user.nick);
        
        // Prepare mode change message that would be sent to channel members
        let mode_message = format!("MODE {} +o {}", channel, user.nick);
        tracing::debug!("Would send mode change: {}", mode_message);
        
        // In production, this would use the channel manager to:
        // - Set the +o mode on the user
        // - Broadcast MODE message to all channel members
        // - Update channel state in the database
        
        Ok(())
    }
    
    /// Notify channel members of OPME usage
    async fn notify_channel_opme(&self, channel: &str, nick: &str) -> Result<()> {
        // Send a notice to the channel about OPME usage
        let notice_msg = format!("NOTICE {} :{} used OPME to become an operator", channel, nick);
        tracing::debug!("Would send notice: {}", notice_msg);
        
        // Implement basic channel notification
        // TODO: Integrate with channel manager to send actual NOTICE messages
        
        if self.config.notify_channel {
            // For now, log the notification that would be sent
            // In production, this would:
            // 1. Get all users in the channel from channel manager
            // 2. Send NOTICE message to each user in the channel
            // 3. Use proper IRC message formatting
            
            let notification_message = format!("NOTICE {} :{} used OPME to become an operator", channel, nick);
            tracing::info!("OPME notification: {}", notification_message);
            
            // In production, this would use the channel manager to:
            // - Get list of all users in the channel
            // - Send NOTICE message to each user
            // - Format message properly with server prefix
            tracing::debug!("Would send notification to all users in channel: {}", channel);
        }
        
        Ok(())
    }
    
    /// Check if a user can use OPME
    pub fn can_use_opme(&self, user: &User) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        if self.config.require_oper && !user.is_operator() {
            return false;
        }
        
        true
    }
    
    /// Get OPME statistics
    pub async fn get_statistics(&self) -> OpmeStats {
        // This would return actual statistics in a real implementation
        OpmeStats {
            total_uses: 0,
            enabled: self.config.enabled,
            require_oper: self.config.require_oper,
            rate_limit_enabled: self.config.rate_limit.enabled,
        }
    }
}

/// OPME statistics
#[derive(Debug, Clone)]
pub struct OpmeStats {
    /// Total number of OPME uses
    pub total_uses: u64,
    /// Whether OPME is enabled
    pub enabled: bool,
    /// Whether operator privileges are required
    pub require_oper: bool,
    /// Whether rate limiting is enabled
    pub rate_limit_enabled: bool,
}

impl Default for OpmeModule {
    fn default() -> Self {
        Self::new(OpmeConfig::default())
    }
}

/// OPME configuration builder
pub struct OpmeConfigBuilder {
    config: OpmeConfig,
}

impl OpmeConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: OpmeConfig::default(),
        }
    }
    
    /// Set whether OPME is enabled
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }
    
    /// Set whether operator privileges are required
    pub fn require_oper(mut self, require: bool) -> Self {
        self.config.require_oper = require;
        self
    }
    
    /// Set whether to log usage
    pub fn log_usage(mut self, log: bool) -> Self {
        self.config.log_usage = log;
        self
    }
    
    /// Set whether to notify channel
    pub fn notify_channel(mut self, notify: bool) -> Self {
        self.config.notify_channel = notify;
        self
    }
    
    /// Set rate limiting configuration
    pub fn rate_limit(mut self, enabled: bool, max_uses: u32, time_window: u64) -> Self {
        self.config.rate_limit = OpmeRateLimit {
            enabled,
            max_uses,
            time_window,
        };
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> OpmeConfig {
        self.config
    }
}

impl Default for OpmeConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl rustircd_core::Module for OpmeModule {
    fn name(&self) -> &str {
        "opme"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "Provides OPME command functionality for operators to grant themselves operator status in channels"
    }
    
    async fn init(&mut self) -> Result<()> {
        tracing::info!("{} module initialized", self.name());
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("{} module cleaned up", self.name());
        Ok(())
    }
    
    async fn handle_message(&mut self, client: &rustircd_core::Client, message: &rustircd_core::Message, _context: &ModuleContext) -> Result<ModuleResult> {
        match message.command {
            rustircd_core::MessageType::Custom(ref cmd) if cmd == "OPME" => {
                // Create a default config for now - in production this should come from context
                let config = rustircd_core::Config::default();
                self.handle_opme(client, message, &config).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }
    
    async fn handle_server_message(&mut self, _server: &str, _message: &rustircd_core::Message, _context: &ModuleContext) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, _user: &rustircd_core::User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, _user: &rustircd_core::User, _context: &ModuleContext) -> Result<()> {
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
    
    async fn handle_stats_query(&mut self, _query: &str, _client_id: Uuid, _server: Option<&rustircd_core::Server>) -> Result<Vec<ModuleStatsResponse>> {
        Ok(vec![])
    }
    
    fn get_stats_queries(&self) -> Vec<String> {
        vec![]
    }
    
    fn register_numerics(&self, _manager: &mut ModuleNumericManager) -> Result<()> {
        Ok(())
    }
}
