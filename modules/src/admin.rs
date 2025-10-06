//! Administrative Module
//! 
//! Provides administrative commands including ADMIN, ADMINWALL, and LOCops.
//! Based on Ratbox's administrative modules.

use rustircd_core::{
    async_trait, Client, Message, MessageType, Module, ModuleManager,
    ModuleNumericManager, module::{ModuleResult, ModuleStatsResponse},
    NumericReply, Result, User
};
use tracing::{debug, info, warn, error};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Administrative module for server administration
pub struct AdminModule {
    /// Server administrative information
    admin_info: AdminInfo,
    /// Local operator commands
    locops_enabled: bool,
    /// Administrative wall message history
    admin_wall_history: RwLock<Vec<AdminWallMessage>>,
    /// Maximum admin wall history size
    max_wall_history: usize,
}

/// Server administrative information
#[derive(Debug, Clone)]
pub struct AdminInfo {
    pub server_name: String,
    pub server_description: String,
    pub server_version: String,
    pub server_location: String,
    pub admin_name: String,
    pub admin_nickname: String,
    pub admin_email: String,
    pub admin_location: String,
    pub server_url: String,
    pub server_contact: String,
}

/// Administrative wall message
#[derive(Debug, Clone)]
pub struct AdminWallMessage {
    pub message: String,
    pub sent_by: String,
    pub timestamp: u64,
    pub target_servers: Option<Vec<String>>,
}

impl Default for AdminInfo {
    fn default() -> Self {
        Self {
            server_name: "rustircd.example.com".to_string(),
            server_description: "Rust IRC Daemon".to_string(),
            server_version: "rustircd-1.0.0".to_string(),
            server_location: "Unknown".to_string(),
            admin_name: "Administrator".to_string(),
            admin_nickname: "admin".to_string(),
            admin_email: "admin@example.com".to_string(),
            admin_location: "Unknown".to_string(),
            server_url: "https://github.com/rustircd/rustircd".to_string(),
            server_contact: "admin@example.com".to_string(),
        }
    }
}

impl AdminModule {
    /// Create a new admin module with default information
    pub fn new() -> Self {
        Self {
            admin_info: AdminInfo::default(),
            locops_enabled: true,
            admin_wall_history: RwLock::new(Vec::new()),
            max_wall_history: 100,
        }
    }
    
    /// Create a new admin module with custom information
    pub fn with_info(admin_info: AdminInfo) -> Self {
        Self {
            admin_info,
            locops_enabled: true,
            admin_wall_history: RwLock::new(Vec::new()),
            max_wall_history: 100,
        }
    }
    
    /// Handle ADMIN command
    async fn handle_admin(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        let target_server = if args.is_empty() {
            None
        } else {
            Some(&args[0])
        };
        
        // Send server administrative information
        client.send_numeric(NumericReply::RplAdminMe, &[&self.admin_info.server_name, "Administrative info"])?;
        client.send_numeric(NumericReply::RplAdminLoc1, &[&self.admin_info.server_location, "Server location"])?;
        client.send_numeric(NumericReply::RplAdminLoc2, &[&self.admin_info.server_description, "Server description"])?;
        client.send_numeric(NumericReply::RplAdminEmail, &[&self.admin_info.admin_email, "Administrator email"])?;
        
        // Send additional admin information
        client.send_numeric(NumericReply::RplAdminLoc1, &[&self.admin_info.admin_name, "Administrator name"])?;
        client.send_numeric(NumericReply::RplAdminLoc2, &[&self.admin_info.admin_nickname, "Administrator nickname"])?;
        client.send_numeric(NumericReply::RplAdminLoc1, &[&self.admin_info.admin_location, "Administrator location"])?;
        client.send_numeric(NumericReply::RplAdminLoc2, &[&self.admin_info.server_url, "Server URL"])?;
        client.send_numeric(NumericReply::RplAdminEmail, &[&self.admin_info.server_contact, "Server contact"])?;
        
        Ok(())
    }
    
