//! KLINE Module
//! 
//! Provides kill line (KLINE) management functionality.
//! Based on Ratbox's ban management modules.

use rustircd_core::{
    async_trait, Client, Error, Message, MessageType, Module, ModuleManager,
    ModuleNumericManager, module::{ModuleResult, ModuleStatsResponse, ModuleContext},
    NumericReply, Result, User
};
use tracing::{debug, info, warn};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use crate::help::{HelpProvider, HelpTopic};

/// KLINE module for kill line management
pub struct KlineModule {
    /// Kill lines (KLINE) 
    klines: RwLock<HashMap<String, KillLine>>,
    /// Configuration
    config: KlineConfig,
}

/// Kill line entry
#[derive(Debug, Clone)]
pub struct KillLine {
    pub mask: String,
    pub reason: String,
    pub set_by: String,
    pub set_time: u64,
    pub expire_time: Option<u64>,
    pub is_active: bool,
}

/// Configuration for KLINE management
#[derive(Debug, Clone)]
pub struct KlineConfig {
    pub max_duration: u64, // in seconds
    pub allow_permanent_bans: bool,
    pub require_operator: bool,
    pub auto_cleanup_expired: bool,
}

impl Default for KlineConfig {
    fn default() -> Self {
        Self {
            max_duration: 86400 * 30, // 30 days
            allow_permanent_bans: true,
            require_operator: true,
            auto_cleanup_expired: true,
        }
    }
}

impl KlineModule {
    /// Create a new KLINE module
    pub fn new() -> Self {
        Self {
            klines: RwLock::new(HashMap::new()),
            config: KlineConfig::default(),
        }
    }
    
    /// Create a new KLINE module with custom configuration
    pub fn with_config(config: KlineConfig) -> Self {
        Self {
            klines: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// Handle KLINE command
    async fn handle_kline(&self, client: &Client, user: &User, args: &[String], context: &ModuleContext) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            self.list_klines(client, user).await?;
            return Ok(());
        }
        
        let mask = &args[0];
        let reason = if args.len() > 1 {
            args[1..].join(" ")
        } else {
            "No reason given".to_string()
        };
        
        let duration = if args.len() > 2 {
            self.parse_duration(&args[2])?
        } else {
            None
        };
        
        self.add_kline(client, user, mask, &reason, duration, context).await?;
        Ok(())
    }
    
    /// Handle UNKLINE command
    async fn handle_unkline(&self, client: &Client, user: &User, args: &[String], context: &ModuleContext) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["UNKLINE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let mask = &args[0];
        self.remove_kline(client, user, mask, context).await?;
        Ok(())
    }
    
    /// Add a KLINE
    async fn add_kline(&self, client: &Client, user: &User, mask: &str, reason: &str, duration: Option<u64>, context: &ModuleContext) -> Result<()> {
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        if let Some(dur) = duration {
            if dur > self.config.max_duration {
                client.send_numeric(NumericReply::ErrInvalidDuration, &[&format!("Maximum duration is {} seconds", self.config.max_duration)])?;
                return Ok(());
            }
        }
        
        let kline = KillLine {
            mask: mask.to_string(),
            reason: reason.to_string(),
            set_by: user.nickname().to_string(),
            set_time: current_time,
            expire_time,
            is_active: true,
        };
        
        let mut klines = self.klines.write().await;
        klines.insert(mask.to_string(), kline);
        
        client.send_numeric(NumericReply::RplKline, &[mask, reason, &format!("Set by {}", user.nickname())])?;
        
        info!("KLINE added: {} by {} - {}", mask, user.nickname(), reason);
        
        // Broadcast to other servers
        self.broadcast_kline_to_servers(mask, reason, &user.nickname(), duration, context).await?;
        
        // Check existing connections and disconnect matching users
        self.disconnect_matching_users(mask, &format!("KLINE: {}", reason), context).await?;
        
        Ok(())
    }
    
