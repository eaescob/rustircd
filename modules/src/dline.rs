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
    async fn handle_dline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
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
        
        self.add_dline(client, user, hostname, &reason, duration).await?;
        Ok(())
    }
    
    /// Handle UNDLINE command
    async fn handle_undline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["UNDLINE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let hostname = &args[0];
        self.remove_dline(client, user, hostname).await?;
        Ok(())
    }
    
    /// Add a DLINE
    async fn add_dline(&self, client: &Client, user: &User, hostname: &str, reason: &str, duration: Option<u64>) -> Result<()> {
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
        Ok(())
    }
    
    /// Remove a DLINE
    async fn remove_dline(&self, client: &Client, user: &User, hostname: &str) -> Result<()> {
        let mut dlines = self.dlines.write().await;
        
        if dlines.remove(hostname).is_some() {
            client.send_numeric(NumericReply::RplDline, &[hostname, "Removed", &format!("Removed by {}", user.nickname())])?;
            info!("DLINE removed: {} by {}", hostname, user.nickname());
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
        
        let mut dlines = self.dlines.write().await;
        dlines.retain(|_, dline| {
            dline.expire_time.map_or(true, |expire| current_time < expire)
        });
        
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

    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled),
        };

        match message.command {
            MessageType::Custom(ref cmd) if cmd == "DLINE" => {
                self.handle_dline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNDLINE" => {
                self.handle_undline(client, user, &message.params).await?;
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