    /// Handle ADMINWALL command
    async fn handle_adminwall(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["ADMINWALL", "Not enough parameters"])?;
            return Ok(());
        }
        
        let message = args.join(" ");
        
        // Create admin wall message
        let admin_wall = AdminWallMessage {
            message: message.clone(),
            sent_by: user.nickname().to_string(),
            timestamp: self.get_current_time(),
            target_servers: None, // Broadcast to all servers
        };
        
        // Store in history
        {
            let mut history = self.admin_wall_history.write().await;
            history.push(admin_wall.clone());
            
            // Trim history if too large
            let history_len = history.len();
            if history_len > self.max_wall_history {
                history.drain(0..history_len - self.max_wall_history);
            }
        }
        
        // Send admin wall message to all operators
        // TODO: Implement broadcasting to all operators on the network
        client.send_numeric(NumericReply::RplAdminWall, &[&format!("ADMINWALL from {}: {}", user.nickname(), message)])?;
        
        info!("ADMINWALL from {}: {}", user.nickname(), message);
        
        Ok(())
    }
    
    /// Handle REHASH command
    async fn handle_rehash(&self, client: &Client, user: &User, args: &[String], server: Option<&rustircd_core::Server>) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            // No parameters - reload main configuration
            client.send_numeric(NumericReply::RplLocops, &["REHASH: Reloading main configuration..."])?;
            
            if let Some(server) = server {
                match server.rehash_service().reload_main_config().await {
                    Ok(_) => {
                        client.send_numeric(NumericReply::RplLocops, &["REHASH: Main configuration reloaded successfully"])?;
                        info!("REHASH: Main configuration reloaded by {}", user.nickname());
                    }
                    Err(e) => {
                        client.send_numeric(NumericReply::RplLocops, &[&format!("REHASH: Failed to reload main configuration: {}", e)])?;
                        error!("REHASH: Failed to reload main configuration by {}: {}", user.nickname(), e);
                    }
                }
            } else {
                client.send_numeric(NumericReply::RplLocops, &["REHASH: Server reference not available"])?;
            }
            return Ok(());
        }
        
        let parameter = &args[0].to_uppercase();
        
        if let Some(server) = server {
            match parameter.as_str() {
                "SSL" => {
                    client.send_numeric(NumericReply::RplLocops, &["REHASH SSL: Reloading TLS settings..."])?;
                    match server.rehash_service().reload_ssl().await {
                        Ok(_) => {
                            client.send_numeric(NumericReply::RplLocops, &["REHASH SSL: TLS settings reloaded successfully"])?;
                            info!("REHASH SSL: TLS settings reloaded by {}", user.nickname());
                        }
                        Err(e) => {
                            client.send_numeric(NumericReply::RplLocops, &[&format!("REHASH SSL: Failed to reload TLS settings: {}", e)])?;
                            error!("REHASH SSL: Failed to reload TLS settings by {}: {}", user.nickname(), e);
                        }
                    }
                }
                "MOTD" => {
                    client.send_numeric(NumericReply::RplLocops, &["REHASH MOTD: Reloading MOTD file..."])?;
                    match server.rehash_service().reload_motd().await {
                        Ok(_) => {
                            client.send_numeric(NumericReply::RplLocops, &["REHASH MOTD: MOTD file reloaded successfully"])?;
                            info!("REHASH MOTD: MOTD file reloaded by {}", user.nickname());
                        }
                        Err(e) => {
                            client.send_numeric(NumericReply::RplLocops, &[&format!("REHASH MOTD: Failed to reload MOTD file: {}", e)])?;
                            error!("REHASH MOTD: Failed to reload MOTD file by {}: {}", user.nickname(), e);
                        }
                    }
                }
                "MODULES" => {
                    client.send_numeric(NumericReply::RplLocops, &["REHASH MODULES: Reloading all modules..."])?;
                    match server.rehash_service().reload_modules().await {
                        Ok(_) => {
                            client.send_numeric(NumericReply::RplLocops, &["REHASH MODULES: All modules reloaded successfully"])?;
                            info!("REHASH MODULES: All modules reloaded by {}", user.nickname());
                        }
                        Err(e) => {
                            client.send_numeric(NumericReply::RplLocops, &[&format!("REHASH MODULES: Failed to reload modules: {}", e)])?;
                            error!("REHASH MODULES: Failed to reload modules by {}: {}", user.nickname(), e);
                        }
                    }
                }
                _ => {
                    client.send_numeric(NumericReply::ErrUnknownCommand, &[parameter, "Unknown REHASH parameter. Use: SSL, MOTD, MODULES, or no parameter for main config"])?;
                }
            }
        } else {
            client.send_numeric(NumericReply::RplLocops, &["REHASH: Server reference not available"])?;
        }
        
        Ok(())
    }

    /// Handle LOCops command (Local Operator commands)
    async fn handle_locops(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if !self.locops_enabled {
            client.send_numeric(NumericReply::ErrDisabled, &["LOCops", "Local operator commands are disabled"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["LOCops", "Not enough parameters"])?;
            return Ok(());
        }
        
        let subcommand = &args[0].to_uppercase();
        
        match subcommand.as_str() {
            "LIST" => {
                self.list_locops(client, user).await?;
            }
            "HELP" => {
                self.show_locops_help(client, user).await?;
            }
            "STATS" => {
                self.show_locops_stats(client, user).await?;
            }
            "VERSION" => {
                self.show_locops_version(client, user).await?;
            }
            "UPTIME" => {
                self.show_locops_uptime(client, user).await?;
            }
            "CONFIG" => {
                self.show_locops_config(client, user).await?;
            }
            "REHASH" => {
                // Note: Server reference not available in LOCops context
                self.handle_rehash(client, user, &args[1..], None).await?;
            }
            _ => {
                client.send_numeric(NumericReply::ErrUnknownCommand, &[subcommand, "Unknown LOCops command"])?;
            }
        }
        
        Ok(())
    }
    
    /// List LOCops commands
    async fn list_locops(&self, client: &Client, user: &User) -> Result<()> {
        client.send_numeric(NumericReply::RplLocops, &["Available LOCops commands:"])?;
        client.send_numeric(NumericReply::RplLocops, &["  LIST - List available commands"])?;
        client.send_numeric(NumericReply::RplLocops, &["  HELP - Show help information"])?;
        client.send_numeric(NumericReply::RplLocops, &["  STATS - Show server statistics"])?;
        client.send_numeric(NumericReply::RplLocops, &["  VERSION - Show server version"])?;
        client.send_numeric(NumericReply::RplLocops, &["  UPTIME - Show server uptime"])?;
        client.send_numeric(NumericReply::RplLocops, &["  CONFIG - Show server configuration"])?;
        client.send_numeric(NumericReply::RplLocops, &["  REHASH - Reload configuration (SSL, MOTD, MODULES, or main config)"])?;
        client.send_numeric(NumericReply::RplEndOfLocops, &["End of LOCops commands"])?;
        
        Ok(())
    }
    
    /// Show LOCops help
    async fn show_locops_help(&self, client: &Client, user: &User) -> Result<()> {
        client.send_numeric(NumericReply::RplLocops, &["LOCops - Local Operator Commands"])?;
        client.send_numeric(NumericReply::RplLocops, &["These commands provide local server administration"])?;
        client.send_numeric(NumericReply::RplLocops, &["Use LOCops LIST to see available commands"])?;
        client.send_numeric(NumericReply::RplLocops, &["Use LOCops HELP <command> for detailed help"])?;
        
        Ok(())
    }
    
    /// Show LOCops statistics
    async fn show_locops_stats(&self, client: &Client, user: &User) -> Result<()> {
        // TODO: Get actual server statistics
        client.send_numeric(NumericReply::RplLocops, &["Server Statistics:"])?;
        client.send_numeric(NumericReply::RplLocops, &["  Uptime: Not implemented"])?;
        client.send_numeric(NumericReply::RplLocops, &["  Users: Not implemented"])?;
        client.send_numeric(NumericReply::RplLocops, &["  Channels: Not implemented"])?;
        client.send_numeric(NumericReply::RplLocops, &["  Servers: Not implemented"])?;
        client.send_numeric(NumericReply::RplLocops, &["  Memory: Not implemented"])?;
        
        Ok(())
    }
    
    /// Show LOCops version
    async fn show_locops_version(&self, client: &Client, user: &User) -> Result<()> {
        client.send_numeric(NumericReply::RplLocops, &[&format!("Server: {}", self.admin_info.server_name)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("Version: {}", self.admin_info.server_version)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("Description: {}", self.admin_info.server_description)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("Location: {}", self.admin_info.server_location)])?;
        
        Ok(())
    }
    
    /// Show LOCops uptime
    async fn show_locops_uptime(&self, client: &Client, user: &User) -> Result<()> {
        // TODO: Get actual server uptime
        client.send_numeric(NumericReply::RplLocops, &["Server uptime: Not implemented"])?;
        client.send_numeric(NumericReply::RplLocops, &["Start time: Not implemented"])?;
        client.send_numeric(NumericReply::RplLocops, &["Current time: Not implemented"])?;
        
        Ok(())
    }
    
    /// Show LOCops configuration
    async fn show_locops_config(&self, client: &Client, user: &User) -> Result<()> {
        client.send_numeric(NumericReply::RplLocops, &["Server Configuration:"])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("  Name: {}", self.admin_info.server_name)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("  Description: {}", self.admin_info.server_description)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("  Version: {}", self.admin_info.server_version)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("  Location: {}", self.admin_info.server_location)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("  Admin: {}", self.admin_info.admin_name)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("  Email: {}", self.admin_info.admin_email)])?;
        client.send_numeric(NumericReply::RplLocops, &[&format!("  URL: {}", self.admin_info.server_url)])?;
        
        Ok(())
    }
    
    /// Get current time as Unix timestamp
    fn get_current_time(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Update admin information
    pub async fn update_admin_info(&mut self, admin_info: AdminInfo) {
        self.admin_info = admin_info;
    }
    
    /// Get admin information
    pub fn get_admin_info(&self) -> &AdminInfo {
        &self.admin_info
    }
    
    /// Enable or disable LOCops
    pub fn set_locops_enabled(&mut self, enabled: bool) {
        self.locops_enabled = enabled;
    }
    
    /// Get admin wall history
    pub async fn get_admin_wall_history(&self) -> Vec<AdminWallMessage> {
        let history = self.admin_wall_history.read().await;
        history.clone()
    }
    
    /// Clear admin wall history
    pub async fn clear_admin_wall_history(&self) {
        let mut history = self.admin_wall_history.write().await;
        history.clear();
    }
}

