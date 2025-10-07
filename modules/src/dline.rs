//! DLINE Module
//! 
//! Provides DNS line (DLINE) management functionality.
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

/// DLINE module for DNS line management
pub struct DlineModule {
    /// DNS lines (DLINE)
    dlines: RwLock<HashMap<String, DnsLine>>,
    /// Configuration
    config: DlineConfig,
}

/// DNS line entry
#[derive(Debug, Clone)]
pub struct DnsLine {
    pub hostname: String,
    pub reason: String,
    pub set_by: String,
    pub set_time: u64,
    pub expire_time: Option<u64>,
    pub is_active: bool,
}

/// Configuration for DLINE management
#[derive(Debug, Clone)]
pub struct DlineConfig {
    pub max_duration: u64, // in seconds
    pub allow_permanent_bans: bool,
    pub require_operator: bool,
    pub auto_cleanup_expired: bool,
}

impl Default for DlineConfig {
    fn default() -> Self {
        Self {
            max_duration: 86400 * 30, // 30 days
            allow_permanent_bans: true,
            require_operator: true,
            auto_cleanup_expired: true,
        }
    }
}

impl DlineModule {
    /// Create a new DLINE module
    pub fn new() -> Self {
        Self {
            dlines: RwLock::new(HashMap::new()),
            config: DlineConfig::default(),
        }
    }
    
    /// Create a new DLINE module with custom configuration
    pub fn with_config(config: DlineConfig) -> Self {
        Self {
            dlines: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// Handle DLINE command
    async fn handle_dline(&self, client: &Client, user: &User, args: &[String], context: &ModuleContext) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            self.list_dlines(client, user).await?;
            return Ok(());
        }
        
        let hostname = &args[0];
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
        
        self.add_dline(client, user, hostname, &reason, duration, context).await?;
        Ok(())
    }
    
    /// Handle UNDLINE command
    async fn handle_undline(&self, client: &Client, user: &User, args: &[String], context: &ModuleContext) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["UNDLINE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let hostname = &args[0];
        self.remove_dline(client, user, hostname, context).await?;
        Ok(())
    }
    
    /// Add a DLINE
    async fn add_dline(&self, client: &Client, user: &User, hostname: &str, reason: &str, duration: Option<u64>, context: &ModuleContext) -> Result<()> {
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        if let Some(dur) = duration {
            if dur > self.config.max_duration {
                client.send_numeric(NumericReply::ErrInvalidDuration, &[&format!("Maximum duration is {} seconds", self.config.max_duration)])?;
                return Ok(());
            }
        }
        
        let dline = DnsLine {
            hostname: hostname.to_string(),
            reason: reason.to_string(),
            set_by: user.nickname().to_string(),
            set_time: current_time,
            expire_time,
            is_active: true,
        };
        
        let mut dlines = self.dlines.write().await;
        dlines.insert(hostname.to_string(), dline);
        
        client.send_numeric(NumericReply::RplDline, &[hostname, reason, &format!("Set by {}", user.nickname())])?;
        
        info!("DLINE added: {} by {} - {}", hostname, user.nickname(), reason);
        
        // Broadcast to other servers
        self.broadcast_dline_to_servers(hostname, reason, &user.nickname(), duration, context).await?;
        
        // Check existing connections and disconnect matching users
        self.disconnect_matching_users(hostname, &format!("DLINE: {}", reason), context).await?;
        
        Ok(())
    }
    
    /// Remove a DLINE
    async fn remove_dline(&self, client: &Client, user: &User, hostname: &str, context: &ModuleContext) -> Result<()> {
        let mut dlines = self.dlines.write().await;
        
        if dlines.remove(hostname).is_some() {
            client.send_numeric(NumericReply::RplDline, &[hostname, "Removed", &format!("Removed by {}", user.nickname())])?;
            info!("DLINE removed: {} by {}", hostname, user.nickname());
            
            // Broadcast removal to other servers
            drop(dlines); // Release the lock before async call
            self.broadcast_undline_to_servers(hostname, &user.nickname(), context).await?;
        } else {
            client.send_numeric(NumericReply::ErrNoSuchDline, &[hostname, "No such DLINE"])?;
        }
        
        Ok(())
    }
    
