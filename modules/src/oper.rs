//! Operator management module
//! 
//! This module provides operator authentication and management functionality,
//! moved from core to follow Solanum's modular architecture.

use rustircd_core::{User, Message, Client, Result, Error, NumericReply, Config, ModuleNumericManager, module::{ModuleContext, ModuleResult, ModuleStatsResponse}};
use rustircd_core::config::OperatorFlag;
use std::collections::HashSet;
use uuid::Uuid;
use async_trait::async_trait;
use tracing::info;

/// Operator module for handling operator authentication and privileges
pub struct OperModule {
    /// Module configuration
    config: OperConfig,
}

/// Configuration for the oper module
#[derive(Debug, Clone)]
pub struct OperConfig {
    /// Whether the oper module is enabled
    pub enabled: bool,
    /// Whether to require operator privileges for certain commands
    pub require_oper_for_connect: bool,
    /// Whether to show detailed server information to operators
    pub show_server_details_in_stats: bool,
    /// Whether to log operator actions
    pub log_operator_actions: bool,
}

impl Default for OperConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_oper_for_connect: true,
            show_server_details_in_stats: true,
            log_operator_actions: true,
        }
    }
}

impl OperModule {
    /// Create a new oper module
    pub fn new(config: OperConfig) -> Self {
        Self { config }
    }
    