    /// Remove a KLINE
    async fn remove_kline(&self, client: &Client, user: &User, mask: &str, context: &ModuleContext) -> Result<()> {
        let mut klines = self.klines.write().await;
        
        if klines.remove(mask).is_some() {
            client.send_numeric(NumericReply::RplKline, &[mask, "Removed", &format!("Removed by {}", user.nickname())])?;
            info!("KLINE removed: {} by {}", mask, user.nickname());
            
            // Broadcast removal to other servers
            drop(klines); // Release the lock before async call
            self.broadcast_unkline_to_servers(mask, &user.nickname(), context).await?;
        } else {
            client.send_numeric(NumericReply::ErrNoSuchKline, &[mask, "No such KLINE"])?;
        }
        
        Ok(())
    }
    
    /// List KLINEs
    async fn list_klines(&self, client: &Client, user: &User) -> Result<()> {
        let klines = self.klines.read().await;
        
        if klines.is_empty() {
            client.send_numeric(NumericReply::RplKline, &["*", "No KLINEs set"])?;
            return Ok(());
        }
        
        for kline in klines.values() {
            let expire_info = if let Some(expire) = kline.expire_time {
                format!("Expires: {}", self.format_time(expire))
            } else {
                "Permanent".to_string()
            };
            
            client.send_numeric(NumericReply::RplKline, &[
                &kline.mask, 
                &kline.reason, 
                &format!("Set by {} at {} - {}", kline.set_by, self.format_time(kline.set_time), expire_info)
            ])?;
        }
        
        client.send_numeric(NumericReply::RplEndOfKlines, &["End of KLINE list"])?;
        Ok(())
    }
    
    /// Parse duration string (e.g., "1d", "2h", "30m", "3600s")
    fn parse_duration(&self, duration_str: &str) -> Result<Option<u64>> {
        if duration_str == "0" || duration_str.is_empty() {
            return Ok(None);
        }
        
        let duration_str = duration_str.to_lowercase();
        let (number_str, unit) = if duration_str.ends_with('d') {
            (&duration_str[..duration_str.len()-1], "d")
        } else if duration_str.ends_with('h') {
            (&duration_str[..duration_str.len()-1], "h")
        } else if duration_str.ends_with('m') {
            (&duration_str[..duration_str.len()-1], "m")
        } else if duration_str.ends_with('s') {
            (&duration_str[..duration_str.len()-1], "s")
        } else {
            (duration_str.as_str(), "s")
        };
        
        let number: u64 = number_str.parse()
            .map_err(|_| "Invalid duration number")?;
        
        let seconds = match unit {
            "d" => number * 86400,
            "h" => number * 3600,
            "m" => number * 60,
            "s" => number,
            _ => return Err(Error::Config("Invalid duration unit".to_string())),
        };
        
        Ok(Some(seconds))
    }
    
    /// Get current time as Unix timestamp
    fn get_current_time(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Format time as readable string
    fn format_time(&self, timestamp: u64) -> String {
        use chrono::{DateTime, Utc, NaiveDateTime};
        let naive = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).unwrap_or_default();
        let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }
    
    /// Check if a user matches any active KLINEs
    pub async fn check_user_kline(&self, user: &User) -> Option<String> {
        let current_time = self.get_current_time();
        
        let klines = self.klines.read().await;
        for kline in klines.values() {
            if kline.is_active && self.matches_mask(&kline.mask, user) {
                if kline.expire_time.map_or(true, |expire| current_time < expire) {
                    return Some(format!("KLINE: {}", kline.reason));
                }
            }
        }
        
        None
    }
    
    /// Check if a user matches a ban mask
    fn matches_mask(&self, mask: &str, user: &User) -> bool {
        // Simple wildcard matching - in a real implementation, this would be more sophisticated
        let user_mask = format!("{}!{}@{}", user.nickname(), user.username(), user.hostname());
        
        // Convert IRC wildcards to regex patterns
        let pattern = mask
            .replace("*", ".*")
            .replace("?", ".");
        
        // Simple pattern matching - in production, use proper regex
        if mask.contains('*') || mask.contains('?') {
            // Wildcard matching
            self.simple_wildcard_match(&pattern, &user_mask)
        } else {
            // Exact match
            mask == user_mask || mask == user.nickname() || mask == user.hostname()
        }
    }
    