    /// List DLINEs
    async fn list_dlines(&self, client: &Client, user: &User) -> Result<()> {
        let dlines = self.dlines.read().await;
        
        if dlines.is_empty() {
            client.send_numeric(NumericReply::RplDline, &["*", "No DLINEs set"])?;
            return Ok(());
        }
        
        for dline in dlines.values() {
            let expire_info = if let Some(expire) = dline.expire_time {
                format!("Expires: {}", self.format_time(expire))
            } else {
                "Permanent".to_string()
            };
            
            client.send_numeric(NumericReply::RplDline, &[
                &dline.hostname, 
                &dline.reason, 
                &format!("Set by {} at {} - {}", dline.set_by, self.format_time(dline.set_time), expire_info)
            ])?;
        }
        
        client.send_numeric(NumericReply::RplEndOfDlines, &["End of DLINE list"])?;
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
    
    /// Check if a user matches any active DLINEs
    pub async fn check_user_dline(&self, user: &User) -> Option<String> {
        let current_time = self.get_current_time();
        
        let dlines = self.dlines.read().await;
        for dline in dlines.values() {
            if dline.is_active && user.hostname().contains(&dline.hostname) {
                if dline.expire_time.map_or(true, |expire| current_time < expire) {
                    return Some(format!("DLINE: {}", dline.reason));
                }
            }
        }
        
        None
    }
    
    /// Clean up expired DLINEs
    pub async fn cleanup_expired_dlines(&self) -> Result<()> {
        if !self.config.auto_cleanup_expired {
            return Ok(());
        }
        
        let current_time = self.get_current_time();
        let mut expired_count = 0;
        
        let mut dlines = self.dlines.write().await;
        dlines.retain(|_, dline| {
            let should_keep = dline.expire_time.map_or(true, |expire| current_time < expire);
            if !should_keep {
                expired_count += 1;
            }
            should_keep
        });
        
        if expired_count > 0 {
            info!("Cleaned up {} expired DLINEs", expired_count);
        }
        
        Ok(())
    }
    
    /// Get count of active DLINEs
    pub async fn get_active_dlines_count(&self) -> usize {
        let dlines = self.dlines.read().await;
        dlines.len()
    }
    
    /// Get count of expired DLINEs
    pub async fn get_expired_dlines_count(&self) -> usize {
        let current_time = self.get_current_time();
        let dlines = self.dlines.read().await;
        
        dlines.values().filter(|dline| {
            dline.expire_time.map_or(false, |expire| current_time >= expire)
        }).count()
    }
    
    /// Broadcast DLINE to other servers
    async fn broadcast_dline_to_servers(&self, hostname: &str, reason: &str, set_by: &str, duration: Option<u64>, context: &ModuleContext) -> Result<()> {
        let mut params = vec![hostname.to_string(), reason.to_string(), set_by.to_string()];
        if let Some(dur) = duration {
            params.push(dur.to_string());
        }
        
        let message = Message::new(MessageType::Custom("DLINE".to_string()), params);
        context.broadcast_to_servers(message).await?;
        info!("DLINE broadcasted to servers: {} {} {} {:?}", hostname, reason, set_by, duration);
        Ok(())
    }
    
    /// Disconnect users matching the ban hostname
    async fn disconnect_matching_users(&self, hostname: &str, quit_reason: &str, context: &ModuleContext) -> Result<()> {
        let client_connections = context.client_connections.read().await;
        let mut users_to_disconnect = Vec::new();
        
        // Find all users that match the ban hostname
        for (user_id, client) in client_connections.iter() {
            if let Some(user) = client.get_user() {
                if user.hostname().contains(hostname) {
                    users_to_disconnect.push((*user_id, user.clone()));
                }
            }
        }
        drop(client_connections);
        
        // Disconnect matching users
        for (user_id, user) in users_to_disconnect {
            info!("Disconnecting user {} due to DLINE: {}", user.nickname(), quit_reason);
            
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
            info!("Disconnected {} users due to DLINE: {}", users_to_disconnect.len(), hostname);
        }
        
        Ok(())
    }
    
    /// Broadcast UNDLINE to other servers
    async fn broadcast_undline_to_servers(&self, hostname: &str, removed_by: &str, context: &ModuleContext) -> Result<()> {
        let message = Message::new(
            MessageType::Custom("UNDLINE".to_string()),
            vec![hostname.to_string(), removed_by.to_string()]
        );
        context.broadcast_to_servers(message).await?;
        info!("UNDLINE broadcasted to servers: {} removed by {}", hostname, removed_by);
        Ok(())
    }
    
    /// Handle DLINE message from another server
    async fn handle_server_dline(&self, server: &str, params: &[String], context: &ModuleContext) -> Result<()> {
        if params.len() < 2 {
            warn!("Invalid DLINE message from server {}: insufficient parameters", server);
            return Ok(());
        }
        
        let hostname = &params[0];
        let reason = &params[1];
        let set_by = if params.len() > 2 { &params[2] } else { "unknown" };
        let duration = if params.len() > 3 {
            self.parse_duration(&params[3]).ok().flatten()
        } else {
            None
        };
        
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        let dline = DnsLine {
            hostname: hostname.to_string(),
            reason: reason.to_string(),
            set_by: set_by.to_string(),
            set_time: current_time,
            expire_time,
            is_active: true,
        };
        
        let mut dlines = self.dlines.write().await;
        dlines.insert(hostname.to_string(), dline);
        
        info!("DLINE received from server {}: {} - {}", server, hostname, reason);
        
        // Check existing connections and disconnect matching users
        drop(dlines); // Release the lock before async call
        self.disconnect_matching_users(hostname, &format!("DLINE: {}", reason), context).await?;
        
        Ok(())
    }
    
    /// Handle UNDLINE message from another server
    async fn handle_server_undline(&self, server: &str, params: &[String], _context: &ModuleContext) -> Result<()> {
        if params.is_empty() {
            warn!("Invalid UNDLINE message from server {}: no parameters", server);
            return Ok(());
        }
        
        let hostname = &params[0];
        let removed_by = if params.len() > 1 { &params[1] } else { "unknown" };
        
        let mut dlines = self.dlines.write().await;
        if dlines.remove(hostname).is_some() {
            info!("UNDLINE received from server {}: {} removed by {}", server, hostname, removed_by);
        } else {
            debug!("UNDLINE received from server {} for non-existent DLINE: {}", server, hostname);
        }
        
        Ok(())
    }
}

#[async_trait]
impl Module for DlineModule {
    fn name(&self) -> &str {
        "dline"
    }
    
