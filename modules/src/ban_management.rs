//! Ban Management Module
//! 
//! **DEPRECATED**: This module is deprecated in favor of separate modules for each ban type.
//! Please use the following modules instead:
//! - `gline` module for GLINE/UNGLINE commands
//! - `kline` module for KLINE/UNKLINE commands  
//! - `dline` module for DLINE/UNDLINE commands
//! - `xline` module for XLINE/UNXLINE commands
//! 
//! This module is kept for backward compatibility but will be removed in a future version.
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

/// Ban management module for various types of bans
pub struct BanManagementModule {
    /// Global bans (GLINE)
    glines: RwLock<HashMap<String, GlobalBan>>,
    /// Kill lines (KLINE) 
    klines: RwLock<HashMap<String, KillLine>>,
    /// DNS lines (DLINE)
    dlines: RwLock<HashMap<String, DnsLine>>,
    /// Extended lines (XLINE)
    xlines: RwLock<HashMap<String, ExtendedLine>>,
    /// Configuration
    config: BanConfig,
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

/// Configuration for ban management
#[derive(Debug, Clone)]
pub struct BanConfig {
    pub max_gline_duration: u64, // in seconds
    pub max_kline_duration: u64,
    pub max_dline_duration: u64,
    pub max_xline_duration: u64,
    pub allow_permanent_bans: bool,
    pub require_operator_for_global: bool,
    pub auto_cleanup_expired: bool,
}

impl Default for BanConfig {
    fn default() -> Self {
        Self {
            max_gline_duration: 86400 * 7, // 7 days
            max_kline_duration: 86400 * 30, // 30 days
            max_dline_duration: 86400 * 30, // 30 days
            max_xline_duration: 86400 * 30, // 30 days
            allow_permanent_bans: true,
            require_operator_for_global: true,
            auto_cleanup_expired: true,
        }
    }
}

impl BanManagementModule {
    /// Create a new ban management module
    pub fn new() -> Self {
        Self {
            glines: RwLock::new(HashMap::new()),
            klines: RwLock::new(HashMap::new()),
            dlines: RwLock::new(HashMap::new()),
            xlines: RwLock::new(HashMap::new()),
            config: BanConfig::default(),
        }
    }
    
