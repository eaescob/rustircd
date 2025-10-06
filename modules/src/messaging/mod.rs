//! Messaging module system for IRC daemon
//!
//! This module provides a framework for implementing IRC messaging commands
//! with configurable sender and receiver mode requirements.

use async_trait::async_trait;
use rustircd_core::{Client, Message, Result, UserMode};

/// Trait for messaging modules that handle IRC messaging commands
#[async_trait]
pub trait MessagingModule: Send + Sync {
    /// The IRC command this module handles (e.g., "WALLOPS", "NOTICE")
    fn command(&self) -> &str;
    
    /// User mode required for clients to send this command (None if no restriction)
    fn sender_mode_required(&self) -> Option<UserMode>;
    
    /// User mode required for clients to receive messages from this command (None if no restriction)
    fn receiver_mode_required(&self) -> Option<UserMode>;
    
    /// Handle the messaging command
    async fn handle_command(
        &mut self,
        sender: &Client,
        message: &Message,
        all_clients: &[&Client],
    ) -> Result<MessagingResult>;
    
    /// Get help text for this command
    fn help_text(&self) -> &str;
    
    /// Check if this command requires operator privileges to send
    fn requires_operator(&self) -> bool {
        self.sender_mode_required() == Some(UserMode::Operator) ||
        self.sender_mode_required() == Some(UserMode::LocalOperator)
    }
}

/// Result of messaging command handling
#[derive(Debug, Clone)]
pub enum MessagingResult {
    /// Command was handled successfully
    Handled,
    /// Command was rejected with error message
    Rejected(String),
    /// Command was not handled by this module
    NotHandled,
}

/// Manager for messaging modules
pub struct MessagingManager {
    modules: Vec<Box<dyn MessagingModule>>,
}

impl MessagingManager {
    /// Create a new messaging manager
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }
    
    /// Register a messaging module
    pub fn register_module(&mut self, module: Box<dyn MessagingModule>) {
        self.modules.push(module);
    }
    
    /// Handle a message command through registered modules
    pub async fn handle_message(
        &mut self,
        sender: &Client,
        message: &Message,
        all_clients: &[&Client],
    ) -> Result<MessagingResult> {
        let command = message.command.to_string();
        
        for module in &mut self.modules {
            if module.command() == command {
                // Check sender mode requirements
                if let Some(required_mode) = module.sender_mode_required() {
                    if let Some(user) = &sender.user {
                        if !user.has_mode(required_mode.to_char()) {
                            return Ok(MessagingResult::Rejected(
                                format!("Permission denied: {} requires {} mode", 
                                    command, 
                                    required_mode.to_char()
                                )
                            ));
                        }
                    } else {
                        return Ok(MessagingResult::Rejected(
                            "Permission denied: User not registered".to_string()
                        ));
                    }
                }
                
                // Check operator requirements
                if module.requires_operator() {
                    if let Some(user) = &sender.user {
                        if !user.is_operator {
                            return Ok(MessagingResult::Rejected(
                                "Permission denied: Operator privileges required".to_string()
                            ));
                        }
                    } else {
                        return Ok(MessagingResult::Rejected(
                            "Permission denied: User not registered".to_string()
                        ));
                    }
                }
                
                // Handle the command
                return module.handle_command(sender, message, all_clients).await;
            }
        }
        
        Ok(MessagingResult::NotHandled)
    }
    
    /// Get all registered commands
    pub fn get_commands(&self) -> Vec<&str> {
        self.modules.iter().map(|m| m.command()).collect()
    }
    
    /// Get help text for a specific command
    pub fn get_help(&self, command: &str) -> Option<&str> {
        self.modules
            .iter()
            .find(|m| m.command() == command.to_uppercase())
            .map(|m| m.help_text())
    }
    
    /// Get all help text
    pub fn get_all_help(&self) -> Vec<(String, String)> {
        self.modules
            .iter()
            .map(|m| (m.command().to_string(), m.help_text().to_string()))
            .collect()
    }
}

impl Default for MessagingManager {
    fn default() -> Self {
        Self::new()
    }
}

// Export the wallops module and wrapper
pub mod wallops;
pub mod wrapper;
pub use wallops::WallopsModule;
pub use wrapper::{MessagingWrapper, create_default_messaging_module};