#[async_trait]
impl Module for AdminModule {
    fn name(&self) -> &str {
        "admin"
    }
    
    fn description(&self) -> &str {
        "Provides administrative commands including ADMIN, ADMINWALL, and LOCops"
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
            MessageType::Admin => {
                self.handle_admin(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "ADMINWALL" => {
                self.handle_adminwall(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "LOCops" => {
                self.handle_locops(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "REHASH" => {
                self.handle_rehash(client, user, &message.params, None).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }
    
    async fn handle_message_with_server(&mut self, client: &Client, message: &Message, server: Option<&rustircd_core::Server>) -> Result<ModuleResult> {
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled),
        };

        match message.command {
            MessageType::Admin => {
                self.handle_admin(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "ADMINWALL" => {
                self.handle_adminwall(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "LOCops" => {
                self.handle_locops(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "REHASH" => {
                self.handle_rehash(client, user, &message.params, server).await?;
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

    fn register_numerics(&self, _manager: &mut ModuleNumericManager) -> Result<()> {
        Ok(())
    }
    
    fn get_stats_queries(&self) -> Vec<String> {
        vec![]
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("Admin module cleaned up");
        Ok(())
    }
}

impl Default for AdminModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_admin_info_default() {
        let admin_info = AdminInfo::default();
        assert_eq!(admin_info.server_name, "rustircd.example.com");
        assert_eq!(admin_info.server_description, "Rust IRC Daemon");
        assert_eq!(admin_info.admin_name, "Administrator");
    }
    
    #[test]
    fn test_admin_module_creation() {
        let module = AdminModule::new();
        assert_eq!(module.admin_info.server_name, "rustircd.example.com");
        assert!(module.locops_enabled);
        assert_eq!(module.max_wall_history, 100);
    }
    
    #[test]
    fn test_admin_module_with_custom_info() {
        let custom_info = AdminInfo {
            server_name: "custom.example.com".to_string(),
            server_description: "Custom IRC Server".to_string(),
            server_version: "custom-1.0.0".to_string(),
            server_location: "Custom Location".to_string(),
            admin_name: "Custom Admin".to_string(),
            admin_nickname: "customadmin".to_string(),
            admin_email: "custom@example.com".to_string(),
            admin_location: "Custom Admin Location".to_string(),
            server_url: "https://custom.example.com".to_string(),
            server_contact: "custom@example.com".to_string(),
        };
        
        let module = AdminModule::with_info(custom_info);
        assert_eq!(module.admin_info.server_name, "custom.example.com");
        assert_eq!(module.admin_info.admin_name, "Custom Admin");
    }
    
    #[tokio::test]
    async fn test_admin_wall_history() {
        let module = AdminModule::new();
        
        // Initially empty
        let history = module.get_admin_wall_history().await;
        assert!(history.is_empty());
        
        // Add a message (this would normally be done through handle_adminwall)
        let admin_wall = AdminWallMessage {
            message: "Test message".to_string(),
            sent_by: "admin".to_string(),
            timestamp: module.get_current_time(),
            target_servers: None,
        };
        
        {
            let mut history = module.admin_wall_history.write().await;
            history.push(admin_wall);
        }
        
        let history = module.get_admin_wall_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].message, "Test message");
        assert_eq!(history[0].sent_by, "admin");
    }
}
