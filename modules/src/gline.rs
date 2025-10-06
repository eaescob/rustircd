//! GLINE Module
//! 
//! Provides global ban (GLINE) management functionality.
//! Based on Ratbox's ban management modules.

use rustircd_core::{
    async_trait, Client, Error, Message, MessageType, Module, ModuleManager,
    ModuleNumericManager, module::{ModuleResult, ModuleStatsResponse},
    NumericReply, Result, User
};
use tracing::{debug, info, warn};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use crate::help::{HelpProvider, HelpTopic};

/// GLINE module for global ban management
pub struct GlineModule {
    /// Global bans (GLINE)
    glines: RwLock<HashMap<String, GlobalBan>>,
    /// Configuration
    config: GlineConfig,
}

/// Global ban entry
#[derive(Debug, Clone)]
pub struct GlobalBan {
    pub mask: String,
    pub reason: String,
    pub set_by: String,
    pub set_time: u64,
    pub expire_time: Option<u64>,
    pub is_active: bool,
}

/// Configuration for GLINE management
#[derive(Debug, Clone)]
pub struct GlineConfig {
    pub max_duration: u64, // in seconds
    pub allow_permanent_bans: bool,
    pub require_operator: bool,
    pub auto_cleanup_expired: bool,
}

impl Default for GlineConfig {
    fn default() -> Self {
        Self {
            max_duration: 86400 * 7, // 7 days
            allow_permanent_bans: true,
            require_operator: true,
            auto_cleanup_expired: true,
        }
    }
}

impl GlineModule {
    /// Create a new GLINE module
    pub fn new() -> Self {
        Self {
            glines: RwLock::new(HashMap::new()),
            config: GlineConfig::default(),
        }
    }
    
    /// Create a new GLINE module with custom configuration
    pub fn with_config(config: GlineConfig) -> Self {
        Self {
            glines: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// Handle GLINE command
    async fn handle_gline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            self.list_glines(client, user).await?;
            return Ok(());
        }
        
        let mask = &args[0];
        let reason = if args.len() > 1 {
            args[1..].join(" ")
        } else {
            "No reason given".to_string()
        };
        
        // Parse duration if provided
        let duration = if args.len() > 2 {
            self.parse_duration(&args[2])?
        } else {
            None
        };
        
        self.add_gline(client, user, mask, &reason, duration).await?;
        Ok(())
    }
    
    /// Handle UNGLINE command
    async fn handle_ungline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["UNGLINE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let mask = &args[0];
        self.remove_gline(client, user, mask).await?;
        Ok(())
    }
    
    /// Add a GLINE
    async fn add_gline(&self, client: &Client, user: &User, mask: &str, reason: &str, duration: Option<u64>) -> Result<()> {
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        // Check duration limits
        if let Some(dur) = duration {
            if dur > self.config.max_duration {
                client.send_numeric(NumericReply::ErrInvalidDuration, &[&format!("Maximum duration is {} seconds", self.config.max_duration)])?;
                return Ok(());
            }
        }
        
        let gline = GlobalBan {
            mask: mask.to_string(),
            reason: reason.to_string(),
            set_by: user.nickname().to_string(),
            set_time: current_time,
            expire_time,
            is_active: true,
        };
        
        let mut glines = self.glines.write().await;
        glines.insert(mask.to_string(), gline);
        
        client.send_numeric(NumericReply::RplGline, &[mask, reason, &format!("Set by {}", user.nickname())])?;
        
        info!("GLINE added: {} by {} - {}", mask, user.nickname(), reason);
        
        // TODO: Broadcast to other servers
        // TODO: Check existing connections and disconnect matching users
        
        Ok(())
    }
    
    /// Remove a GLINE
    async fn remove_gline(&self, client: &Client, user: &User, mask: &str) -> Result<()> {
        let mut glines = self.glines.write().await;
        
        if glines.remove(mask).is_some() {
            client.send_numeric(NumericReply::RplGline, &[mask, "Removed", &format!("Removed by {}", user.nickname())])?;
            info!("GLINE removed: {} by {}", mask, user.nickname());
        } else {
            client.send_numeric(NumericReply::ErrNoSuchGline, &[mask, "No such GLINE"])?;
        }
        
        Ok(())
    }
    