    /// Handle OPER command with full database access
    pub async fn handle_oper(&self, client: &Client, message: &Message, config: &Config, context: &ModuleContext) -> Result<()> {
        if !self.config.enabled {
            let error_msg = rustircd_core::Message::new(
                rustircd_core::MessageType::Error,
                vec!["OPER command is disabled".to_string()]
            );
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        if message.params.len() < 2 {
            let error_msg = NumericReply::need_more_params("OPER");
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        let oper_name = &message.params[0];
        let password = &message.params[1];
        
        // Get username and host from client
        let username = client.username().unwrap_or_default();
        let host = &client.remote_addr;
        
        // Authenticate operator against config
        if let Some(operator_config) = config.authenticate_operator(oper_name, password, &username, host) {
            // Get the user from the database
            if let Some(mut user) = context.get_user_by_nick(oper_name) {
                // Set operator flags on the user
                let mut operator_flags = HashSet::new();
                for flag in &operator_config.flags {
                    operator_flags.insert(*flag);
                }
                
                // Grant operator privileges (this will set the +o mode securely)
                user.set_operator_flags(operator_flags.clone());
                
                // Update user in database
                context.update_user(user.clone())?;
                
                info!("Operator {} successfully authenticated with flags: {:?}", 
                      user.nick, operator_flags);
                
                // Send success message
                let success_msg = NumericReply::youre_oper();
                let _ = client.send(success_msg);
                
                // Send operator privileges information
                self.send_operator_privileges(client, &operator_flags).await?;
                
                if self.config.log_operator_actions {
                    tracing::info!("User {} authenticated as operator with flags: {:?}", 
                        user.nick, operator_flags);
                }
            } else {
                // User not found in database
                let error_msg = NumericReply::password_mismatch();
                let _ = client.send(error_msg);
                
                if self.config.log_operator_actions {
                    tracing::warn!("Failed operator authentication - user not in database: {}", oper_name);
                }
            }
        } else {
            // Authentication failed
            let error_msg = NumericReply::password_mismatch();
            let _ = client.send(error_msg);
            
            if self.config.log_operator_actions {
                tracing::warn!("Failed operator authentication attempt for user {} from {}", 
                    oper_name, host);
            }
        }
        
        Ok(())
    }
    
    /// Send operator privileges information to client
    async fn send_operator_privileges(&self, client: &Client, flags: &HashSet<OperatorFlag>) -> Result<()> {
        let mut privileges = Vec::new();
        
        for flag in flags {
            match flag {
                OperatorFlag::GlobalOper => privileges.push("Global Operator"),
                OperatorFlag::LocalOper => privileges.push("Local Operator"),
                OperatorFlag::RemoteConnect => privileges.push("Remote Connect"),
                OperatorFlag::LocalConnect => privileges.push("Local Connect"),
                OperatorFlag::Administrator => privileges.push("Administrator"),
                OperatorFlag::Spy => privileges.push("Spy"),
                OperatorFlag::Squit => privileges.push("SQUIT"),
            }
        }
        
        if !privileges.is_empty() {
            let msg = format!("Your operator privileges: {}", privileges.join(", "));
            let notice = rustircd_core::Message::new(
                rustircd_core::MessageType::Notice,
                vec!["*".to_string(), msg]
            );
            let _ = client.send(notice);
        }
        
        Ok(())
    }
    
    /// Check if user has operator privileges
    pub fn has_operator_privileges(&self, user: &User) -> bool {
        user.is_operator()
    }
    
    /// Check if user has specific operator flag
    pub fn has_operator_flag(&self, user: &User, flag: OperatorFlag) -> bool {
        user.has_operator_flag(flag)
    }
    
    /// Check if user can perform remote connect
    pub fn can_remote_connect(&self, user: &User) -> bool {
        self.config.require_oper_for_connect && user.can_remote_connect()
    }
    
    /// Check if user can perform local connect
    pub fn can_local_connect(&self, user: &User) -> bool {
        self.config.require_oper_for_connect && user.can_local_connect()
    }
    
    /// Check if user can use SQUIT command
    pub fn can_squit(&self, user: &User) -> bool {
        user.can_squit()
    }
    
    /// Check if user is administrator
    pub fn is_administrator(&self, user: &User) -> bool {
        user.is_administrator()
    }
    
    /// Check if user has spy privileges
    pub fn is_spy(&self, user: &User) -> bool {
        user.is_spy()
    }
    
    /// Get operator information for STATS command
    pub fn get_operator_stats(&self, user: &User, requesting_user: Option<&User>) -> Option<String> {
        if !user.is_operator() {
            return None;
        }
        
        let is_operator = requesting_user.map(|u| u.is_operator()).unwrap_or(false);
        
        if is_operator && self.config.show_server_details_in_stats {
            // Show full information to operators
            Some(format!("{}@{} {} 0 Operator", user.username, user.host, user.nick))
        } else {
            // Show limited information to non-operators
            Some(format!("***@*** {} 0 Operator", user.nick))
        }
    }
    
    /// Get connection stats for operators
    pub fn get_connection_stats(&self, stats: &rustircd_core::ServerStatistics, requesting_user: Option<&User>) -> String {
        let is_operator = requesting_user.map(|u| u.is_operator()).unwrap_or(false);
        
        if is_operator && self.config.show_server_details_in_stats {
            // Show detailed connection information to operators
            format!("CONNECTIONS {} {} {}", 
                stats.total_connections, 
                stats.current_clients, 
                stats.current_servers)
        } else {
            // Show limited information to non-operators
            format!("CONNECTIONS {} {} {}", 
                stats.current_clients, 
                stats.current_clients, 
                stats.current_servers)
        }
    }
    
    /// Handle DEOP command (remove operator privileges)
    pub async fn handle_deop(&self, client: &Client, message: &Message, config: &Config) -> Result<()> {
        if !self.config.enabled {
            client.send_numeric(NumericReply::ErrUnknownCommand, &["DEOP"])?;
            return Ok(());
        }
        
        if message.params.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["DEOP", "Not enough parameters"])?;
            return Ok(());
        }
        
        let target_nick = &message.params[0];
        
        // Check if the requesting user is an operator
        if let Some(user) = &client.user {
            if !user.is_operator() {
                let error_msg = NumericReply::no_privileges();
                let _ = client.send(error_msg);
                return Ok(());
            }
        }
        
        // Implement finding the target user and revoking operator privileges
        // TODO: Integrate with user database/manager for full functionality
        
        // For now, implement basic logic that can be extended
        // In production, this would:
        // 1. Query the user database to find the target user by nickname
        // 2. Check if the target user exists and is currently an operator
        // 3. Revoke operator privileges using the revoke_operator_privileges method
        // 4. Update the user in the database
        // 5. Broadcast the privilege change to other servers
        // 6. Send confirmation to the requesting operator
        
        tracing::info!("DEOP command: {} attempting to revoke operator privileges from {}", 
            client.user.as_ref().map(|u| u.nick.as_str()).unwrap_or("unknown"), 
            target_nick);
        
        // In production, this would use the user manager/database:
        // if let Some(mut target_user) = user_manager.find_user_by_nick(target_nick) {
        //     if target_user.is_operator() {
        //         self.revoke_operator_privileges(&mut target_user);
        //         user_manager.update_user(target_user);
        //         
        //         // Send success message to requesting operator
        //         let success_msg = format!("Successfully revoked operator privileges from {}", target_nick);
        //         client.send_numeric(NumericReply::RplYoureOper, &[&success_msg])?;
        //     } else {
        //         let error_msg = format!("User {} is not an operator", target_nick);
        //         client.send_numeric(NumericReply::ErrNoSuchNick, &[target_nick, &error_msg])?;
        //     }
        // } else {
        //     let error_msg = format!("No such nick: {}", target_nick);
        //     client.send_numeric(NumericReply::ErrNoSuchNick, &[target_nick, &error_msg])?;
        // }
        
        // For now, just log the attempt
        tracing::debug!("DEOP: Would revoke operator privileges from user: {}", target_nick);
        
        if self.config.log_operator_actions {
            tracing::info!("Operator {} attempted to deop user {}", 
                client.user.as_ref().map(|u| u.nick.as_str()).unwrap_or("unknown"), 
                target_nick);
        }
        
        Ok(())
    }
    
    /// Revoke operator privileges for a user
    pub fn revoke_operator_privileges(&self, user: &mut User) {
        user.revoke_operator_privileges();
    }
    