    /// Simple wildcard matching
    fn simple_wildcard_match(&self, pattern: &str, text: &str) -> bool {
        // Very basic wildcard matching - in production, use proper regex
        if pattern == ".*" {
            return true;
        }
        
        if pattern.starts_with(".*") && pattern.ends_with(".*") {
            let middle = &pattern[2..pattern.len()-2];
            return text.contains(middle);
        }
        
        if pattern.starts_with(".*") {
            return text.ends_with(&pattern[2..]);
        }
        
        if pattern.ends_with(".*") {
            return text.starts_with(&pattern[..pattern.len()-2]);
        }
        
        text == pattern
    }
    
    /// Clean up expired KLINEs
    pub async fn cleanup_expired_klines(&self) -> Result<()> {
        if !self.config.auto_cleanup_expired {
            return Ok(());
        }
        
        let current_time = self.get_current_time();
        let mut expired_count = 0;
        
        let mut klines = self.klines.write().await;
        klines.retain(|_, kline| {
            let should_keep = kline.expire_time.map_or(true, |expire| current_time < expire);
            if !should_keep {
                expired_count += 1;
            }
            should_keep
        });
        
        if expired_count > 0 {
            info!("Cleaned up {} expired KLINEs", expired_count);
        }
        
        Ok(())
    }
    
    /// Get count of active KLINEs
    pub async fn get_active_klines_count(&self) -> usize {
        let klines = self.klines.read().await;
        klines.len()
    }
    
    /// Get count of expired KLINEs
    pub async fn get_expired_klines_count(&self) -> usize {
        let current_time = self.get_current_time();
        let klines = self.klines.read().await;
        
        klines.values().filter(|kline| {
            kline.expire_time.map_or(false, |expire| current_time >= expire)
        }).count()
    }
    
    /// Broadcast KLINE to other servers
    async fn broadcast_kline_to_servers(&self, mask: &str, reason: &str, set_by: &str, duration: Option<u64>, context: &ModuleContext) -> Result<()> {
        let mut params = vec![mask.to_string(), reason.to_string(), set_by.to_string()];
        if let Some(dur) = duration {
            params.push(dur.to_string());
        }
        
        let message = Message::new(MessageType::Custom("KLINE".to_string()), params);
        context.broadcast_to_servers(message).await?;
        info!("KLINE broadcasted to servers: {} {} {} {:?}", mask, reason, set_by, duration);
        Ok(())
    }
    
    /// Disconnect users matching the ban mask
    async fn disconnect_matching_users(&self, mask: &str, quit_reason: &str, context: &ModuleContext) -> Result<()> {
        let client_connections = context.client_connections.read().await;
        let mut users_to_disconnect = Vec::new();
        
        // Find all users that match the ban mask
        for (user_id, client) in client_connections.iter() {
            if let Some(user) = client.get_user() {
                if self.matches_mask(mask, user) {
                    users_to_disconnect.push((*user_id, user.clone()));
                }
            }
        }
        drop(client_connections);
        
        // Disconnect matching users
        for (user_id, user) in users_to_disconnect.clone() {
            info!("Disconnecting user {} due to KLINE: {}", user.nickname(), quit_reason);
            
            // Send QUIT message to the user
            let quit_message = Message::new(MessageType::Quit, vec![quit_reason.to_string()]);
            if let Some(client) = context.client_connections.read().await.get(&user_id) {
                let _ = client.send(quit_message);
            }
            
            // Broadcast QUIT to all users in the same channels
            let quit_broadcast = Message::with_prefix(
                user.prefix(),
                MessageType::Quit,
                vec![quit_reason.to_string()],
            );
            
            for channel in &user.channels {
                context.send_to_channel(channel, quit_broadcast.clone()).await?;
            }
            
            // Remove user from database
            context.remove_user(user_id)?;
            
            // Unregister client connection
            context.unregister_client(user_id).await?;
        }
        
        if !users_to_disconnect.is_empty() {
            info!("Disconnected {} users due to KLINE: {}", users_to_disconnect.len(), mask);
        }
        
        Ok(())
    }
    
