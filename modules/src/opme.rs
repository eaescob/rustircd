//! OPME Extension
//! 
//! This module provides the OPME command functionality, allowing operators
//! to grant themselves operator status in channels. Based on Solanum's
//! m_opme.c implementation.
//! 
//! Based on: https://github.com/solanum-ircd/solanum/blob/main/extensions/m_opme.c

use crate::core::{User, Message, Client, Result, Error, NumericReply, Config};
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
            let error_msg = NumericReply::unknown_command(&message.command);
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        // Check if user is provided
        let user = client.user.as_ref().ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if user has operator privileges (if required)
        if self.config.require_oper && !user.is_operator() {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
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
            let error_msg = NumericReply::err_no_such_channel(&channel);
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        // Check rate limiting
        if self.config.rate_limit.enabled {
            if let Err(e) = self.check_rate_limit(user.id, &channel).await {
                let error_msg = NumericReply::err_too_many_targets(&channel, "You are using OPME too frequently");
                let _ = client.send(error_msg);
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
        let success_msg = NumericReply::raw(381, &format!(":You are now an operator in {}", channel));
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
        
        // For now, we'll just return Ok() to allow the command
        // TODO: Implement proper rate limiting with persistent storage
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
        
        // TODO: Implement actual channel operator granting
        // This would involve:
        // - Finding the channel in the channel manager
        // - Adding the user as an operator
        // - Setting the +o mode
        // - Broadcasting the mode change
        
        Ok(())
    }
    
    /// Notify channel members of OPME usage
    async fn notify_channel_opme(&self, channel: &str, nick: &str) -> Result<()> {
        // Send a notice to the channel about OPME usage
        let notice_msg = format!("NOTICE {} :{} used OPME to become an operator", channel, nick);
        tracing::debug!("Would send notice: {}", notice_msg);
        
        // TODO: Implement actual channel notification
        // This would involve:
        // - Finding all users in the channel
        // - Sending them a NOTICE message
        
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