    fn description(&self) -> &str {
        "Provides DNS line (DLINE) management functionality"
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
            MessageType::Custom(ref cmd) if cmd == "DLINE" => {
                self.handle_dline(client, user, &message.params, context).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNDLINE" => {
                self.handle_undline(client, user, &message.params, context).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_server_message(&mut self, server: &str, message: &Message, context: &ModuleContext) -> Result<ModuleResult> {
        match message.command {
            MessageType::Custom(ref cmd) if cmd == "DLINE" => {
                self.handle_server_dline(server, &message.params, context).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNDLINE" => {
                self.handle_server_undline(server, &message.params, context).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_user_registration(&mut self, user: &User, _context: &ModuleContext) -> Result<()> {
        // Check if the user matches any active DLINEs
        if let Some(ban_reason) = self.check_user_dline(user).await {
            // User is banned, we need to disconnect them
            // This would need access to the server's client management system
            info!("User {} blocked by DLINE: {}", user.nickname(), ban_reason);
            // TODO: Implement actual user disconnection when server context is available
            return Err(Error::PermissionDenied(format!("Banned: {}", ban_reason)));
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
            NumericReply::RplDline.numeric_code(),
            NumericReply::RplEndOfDlines.numeric_code(),
            NumericReply::ErrNoSuchDline.numeric_code(),
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
        info!("DLINE module cleaned up");
        Ok(())
    }
}

impl Default for DlineModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpProvider for DlineModule {
    fn get_help_topics(&self) -> Vec<HelpTopic> {
        vec![
            HelpTopic {
                command: "DLINE".to_string(),
                syntax: "DLINE <hostname> <reason> [duration]".to_string(),
                description: "Set a DNS line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "DLINE badhost.com Bad host".to_string(),
                    "DLINE *.badhost.com 30d Bad domain".to_string(),
                ],
                module_name: Some("dline".to_string()),
            },
            HelpTopic {
                command: "UNDLINE".to_string(),
                syntax: "UNDLINE <hostname>".to_string(),
                description: "Remove a DNS line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNDLINE badhost.com".to_string(),
                ],
                module_name: Some("dline".to_string()),
            },
        ]
    }
    
    fn get_command_help(&self, command: &str) -> Option<HelpTopic> {
        match command {
            "DLINE" => Some(HelpTopic {
                command: "DLINE".to_string(),
                syntax: "DLINE <hostname> <reason> [duration]".to_string(),
                description: "Set a DNS line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "DLINE badhost.com Bad host".to_string(),
                    "DLINE *.badhost.com 30d Bad domain".to_string(),
                ],
                module_name: Some("dline".to_string()),
            }),
            "UNDLINE" => Some(HelpTopic {
                command: "UNDLINE".to_string(),
                syntax: "UNDLINE <hostname>".to_string(),
                description: "Remove a DNS line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNDLINE badhost.com".to_string(),
                ],
                module_name: Some("dline".to_string()),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dline_config_default() {
        let config = DlineConfig::default();
        assert_eq!(config.max_duration, 86400 * 30);
        assert!(config.allow_permanent_bans);
        assert!(config.require_operator);
    }
    
    #[test]
    fn test_parse_duration() {
        let module = DlineModule::new();
        
        assert_eq!(module.parse_duration("1d").unwrap(), Some(86400));
        assert_eq!(module.parse_duration("2h").unwrap(), Some(7200));
        assert_eq!(module.parse_duration("30m").unwrap(), Some(1800));
        assert_eq!(module.parse_duration("3600s").unwrap(), Some(3600));
        assert_eq!(module.parse_duration("3600").unwrap(), Some(3600));
        assert_eq!(module.parse_duration("0").unwrap(), None);
        assert_eq!(module.parse_duration("").unwrap(), None);
    }
}
