//! Monitor Module
//! 
//! Provides user notification system for monitoring when users come online/offline.
//! Based on Ratbox's m_monitor.c module.

use rustircd_core::{
    async_trait, Client, Message, MessageType, Module, Result, User, ModuleNumericManager, ModuleNumericClient,
    define_module_numerics,
};
use rustircd_core::module::{ModuleResult, ModuleContext};
use tracing::{debug, info};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use crate::help::{HelpProvider, HelpTopic};

/// Monitor system module that tracks user online/offline status
pub struct MonitorModule {
    /// Map of user nicknames to the clients monitoring them
    monitored_users: RwLock<HashMap<String, HashSet<uuid::Uuid>>>,
    /// Map of client IDs to the users they are monitoring
    client_monitors: RwLock<HashMap<uuid::Uuid, HashSet<String>>>,
    /// Module-specific numeric manager
    numeric_manager: ModuleNumericManager,
}

impl MonitorModule {
    /// Create a new monitor module
    pub fn new() -> Self {
        Self {
            monitored_users: RwLock::new(HashMap::new()),
            client_monitors: RwLock::new(HashMap::new()),
            numeric_manager: ModuleNumericManager::new(),
        }
    }
    
    /// Send a module-specific numeric reply
    fn send_module_numeric(&self, client: &Client, numeric: &str, params: &[&str]) -> Result<()> {
        client.send_module_numeric(&self.numeric_manager, numeric, params)
    }
    
    /// Add a user to monitor list for a client
    async fn add_monitor(&self, client_id: uuid::Uuid, nickname: &str) -> Result<()> {
        let mut monitored_users = self.monitored_users.write().await;
        let mut client_monitors = self.client_monitors.write().await;
        
        // Add to monitored users map
        monitored_users
            .entry(nickname.to_string())
            .or_insert_with(HashSet::new)
            .insert(client_id);
        
        // Add to client monitors map
        client_monitors
            .entry(client_id)
            .or_insert_with(HashSet::new)
            .insert(nickname.to_string());
        
        debug!("Added monitor: client {} monitoring {}", client_id, nickname);
        Ok(())
    }
    
    /// Remove a user from monitor list for a client
    async fn remove_monitor(&self, client_id: uuid::Uuid, nickname: &str) -> Result<()> {
        let mut monitored_users = self.monitored_users.write().await;
        let mut client_monitors = self.client_monitors.write().await;
        
        // Remove from monitored users map
        if let Some(monitors) = monitored_users.get_mut(nickname) {
            monitors.remove(&client_id);
            if monitors.is_empty() {
                monitored_users.remove(nickname);
            }
        }
        
        // Remove from client monitors map
        if let Some(monitored) = client_monitors.get_mut(&client_id) {
            monitored.remove(nickname);
        }
        
        debug!("Removed monitor: client {} no longer monitoring {}", client_id, nickname);
        Ok(())
    }
    
    /// Clear all monitors for a client
    async fn clear_monitors(&self, client_id: uuid::Uuid) -> Result<()> {
        let mut monitored_users = self.monitored_users.write().await;
        let mut client_monitors = self.client_monitors.write().await;
        
        // Get all monitored users for this client
        if let Some(monitored) = client_monitors.remove(&client_id) {
            // Remove this client from all monitored users
            for nickname in monitored {
                if let Some(monitors) = monitored_users.get_mut(&nickname) {
                    monitors.remove(&client_id);
                    if monitors.is_empty() {
                        monitored_users.remove(&nickname);
                    }
                }
            }
        }
        
        debug!("Cleared all monitors for client {}", client_id);
        Ok(())
    }
    
