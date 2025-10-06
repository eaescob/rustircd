//! XLINE Module
//! 
//! Provides extended line (XLINE) management functionality.
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

/// XLINE module for extended line management
pub struct XlineModule {
    /// Extended lines (XLINE)
    xlines: RwLock<HashMap<String, ExtendedLine>>,
    /// Configuration
    config: XlineConfig,
}

/// Extended line entry
#[derive(Debug, Clone)]
pub struct ExtendedLine {
    pub mask: String,
    pub reason: String,
    pub set_by: String,
    pub set_time: u64,
    pub expire_time: Option<u64>,
    pub is_active: bool,
    pub line_type: String,
}

/// Configuration for XLINE management
#[derive(Debug, Clone)]
pub struct XlineConfig {
    pub max_duration: u64, // in seconds
    pub allow_permanent_bans: bool,
    pub require_operator: bool,
    pub auto_cleanup_expired: bool,
}

impl Default for XlineConfig {
    fn default() -> Self {
        Self {
            max_duration: 86400 * 30, // 30 days
            allow_permanent_bans: true,
            require_operator: true,
            auto_cleanup_expired: true,
        }
    }
}

impl XlineModule {
    /// Create a new XLINE module
    pub fn new() -> Self {
        Self {
            xlines: RwLock::new(HashMap::new()),
            config: XlineConfig::default(),
        }
    }
    