    /// Broadcast UNKLINE to other servers
    async fn broadcast_unkline_to_servers(&self, mask: &str, removed_by: &str, context: &ModuleContext) -> Result<()> {
        let message = Message::new(
            MessageType::Custom("UNKLINE".to_string()),
            vec![mask.to_string(), removed_by.to_string()]
        );
        context.broadcast_to_servers(message).await?;
        info!("UNKLINE broadcasted to servers: {} removed by {}", mask, removed_by);
        Ok(())
    }
    
    /// Handle KLINE message from another server
    async fn handle_server_kline(&self, server: &str, params: &[String], context: &ModuleContext) -> Result<()> {
        if params.len() < 2 {
            warn!("Invalid KLINE message from server {}: insufficient parameters", server);
            return Ok(());
        }
        
        let mask = &params[0];
        let reason = &params[1];
        let set_by = if params.len() > 2 { &params[2] } else { "unknown" };
        let duration = if params.len() > 3 {
            self.parse_duration(&params[3]).ok().flatten()
        } else {
            None
        };
        
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        let kline = KillLine {
            mask: mask.to_string(),
            reason: reason.to_string(),
            set_by: set_by.to_string(),
            set_time: current_time,
            expire_time,
            is_active: true,
        };
        
        let mut klines = self.klines.write().await;
        klines.insert(mask.to_string(), kline);
        
        info!("KLINE received from server {}: {} - {}", server, mask, reason);
        
        // Check existing connections and disconnect matching users
        drop(klines); // Release the lock before async call
        self.disconnect_matching_users(mask, &format!("KLINE: {}", reason), context).await?;
        
        Ok(())
    }
    
    /// Handle UNKLINE message from another server
    async fn handle_server_unkline(&self, server: &str, params: &[String], _context: &ModuleContext) -> Result<()> {
        if params.is_empty() {
            warn!("Invalid UNKLINE message from server {}: no parameters", server);
            return Ok(());
        }
        
        let mask = &params[0];
        let removed_by = if params.len() > 1 { &params[1] } else { "unknown" };
        
        let mut klines = self.klines.write().await;
        if klines.remove(mask).is_some() {
            info!("UNKLINE received from server {}: {} removed by {}", server, mask, removed_by);
        } else {
            debug!("UNKLINE received from server {} for non-existent KLINE: {}", server, mask);
        }
        
        Ok(())
    }
}

#[async_trait]
impl Module for KlineModule {
    fn name(&self) -> &str {
        "kline"
    }
    
    fn description(&self) -> &str {
        "Provides kill line (KLINE) management functionality"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("{} module initialized", self.name());
        Ok(())
    }