    /// Get list of users being monitored by a client
    async fn get_monitored_users(&self, client_id: uuid::Uuid) -> Vec<String> {
        let client_monitors = self.client_monitors.read().await;
        client_monitors
            .get(&client_id)
            .map(|monitored| monitored.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Notify monitors when a user comes online
    pub async fn notify_user_online(&self, nickname: &str, _user: &User) -> Result<()> {
        let monitored_users = self.monitored_users.read().await;
        
        if let Some(monitors) = monitored_users.get(nickname) {
            for client_id in monitors {
                // Implement notification to client
                // TODO: Integrate with client manager for full notification support
                
                // For now, log the notification that would be sent
                // In production, this would:
                // 1. Get client connection from client manager
                // 2. Send RPL_IS_ONLINE numeric message
                // 3. Handle errors if client is no longer connected
                
                debug!("Notifying client {} that {} is online", client_id, nickname);
                tracing::info!("MONITOR: Would send online notification for {} to client {}", nickname, client_id);
            }
        }
        
        Ok(())
    }
    
    /// Notify monitors when a user goes offline
    pub async fn notify_user_offline(&self, nickname: &str) -> Result<()> {
        let monitored_users = self.monitored_users.read().await;
        
        if let Some(monitors) = monitored_users.get(nickname) {
            for client_id in monitors {
                // Implement notification to client
                // TODO: Integrate with client manager for full notification support
                
                // For now, log the notification that would be sent
                // In production, this would:
                // 1. Get client connection from client manager
                // 2. Send RPL_IS_OFFLINE numeric message
                // 3. Handle errors if client is no longer connected
                
                debug!("Notifying client {} that {} is offline", client_id, nickname);
                tracing::info!("MONITOR: Would send offline notification for {} to client {}", nickname, client_id);
            }
        }
        
        Ok(())
    }
    
    /// Handle MONITOR command
    async fn handle_monitor(&self, client: &Client, args: &[String]) -> Result<()> {
        if args.is_empty() {
            // Show current monitor list
            self.show_monitor_list(client).await?;
            return Ok(());
        }
        
        let subcommand = &args[0].to_uppercase();
        
        match subcommand.as_str() {
            "+" => {
                // Add users to monitor list
                if args.len() < 2 {
                    self.send_module_numeric(client, "ERR_NEEDMOREPARAMS", &["MONITOR", "Not enough parameters"])?;
                    return Ok(());
                }
                
                let nicknames = args[1].split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>();
                self.add_monitors(client, &nicknames).await?;
            }
            "-" => {
                // Remove users from monitor list
                if args.len() < 2 {
                    self.send_module_numeric(client, "ERR_NEEDMOREPARAMS", &["MONITOR", "Not enough parameters"])?;
                    return Ok(());
                }
                
                let nicknames = args[1].split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>();
                self.remove_monitors(client, &nicknames).await?;
            }
            "C" => {
                // Clear monitor list
                self.clear_monitors(client.id()).await?;
                self.send_module_numeric(client, "RPL_MONOFFLINE", &["*", "Monitor list cleared"])?;
            }
            "L" => {
                // List current monitor list
                self.show_monitor_list(client).await?;
            }
            "S" => {
                // Show monitor status
                self.show_monitor_status(client).await?;
            }
            _ => {
                self.send_module_numeric(client, "ERR_UNKNOWNCOMMAND", &[subcommand, "Unknown MONITOR subcommand"])?;
            }
        }
        
        Ok(())
    }
    
    /// Add multiple users to monitor list
    async fn add_monitors(&self, client: &Client, nicknames: &[String]) -> Result<()> {
        let mut added_count = 0;
        let mut already_monitored = Vec::new();
        
        for nickname in nicknames {
            if nickname.is_empty() {
                continue;
            }
            
            // Check if already monitoring this user
            let current_monitors = self.get_monitored_users(client.id()).await;
            if current_monitors.contains(nickname) {
                already_monitored.push(nickname.clone());
                continue;
            }
            
            // Add to monitor list
            self.add_monitor(client.id(), nickname).await?;
            added_count += 1;
        }
        
        // Send response
        if added_count > 0 {
            self.send_module_numeric(client, "RPL_MONONLINE", &[&format!("{}", added_count), "users added to monitor list"])?;
        }
        
        if !already_monitored.is_empty() {
            let already_list = already_monitored.join(",");
            self.send_module_numeric(client, "RPL_MONOFFLINE", &[&already_list, "already being monitored"])?;
        }
        
        Ok(())
    }
    
    /// Remove multiple users from monitor list
    async fn remove_monitors(&self, client: &Client, nicknames: &[String]) -> Result<()> {
        let mut removed_count = 0;
        let mut not_monitored = Vec::new();
        
        for nickname in nicknames {
            if nickname.is_empty() {
                continue;
            }
            
            // Check if monitoring this user
            let current_monitors = self.get_monitored_users(client.id()).await;
            if !current_monitors.contains(nickname) {
                not_monitored.push(nickname.clone());
                continue;
            }
            
            // Remove from monitor list
            self.remove_monitor(client.id(), nickname).await?;
            removed_count += 1;
        }
        
        // Send response
        if removed_count > 0 {
            self.send_module_numeric(client, "RPL_MONOFFLINE", &[&format!("{}", removed_count), "users removed from monitor list"])?;
        }
        
        if !not_monitored.is_empty() {
            let not_list = not_monitored.join(",");
            self.send_module_numeric(client, "RPL_MONONLINE", &[&not_list, "not being monitored"])?;
        }
        
        Ok(())
    }
    
    /// Show current monitor list
    async fn show_monitor_list(&self, client: &Client) -> Result<()> {
        let monitored_users = self.get_monitored_users(client.id()).await;
        
        if monitored_users.is_empty() {
            self.send_module_numeric(client, "RPL_MONOFFLINE", &["*", "No users being monitored"])?;
            return Ok(());
        }
        
        // Send monitor list in chunks (IRC line length limit)
        const MAX_NICKNAMES_PER_LINE: usize = 20;
        let mut current_line = Vec::new();
        
        for nickname in monitored_users {
            current_line.push(nickname);
            
            if current_line.len() >= MAX_NICKNAMES_PER_LINE {
                let line = current_line.join(",");
                self.send_module_numeric(client, "RPL_MONONLINE", &[&line, "monitored users"])?;
                current_line.clear();
            }
        }
        
        if !current_line.is_empty() {
            let line = current_line.join(",");
            self.send_module_numeric(client, "RPL_MONONLINE", &[&line, "monitored users"])?;
        }
        
        self.send_module_numeric(client, "RPL_ENDOFMONLIST", &["End of MONITOR list"])?;
        
        Ok(())
    }
    
    /// Show monitor status
    async fn show_monitor_status(&self, client: &Client) -> Result<()> {
        let monitored_users = self.get_monitored_users(client.id()).await;
        let count = monitored_users.len();
        
        self.send_module_numeric(client, "RPL_MONONLINE", &[&format!("{}", count), "users in monitor list"])?;
        
        if count > 0 {
            self.send_module_numeric(client, "RPL_MONOFFLINE", &["*", "Use MONITOR L to list monitored users"])?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl Module for MonitorModule {
    fn name(&self) -> &str {
        "monitor"
    }
    
    fn description(&self) -> &str {
        "Provides user notification system for monitoring online/offline status"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        match message.command {
            MessageType::Custom(ref cmd) if cmd == "MONITOR" => {
                self.handle_monitor(client, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            _ => {
                Ok(ModuleResult::NotHandled)
            }
        }
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("Monitor module initialized");
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        info!("Monitor module cleaned up");
        Ok(())
    }
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, user: &User, context: &ModuleContext) -> Result<()> {
        // Implement user online notifications
        // TODO: Integrate with full client notification system
        
        let nickname = user.nickname();
        tracing::debug!("User {} registered, checking for monitors", nickname);
        
        // Get clients monitoring this user
        let monitored_users = self.monitored_users.read().await;
        if let Some(monitors) = monitored_users.get(nickname) {
            for client_id in monitors {
                // Send notification to monitoring client
                // In production, this would:
                // 1. Get the client connection from context
                // 2. Send RPL_IS_ONLINE numeric to the client
                // 3. Format proper IRC message with server prefix
                
                tracing::info!("Would notify client {} that {} is online", client_id, nickname);
                
                // In production, would use:
                // if let Some(client) = context.client_connections.read().await.get(client_id) {
                //     client.send_numeric(NumericReply::RplIsOn, &[nickname])?;
                // }
            }
        }
        
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, user: &User, context: &ModuleContext) -> Result<()> {
        // Implement user offline notifications
        // TODO: Integrate with full client notification system
        
        let nickname = user.nickname();
        tracing::debug!("User {} disconnected, checking for monitors", nickname);
        
        // Get clients monitoring this user
        let monitored_users = self.monitored_users.read().await;
        if let Some(monitors) = monitored_users.get(nickname) {
            for client_id in monitors {
                // Send notification to monitoring client
                // In production, this would:
                // 1. Get the client connection from context
                // 2. Send RPL_IS_OFFLINE numeric to the client
                // 3. Format proper IRC message with server prefix
                
                tracing::info!("Would notify client {} that {} is offline", client_id, nickname);
                
                // In production, would use:
                // if let Some(client) = context.client_connections.read().await.get(client_id) {
                //     client.send_numeric(NumericReply::RplIsOff, &[nickname])?;
                // }
            }
        }
        
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
    
    async fn handle_stats_query(&mut self, _query: &str, _client_id: uuid::Uuid, _server: Option<&rustircd_core::Server>) -> Result<Vec<rustircd_core::module::ModuleStatsResponse>> {
        Ok(vec![])
    }
    
    fn get_stats_queries(&self) -> Vec<String> {
        vec![]
    }
    
    fn register_numerics(&self, manager: &mut ModuleNumericManager) -> Result<()> {
        // Register monitor-specific numerics
        define_module_numerics!(monitor, manager, {
            RPL_MONONLINE = 730,
            RPL_MONOFFLINE = 731,
            RPL_ENDOFMONLIST = 732,
            ERR_NEEDMOREPARAMS = 461,
            ERR_UNKNOWNCOMMAND = 421
        });
        Ok(())
    }
}

impl Default for MonitorModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpProvider for MonitorModule {
    fn get_help_topics(&self) -> Vec<HelpTopic> {
        vec![
            HelpTopic {
                command: "MONITOR".to_string(),
                syntax: "MONITOR [+<users>|-<users>|C|L|S]".to_string(),
                description: "Monitor users for online/offline notifications".to_string(),
                oper_only: false,
                examples: vec![
                    "MONITOR +alice,bob".to_string(),
                    "MONITOR -alice".to_string(),
                    "MONITOR C".to_string(),
                    "MONITOR L".to_string(),
                    "MONITOR S".to_string(),
                ],
                module_name: Some("monitor".to_string()),
            },
        ]
    }
    
    fn get_command_help(&self, command: &str) -> Option<HelpTopic> {
        if command == "MONITOR" {
            Some(HelpTopic {
                command: "MONITOR".to_string(),
                syntax: "MONITOR [+<users>|-<users>|C|L|S]".to_string(),
                description: "Monitor users for online/offline notifications".to_string(),
                oper_only: false,
                examples: vec![
                    "MONITOR +alice,bob".to_string(),
                    "MONITOR -alice".to_string(),
                    "MONITOR C".to_string(),
                    "MONITOR L".to_string(),
                    "MONITOR S".to_string(),
                ],
                module_name: Some("monitor".to_string()),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_monitor_module_creation() {
        let module = MonitorModule::new();
        let monitored_users = module.get_monitored_users("test_client").await;
        assert!(monitored_users.is_empty());
    }
    
    #[tokio::test]
    async fn test_add_remove_monitor() {
        let module = MonitorModule::new();
        
        // Add monitor
        module.add_monitor("client1", "alice").await.unwrap();
        let monitored = module.get_monitored_users("client1").await;
        assert!(monitored.contains("alice"));
        
        // Remove monitor
        module.remove_monitor("client1", "alice").await.unwrap();
        let monitored = module.get_monitored_users("client1").await;
        assert!(!monitored.contains("alice"));
    }
    
    #[tokio::test]
    async fn test_clear_monitors() {
        let module = MonitorModule::new();
        
        // Add multiple monitors
        module.add_monitor("client1", "alice").await.unwrap();
        module.add_monitor("client1", "bob").await.unwrap();
        
        let monitored = module.get_monitored_users("client1").await;
        assert_eq!(monitored.len(), 2);
        
        // Clear all monitors
        module.clear_monitors("client1").await.unwrap();
        let monitored = module.get_monitored_users("client1").await;
        assert!(monitored.is_empty());
    }
}