    /// List GLINEs
    async fn list_glines(&self, client: &Client, user: &User) -> Result<()> {
        let glines = self.glines.read().await;
        
        if glines.is_empty() {
            client.send_numeric(NumericReply::RplGline, &["*", "No GLINEs set"])?;
            return Ok(());
        }
        
        for gline in glines.values() {
            let expire_info = if let Some(expire) = gline.expire_time {
                format!("Expires: {}", self.format_time(expire))
            } else {
                "Permanent".to_string()
            };
            
            client.send_numeric(NumericReply::RplGline, &[
                &gline.mask, 
                &gline.reason, 
                &format!("Set by {} at {} - {}", gline.set_by, self.format_time(gline.set_time), expire_info)
            ])?;
        }
        
        client.send_numeric(NumericReply::RplEndOfGlines, &["End of GLINE list"])?;
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
    
    /// Check if a user matches any active GLINEs
    pub async fn check_user_gline(&self, user: &User) -> Option<String> {
        let current_time = self.get_current_time();
        
        let glines = self.glines.read().await;
        for gline in glines.values() {
            if gline.is_active && self.matches_mask(&gline.mask, user) {
                if gline.expire_time.map_or(true, |expire| current_time < expire) {
                    return Some(format!("GLINE: {}", gline.reason));
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
    
    /// Clean up expired GLINEs
    pub async fn cleanup_expired_glines(&self) -> Result<()> {
        if !self.config.auto_cleanup_expired {
            return Ok(());
        }
        
        let current_time = self.get_current_time();
        
        let mut glines = self.glines.write().await;
        glines.retain(|_, gline| {
            gline.expire_time.map_or(true, |expire| current_time < expire)
        });
        
        Ok(())
    }
}

#[async_trait]
impl Module for GlineModule {
    fn name(&self) -> &str {
        "gline"
    }
    
    fn description(&self) -> &str {
        "Provides global ban (GLINE) management functionality"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("{} module initialized", self.name());
        Ok(())
    }

    async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ModuleResult> {
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled),
        };

        match message.command {
            MessageType::Custom(ref cmd) if cmd == "GLINE" => {
                self.handle_gline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNGLINE" => {
                self.handle_ungline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_server_message(&mut self, _server: &str, _message: &Message) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }

    async fn handle_user_registration(&mut self, _user: &User) -> Result<()> {
        Ok(())
    }

    async fn handle_user_disconnection(&mut self, _user: &User) -> Result<()> {
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
        info!("GLINE module cleaned up");
        Ok(())
    }
}

impl Default for GlineModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpProvider for GlineModule {
    fn get_help_topics(&self) -> Vec<HelpTopic> {
        vec![
            HelpTopic {
                command: "GLINE".to_string(),
                syntax: "GLINE <mask> <reason> [duration]".to_string(),
                description: "Set a global ban".to_string(),
                oper_only: true,
                examples: vec![
                    "GLINE *@spam.com Spamming".to_string(),
                    "GLINE *@*.badhost.com 1d Bad host".to_string(),
                ],
                module_name: Some("gline".to_string()),
            },
            HelpTopic {
                command: "UNGLINE".to_string(),
                syntax: "UNGLINE <mask>".to_string(),
                description: "Remove a global ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNGLINE *@spam.com".to_string(),
                ],
                module_name: Some("gline".to_string()),
            },
        ]
    }
    
    fn get_command_help(&self, command: &str) -> Option<HelpTopic> {
        match command {
            "GLINE" => Some(HelpTopic {
                command: "GLINE".to_string(),
                syntax: "GLINE <mask> <reason> [duration]".to_string(),
                description: "Set a global ban".to_string(),
                oper_only: true,
                examples: vec![
                    "GLINE *@spam.com Spamming".to_string(),
                    "GLINE *@*.badhost.com 1d Bad host".to_string(),
                ],
                module_name: Some("gline".to_string()),
            }),
            "UNGLINE" => Some(HelpTopic {
                command: "UNGLINE".to_string(),
                syntax: "UNGLINE <mask>".to_string(),
                description: "Remove a global ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNGLINE *@spam.com".to_string(),
                ],
                module_name: Some("gline".to_string()),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gline_config_default() {
        let config = GlineConfig::default();
        assert_eq!(config.max_duration, 86400 * 7);
        assert!(config.allow_permanent_bans);
        assert!(config.require_operator);
    }
    
    #[test]
    fn test_parse_duration() {
        let module = GlineModule::new();
        
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
        let module = GlineModule::new();
        
        // Note: This is a simplified test - real implementation would use proper regex
        assert!(module.simple_wildcard_match(".*", "anything"));
        assert!(module.simple_wildcard_match("test.*", "test123"));
        assert!(module.simple_wildcard_match(".*test", "123test"));
        assert!(!module.simple_wildcard_match("test", "notest"));
    }
}