    /// Create a new XLINE module with custom configuration
    pub fn with_config(config: XlineConfig) -> Self {
        Self {
            xlines: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// Handle XLINE command
    async fn handle_xline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            self.list_xlines(client, user).await?;
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
        
        self.add_xline(client, user, mask, &reason, duration).await?;
        Ok(())
    }
    
    /// Handle UNXLINE command
    async fn handle_unxline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["UNXLINE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let mask = &args[0];
        self.remove_xline(client, user, mask).await?;
        Ok(())
    }
    
    /// Add an XLINE
    async fn add_xline(&self, client: &Client, user: &User, mask: &str, reason: &str, duration: Option<u64>) -> Result<()> {
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        if let Some(dur) = duration {
            if dur > self.config.max_duration {
                client.send_numeric(NumericReply::ErrInvalidDuration, &[&format!("Maximum duration is {} seconds", self.config.max_duration)])?;
                return Ok(());
            }
        }
        
        let xline = ExtendedLine {
            mask: mask.to_string(),
            reason: reason.to_string(),
            set_by: user.nickname().to_string(),
            set_time: current_time,
            expire_time,
            is_active: true,
            line_type: "XLINE".to_string(),
        };
        
        let mut xlines = self.xlines.write().await;
        xlines.insert(mask.to_string(), xline);
        
        client.send_numeric(NumericReply::RplXline, &[mask, reason, &format!("Set by {}", user.nickname())])?;
        
        info!("XLINE added: {} by {} - {}", mask, user.nickname(), reason);
        Ok(())
    }
    
    /// Remove an XLINE
    async fn remove_xline(&self, client: &Client, user: &User, mask: &str) -> Result<()> {
        let mut xlines = self.xlines.write().await;
        
        if xlines.remove(mask).is_some() {
            client.send_numeric(NumericReply::RplXline, &[mask, "Removed", &format!("Removed by {}", user.nickname())])?;
            info!("XLINE removed: {} by {}", mask, user.nickname());
        } else {
            client.send_numeric(NumericReply::ErrNoSuchXline, &[mask, "No such XLINE"])?;
        }
        
        Ok(())
    }
    
    /// List XLINEs
    async fn list_xlines(&self, client: &Client, user: &User) -> Result<()> {
        let xlines = self.xlines.read().await;
        
        if xlines.is_empty() {
            client.send_numeric(NumericReply::RplXline, &["*", "No XLINEs set"])?;
            return Ok(());
        }
        
        for xline in xlines.values() {
            let expire_info = if let Some(expire) = xline.expire_time {
                format!("Expires: {}", self.format_time(expire))
            } else {
                "Permanent".to_string()
            };
            
            client.send_numeric(NumericReply::RplXline, &[
                &xline.mask, 
                &xline.reason, 
                &format!("Set by {} at {} - {}", xline.set_by, self.format_time(xline.set_time), expire_info)
            ])?;
        }
        
        client.send_numeric(NumericReply::RplEndOfXlines, &["End of XLINE list"])?;
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
    
    /// Check if a user matches any active XLINEs
    pub async fn check_user_xline(&self, user: &User) -> Option<String> {
        let current_time = self.get_current_time();
        
        let xlines = self.xlines.read().await;
        for xline in xlines.values() {
            if xline.is_active && self.matches_mask(&xline.mask, user) {
                if xline.expire_time.map_or(true, |expire| current_time < expire) {
                    return Some(format!("XLINE: {}", xline.reason));
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
    
    /// Clean up expired XLINEs
    pub async fn cleanup_expired_xlines(&self) -> Result<()> {
        if !self.config.auto_cleanup_expired {
            return Ok(());
        }
        
        let current_time = self.get_current_time();
        
        let mut xlines = self.xlines.write().await;
        xlines.retain(|_, xline| {
            xline.expire_time.map_or(true, |expire| current_time < expire)
        });
        
        Ok(())
    }
}

#[async_trait]
impl Module for XlineModule {
    fn name(&self) -> &str {
        "xline"
    }
    
    fn description(&self) -> &str {
        "Provides extended line (XLINE) management functionality"
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
            MessageType::Custom(ref cmd) if cmd == "XLINE" => {
                self.handle_xline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNXLINE" => {
                self.handle_unxline(client, user, &message.params).await?;
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
        info!("XLINE module cleaned up");
        Ok(())
    }
}

impl Default for XlineModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpProvider for XlineModule {
    fn get_help_topics(&self) -> Vec<HelpTopic> {
        vec![
            HelpTopic {
                command: "XLINE".to_string(),
                syntax: "XLINE <mask> <reason> [duration]".to_string(),
                description: "Set an extended line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "XLINE *@baduser.com Extended ban".to_string(),
                ],
                module_name: Some("xline".to_string()),
            },
            HelpTopic {
                command: "UNXLINE".to_string(),
                syntax: "UNXLINE <mask>".to_string(),
                description: "Remove an extended line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNXLINE *@baduser.com".to_string(),
                ],
                module_name: Some("xline".to_string()),
            },
        ]
    }
    
    fn get_command_help(&self, command: &str) -> Option<HelpTopic> {
        match command {
            "XLINE" => Some(HelpTopic {
                command: "XLINE".to_string(),
                syntax: "XLINE <mask> <reason> [duration]".to_string(),
                description: "Set an extended line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "XLINE *@baduser.com Extended ban".to_string(),
                ],
                module_name: Some("xline".to_string()),
            }),
            "UNXLINE" => Some(HelpTopic {
                command: "UNXLINE".to_string(),
                syntax: "UNXLINE <mask>".to_string(),
                description: "Remove an extended line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNXLINE *@baduser.com".to_string(),
                ],
                module_name: Some("xline".to_string()),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_xline_config_default() {
        let config = XlineConfig::default();
        assert_eq!(config.max_duration, 86400 * 30);
        assert!(config.allow_permanent_bans);
        assert!(config.require_operator);
    }
    
    #[test]
    fn test_parse_duration() {
        let module = XlineModule::new();
        
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
        let module = XlineModule::new();
        
        // Note: This is a simplified test - real implementation would use proper regex
        assert!(module.simple_wildcard_match(".*", "anything"));
        assert!(module.simple_wildcard_match("test.*", "test123"));
        assert!(module.simple_wildcard_match(".*test", "123test"));
        assert!(!module.simple_wildcard_match("test", "notest"));
    }
}