    /// Log operator action
    pub fn log_operator_action(&self, user: &User, action: &str, details: Option<&str>) {
        if self.config.log_operator_actions {
            if let Some(details) = details {
                tracing::info!("Operator {} performed {}: {}", user.nick, action, details);
            } else {
                tracing::info!("Operator {} performed {}", user.nick, action);
            }
        }
    }
}

/// Trait for modules that need operator privilege checking
#[async_trait]
pub trait OperatorAware {
    /// Check if the current user has operator privileges
    async fn check_operator_privileges(&self, user: &User) -> Result<()>;
    
    /// Check if the current user has specific operator flag
    async fn check_operator_flag(&self, user: &User, flag: OperatorFlag) -> Result<()>;
    
    /// Log operator action
    async fn log_operator_action(&self, user: &User, action: &str, details: Option<&str>);
}

/// Default implementation for operator-aware modules
pub struct DefaultOperatorAware {
    oper_module: OperModule,
}

impl DefaultOperatorAware {
    pub fn new(oper_module: OperModule) -> Self {
        Self { oper_module }
    }
}

#[async_trait]
impl OperatorAware for DefaultOperatorAware {
    async fn check_operator_privileges(&self, user: &User) -> Result<()> {
        if !self.oper_module.has_operator_privileges(user) {
            return Err(Error::User("Insufficient privileges".to_string()));
        }
        Ok(())
    }
    
    async fn check_operator_flag(&self, user: &User, flag: OperatorFlag) -> Result<()> {
        if !self.oper_module.has_operator_flag(user, flag) {
            return Err(Error::User("Insufficient privileges for this operation".to_string()));
        }
        Ok(())
    }
    
    async fn log_operator_action(&self, user: &User, action: &str, details: Option<&str>) {
        self.oper_module.log_operator_action(user, action, details);
    }
}

/// Operator privilege checker utility
pub struct OperatorChecker {
    oper_module: OperModule,
}

impl OperatorChecker {
    pub fn new(oper_module: OperModule) -> Self {
        Self { oper_module }
    }
    
    /// Check if user can perform an action that requires operator privileges
    pub fn can_perform_action(&self, user: &User, action: OperatorAction) -> bool {
        match action {
            OperatorAction::RemoteConnect => self.oper_module.can_remote_connect(user),
            OperatorAction::LocalConnect => self.oper_module.can_local_connect(user),
            OperatorAction::Squit => self.oper_module.can_squit(user),
            OperatorAction::Administrator => self.oper_module.is_administrator(user),
            OperatorAction::Spy => self.oper_module.is_spy(user),
            OperatorAction::AnyOperator => self.oper_module.has_operator_privileges(user),
        }
    }
    
    /// Get required operator flag for an action
    pub fn get_required_flag(&self, action: OperatorAction) -> Option<OperatorFlag> {
        match action {
            OperatorAction::RemoteConnect => Some(OperatorFlag::RemoteConnect),
            OperatorAction::LocalConnect => Some(OperatorFlag::LocalConnect),
            OperatorAction::Squit => Some(OperatorFlag::Squit),
            OperatorAction::Administrator => Some(OperatorFlag::Administrator),
            OperatorAction::Spy => Some(OperatorFlag::Spy),
            OperatorAction::AnyOperator => None,
        }
    }
}

/// Types of operator actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorAction {
    /// Remote connect action
    RemoteConnect,
    /// Local connect action
    LocalConnect,
    /// SQUIT command
    Squit,
    /// Administrator action
    Administrator,
    /// Spy action
    Spy,
    /// Any operator action
    AnyOperator,
}

impl Default for OperModule {
    fn default() -> Self {
        Self::new(OperConfig::default())
    }
}

#[async_trait]
impl rustircd_core::Module for OperModule {
    fn name(&self) -> &str {
        "oper"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "Provides operator authentication and management functionality"
    }
    
    async fn init(&mut self) -> Result<()> {
        tracing::info!("{} module initialized", self.name());
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("{} module cleaned up", self.name());
        Ok(())
    }
    
    async fn handle_message(&mut self, client: &rustircd_core::Client, message: &rustircd_core::Message, context: &ModuleContext) -> Result<ModuleResult> {
        // Note: This is never called because handle_message_with_server is used
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_message_with_server(&mut self, client: &rustircd_core::Client, message: &rustircd_core::Message, server: Option<&rustircd_core::Server>, context: &ModuleContext) -> Result<ModuleResult> {
        match message.command {
            rustircd_core::MessageType::Oper => {
                // Get config from server if available
                let config = if let Some(srv) = server {
                    srv.config().clone()
                } else {
                    rustircd_core::Config::default()
                };
                
                self.handle_oper(client, message, &config, context).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Custom(ref cmd) if cmd == "DEOP" => {
                // Get config from server if available
                let config = if let Some(srv) = server {
                    srv.config().clone()
                } else {
                    rustircd_core::Config::default()
                };
                
                self.handle_deop(client, message, &config).await?;
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