    /// Create a new ban management module with custom configuration
    pub fn with_config(config: BanConfig) -> Self {
        Self {
            glines: RwLock::new(HashMap::new()),
            klines: RwLock::new(HashMap::new()),
            dlines: RwLock::new(HashMap::new()),
            xlines: RwLock::new(HashMap::new()),
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
    
    /// Handle KLINE command
    async fn handle_kline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
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
        
        self.add_kline(client, user, mask, &reason, duration).await?;
        Ok(())
    }
    
    /// Handle UNKLINE command
    async fn handle_unkline(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["UNKLINE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let mask = &args[0];
        self.remove_kline(client, user, mask).await?;
        Ok(())
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
    
    /// Add a GLINE
    async fn add_gline(&self, client: &Client, user: &User, mask: &str, reason: &str, duration: Option<u64>) -> Result<()> {
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        // Check duration limits
        if let Some(dur) = duration {
            if dur > self.config.max_gline_duration {
                client.send_numeric(NumericReply::ErrInvalidDuration, &[&format!("Maximum duration is {} seconds", self.config.max_gline_duration)])?;
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
    
    /// Add a KLINE
    async fn add_kline(&self, client: &Client, user: &User, mask: &str, reason: &str, duration: Option<u64>) -> Result<()> {
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        if let Some(dur) = duration {
            if dur > self.config.max_kline_duration {
                client.send_numeric(NumericReply::ErrInvalidDuration, &[&format!("Maximum duration is {} seconds", self.config.max_kline_duration)])?;
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
        Ok(())
    }
    
    /// Remove a KLINE
    async fn remove_kline(&self, client: &Client, user: &User, mask: &str) -> Result<()> {
        let mut klines = self.klines.write().await;
        
        if klines.remove(mask).is_some() {
            client.send_numeric(NumericReply::RplKline, &[mask, "Removed", &format!("Removed by {}", user.nickname())])?;
            info!("KLINE removed: {} by {}", mask, user.nickname());
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
    
    /// Add a DLINE
    async fn add_dline(&self, client: &Client, user: &User, hostname: &str, reason: &str, duration: Option<u64>) -> Result<()> {
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        if let Some(dur) = duration {
            if dur > self.config.max_dline_duration {
                client.send_numeric(NumericReply::ErrInvalidDuration, &[&format!("Maximum duration is {} seconds", self.config.max_dline_duration)])?;
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
    
    /// Add an XLINE
    async fn add_xline(&self, client: &Client, user: &User, mask: &str, reason: &str, duration: Option<u64>) -> Result<()> {
        let current_time = self.get_current_time();
        let expire_time = duration.map(|d| current_time + d);
        
        if let Some(dur) = duration {
            if dur > self.config.max_xline_duration {
                client.send_numeric(NumericReply::ErrInvalidDuration, &[&format!("Maximum duration is {} seconds", self.config.max_xline_duration)])?;
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
    
    /// Check if a user matches any active bans
    pub async fn check_user_bans(&self, user: &User) -> Option<String> {
        let current_time = self.get_current_time();
        
        // Check GLINEs
        let glines = self.glines.read().await;
        for gline in glines.values() {
            if gline.is_active && self.matches_mask(&gline.mask, user) {
                if gline.expire_time.map_or(true, |expire| current_time < expire) {
                    return Some(format!("GLINE: {}", gline.reason));
                }
            }
        }
        
        // Check KLINEs
        let klines = self.klines.read().await;
        for kline in klines.values() {
            if kline.is_active && self.matches_mask(&kline.mask, user) {
                if kline.expire_time.map_or(true, |expire| current_time < expire) {
                    return Some(format!("KLINE: {}", kline.reason));
                }
            }
        }
        
        // Check DLINEs
        let dlines = self.dlines.read().await;
        for dline in dlines.values() {
            if dline.is_active && user.hostname().contains(&dline.hostname) {
                if dline.expire_time.map_or(true, |expire| current_time < expire) {
                    return Some(format!("DLINE: {}", dline.reason));
                }
            }
        }
        
        // Check XLINEs
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
    
    /// Clean up expired bans
    pub async fn cleanup_expired_bans(&self) -> Result<()> {
        if !self.config.auto_cleanup_expired {
            return Ok(());
        }
        
        let current_time = self.get_current_time();
        
        // Clean up expired GLINEs
        {
            let mut glines = self.glines.write().await;
            glines.retain(|_, gline| {
                gline.expire_time.map_or(true, |expire| current_time < expire)
            });
        }
        
        // Clean up expired KLINEs
        {
            let mut klines = self.klines.write().await;
            klines.retain(|_, kline| {
                kline.expire_time.map_or(true, |expire| current_time < expire)
            });
        }
        
        // Clean up expired DLINEs
        {
            let mut dlines = self.dlines.write().await;
            dlines.retain(|_, dline| {
                dline.expire_time.map_or(true, |expire| current_time < expire)
            });
        }
        
        // Clean up expired XLINEs
        {
            let mut xlines = self.xlines.write().await;
            xlines.retain(|_, xline| {
                xline.expire_time.map_or(true, |expire| current_time < expire)
            });
        }
        
        Ok(())
    }
}

#[async_trait]
impl Module for BanManagementModule {
    fn name(&self) -> &str {
        "ban_management"
    }
    
    fn description(&self) -> &str {
        "Provides comprehensive ban management system (GLINE, KLINE, DLINE, XLINE)"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn init(&mut self) -> Result<()> {
        warn!("{} module is DEPRECATED. Please use separate gline, kline, dline, and xline modules instead.", self.name());
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
            MessageType::Custom(ref cmd) if cmd == "KLINE" => {
                self.handle_kline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNKLINE" => {
                self.handle_unkline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "DLINE" => {
                self.handle_dline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNDLINE" => {
                self.handle_undline(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
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
        info!("Ban management module cleaned up");
        Ok(())
    }
}

impl Default for BanManagementModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpProvider for BanManagementModule {
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
                module_name: Some("ban_management".to_string()),
            },
            HelpTopic {
                command: "UNGLINE".to_string(),
                syntax: "UNGLINE <mask>".to_string(),
                description: "Remove a global ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNGLINE *@spam.com".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            },
            HelpTopic {
                command: "KLINE".to_string(),
                syntax: "KLINE <mask> <reason> [duration]".to_string(),
                description: "Set a kill line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "KLINE *@baduser.com Bad user".to_string(),
                    "KLINE *@*.badhost.com 7d Bad host".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            },
            HelpTopic {
                command: "UNKLINE".to_string(),
                syntax: "UNKLINE <mask>".to_string(),
                description: "Remove a kill line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNKLINE *@baduser.com".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            },
            HelpTopic {
                command: "DLINE".to_string(),
                syntax: "DLINE <hostname> <reason> [duration]".to_string(),
                description: "Set a DNS line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "DLINE badhost.com Bad host".to_string(),
                    "DLINE *.badhost.com 30d Bad domain".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            },
            HelpTopic {
                command: "UNDLINE".to_string(),
                syntax: "UNDLINE <hostname>".to_string(),
                description: "Remove a DNS line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNDLINE badhost.com".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            },
            HelpTopic {
                command: "XLINE".to_string(),
                syntax: "XLINE <mask> <reason> [duration]".to_string(),
                description: "Set an extended line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "XLINE *@baduser.com Extended ban".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            },
            HelpTopic {
                command: "UNXLINE".to_string(),
                syntax: "UNXLINE <mask>".to_string(),
                description: "Remove an extended line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNXLINE *@baduser.com".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
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
                module_name: Some("ban_management".to_string()),
            }),
            "UNGLINE" => Some(HelpTopic {
                command: "UNGLINE".to_string(),
                syntax: "UNGLINE <mask>".to_string(),
                description: "Remove a global ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNGLINE *@spam.com".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            }),
            "KLINE" => Some(HelpTopic {
                command: "KLINE".to_string(),
                syntax: "KLINE <mask> <reason> [duration]".to_string(),
                description: "Set a kill line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "KLINE *@baduser.com Bad user".to_string(),
                    "KLINE *@*.badhost.com 7d Bad host".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            }),
            "UNKLINE" => Some(HelpTopic {
                command: "UNKLINE".to_string(),
                syntax: "UNKLINE <mask>".to_string(),
                description: "Remove a kill line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNKLINE *@baduser.com".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            }),
            "DLINE" => Some(HelpTopic {
                command: "DLINE".to_string(),
                syntax: "DLINE <hostname> <reason> [duration]".to_string(),
                description: "Set a DNS line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "DLINE badhost.com Bad host".to_string(),
                    "DLINE *.badhost.com 30d Bad domain".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            }),
            "UNDLINE" => Some(HelpTopic {
                command: "UNDLINE".to_string(),
                syntax: "UNDLINE <hostname>".to_string(),
                description: "Remove a DNS line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNDLINE badhost.com".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            }),
            "XLINE" => Some(HelpTopic {
                command: "XLINE".to_string(),
                syntax: "XLINE <mask> <reason> [duration]".to_string(),
                description: "Set an extended line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "XLINE *@baduser.com Extended ban".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            }),
            "UNXLINE" => Some(HelpTopic {
                command: "UNXLINE".to_string(),
                syntax: "UNXLINE <mask>".to_string(),
                description: "Remove an extended line ban".to_string(),
                oper_only: true,
                examples: vec![
                    "UNXLINE *@baduser.com".to_string(),
                ],
                module_name: Some("ban_management".to_string()),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ban_config_default() {
        let config = BanConfig::default();
        assert_eq!(config.max_gline_duration, 86400 * 7);
        assert!(config.allow_permanent_bans);
        assert!(config.require_operator_for_global);
    }
    
    #[test]
    fn test_parse_duration() {
        let module = BanManagementModule::new();
        
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
        let module = BanManagementModule::new();
        
        // Note: This is a simplified test - real implementation would use proper regex
        assert!(module.simple_wildcard_match(".*", "anything"));
        assert!(module.simple_wildcard_match("test.*", "test123"));
        assert!(module.simple_wildcard_match(".*test", "123test"));
        assert!(!module.simple_wildcard_match("test", "notest"));
    }
}