    async fn handle_message(&mut self, client: &Client, message: &Message, context: &ModuleContext) -> Result<ModuleResult> {
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled),
        };

        match message.command {
            MessageType::Custom(ref cmd) if cmd == "KLINE" => {
                self.handle_kline(client, user, &message.params, context).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNKLINE" => {
                self.handle_unkline(client, user, &message.params, context).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_server_message(&mut self, server: &str, message: &Message, context: &ModuleContext) -> Result<ModuleResult> {
        match message.command {
            MessageType::Custom(ref cmd) if cmd == "KLINE" => {
                self.handle_server_kline(server, &message.params, context).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNKLINE" => {
                self.handle_server_unkline(server, &message.params, context).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_user_registration(&mut self, user: &User, context: &ModuleContext) -> Result<()> {
        // Check if the user matches any active KLINEs
        if let Some(ban_reason) = self.check_user_kline(user).await {
            // User is banned, disconnect them
            info!("User {} blocked by KLINE: {}", user.nickname(), ban_reason);
            
            // Send QUIT message to the user
            let quit_message = Message::new(MessageType::Quit, vec![ban_reason.clone()]);
            if let Some(client) = context.client_connections.read().await.get(&user.id) {
                let _ = client.send(quit_message);
            }
            
            // Broadcast QUIT to all users in the same channels
            let quit_broadcast = Message::with_prefix(
                user.prefix(),
                MessageType::Quit,
                vec![ban_reason.clone()],
            );
            
            for channel in &user.channels {
                context.send_to_channel(channel, quit_broadcast.clone()).await?;
            }
            
            // Remove user from database
            context.remove_user(user.id)?;
            
            // Unregister client connection
            context.unregister_client(user.id).await?;
            
            return Err(Error::Auth(format!("Banned: {}", ban_reason)));
        }
        Ok(())
    }

    async fn handle_user_disconnection(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["message_handler".to_string(), "user_registration_handler".to_string(), "server_message_handler".to_string()]
    }

    fn supports_capability(&self, capability: &str) -> bool {
        capability == "message_handler" || capability == "user_registration_handler" || capability == "server_message_handler"
    }

    fn get_numeric_replies(&self) -> Vec<u16> {
        vec![
            NumericReply::RplKline.numeric_code(),
            NumericReply::RplEndOfKlines.numeric_code(),
            NumericReply::ErrNoSuchKline.numeric_code(),
            NumericReply::ErrInvalidDuration.numeric_code(),
        ]
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
        info!("KLINE module cleaned up");
        Ok(())
    }
}

impl Default for KlineModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpProvider for KlineModule {
    fn get_help_topics(&self) -> Vec<HelpTopic> {
        vec![
            HelpTopic {
                command: "KLINE".to_string(),
                syntax: "KLINE <mask> <reason> [duration]".to_string(),
                description: "Set a kill line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "KLINE *@baduser.com Bad user".to_string(),
                    "KLINE *@*.badhost.com 7d Bad host".to_string(),
                ],
                module_name: Some("kline".to_string()),
            },
            HelpTopic {
                command: "UNKLINE".to_string(),
                syntax: "UNKLINE <mask>".to_string(),
                description: "Remove a kill line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNKLINE *@baduser.com".to_string(),
                ],
                module_name: Some("kline".to_string()),
            },
        ]
    }
    
    fn get_command_help(&self, command: &str) -> Option<HelpTopic> {
        match command {
            "KLINE" => Some(HelpTopic {
                command: "KLINE".to_string(),
                syntax: "KLINE <mask> <reason> [duration]".to_string(),
                description: "Set a kill line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "KLINE *@baduser.com Bad user".to_string(),
                    "KLINE *@*.badhost.com 7d Bad host".to_string(),
                ],
                module_name: Some("kline".to_string()),
            }),
            "UNKLINE" => Some(HelpTopic {
                command: "UNKLINE".to_string(),
                syntax: "UNKLINE <mask>".to_string(),
                description: "Remove a kill line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNKLINE *@baduser.com".to_string(),
                ],
                module_name: Some("kline".to_string()),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_kline_config_default() {
        let config = KlineConfig::default();
        assert_eq!(config.max_duration, 86400 * 30);
        assert!(config.allow_permanent_bans);
        assert!(config.require_operator);
    }
    
    #[test]
    fn test_parse_duration() {
        let module = KlineModule::new();
        
        assert_eq!(module.parse_duration("1d").unwrap(), Some(86400));
        assert_eq!(module.parse_duration("2h").unwrap(), Some(7200));
        assert_eq!(module.parse_duration("30m").unwrap(), Some(1800));
        assert_eq!(module.parse_duration("3600s").unwrap(), Some(3600));
        assert_eq!(module.parse_duration("3600").unwrap(), Some(3600));
        assert_eq!(module.parse_duration("0").unwrap(), None);
        assert_eq!(module.parse_duration("").unwrap(), None);
    }
    
    #[test]
    fn test_wildcard_matching() {
        let module = KlineModule::new();
        
        // Note: This is a simplified test - real implementation would use proper regex
        assert!(module.simple_wildcard_match(".*", "anything"));
        assert!(module.simple_wildcard_match("test.*", "test123"));
        assert!(module.simple_wildcard_match(".*test", "123test"));
        assert!(!module.simple_wildcard_match("test", "notest"));
    }
}
