//! Messaging module wrapper for integration with core module system
//!
//! This module provides a wrapper that implements the core Module trait
//! and delegates to the MessagingManager for handling messaging commands.

use async_trait::async_trait;
use rustircd_core::{Client, Message, Result, Server, User};
use rustircd_core::module::{Module, ModuleResult, ModuleStatsResponse};
use super::{MessagingManager, MessagingModule};

/// Wrapper module that integrates messaging modules with the core module system
pub struct MessagingWrapper {
    manager: MessagingManager,
    name: String,
    version: String,
    description: String,
}

impl MessagingWrapper {
    /// Create a new messaging wrapper with the given name and version
    pub fn new(name: String, version: String, description: String) -> Self {
        Self {
            manager: MessagingManager::new(),
            name,
            version,
            description,
        }
    }
    
    /// Register a messaging module
    pub fn register_messaging_module(&mut self, module: Box<dyn MessagingModule>) {
        self.manager.register_module(module);
    }
    
    /// Get the messaging manager
    pub fn manager(&mut self) -> &mut MessagingManager {
        &mut self.manager
    }
}

#[async_trait]
impl Module for MessagingWrapper {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn init(&mut self) -> Result<()> {
        tracing::info!("Messaging module '{}' initialized", self.name);
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Messaging module '{}' cleaned up", self.name);
        Ok(())
    }

    fn register_numerics(&self, _manager: &mut rustircd_core::ModuleNumericManager) -> Result<()> {
        Ok(())
    }
    
    async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ModuleResult> {
        // Get all connected clients for messaging modules that need to broadcast
        // Note: In a real implementation, this would need to be passed from the server
        let all_clients = vec![client]; // Simplified for now
        
        match self.manager.handle_message(client, message, &all_clients).await? {
            super::MessagingResult::Handled => Ok(ModuleResult::Handled),
            super::MessagingResult::Rejected(reason) => {
                // Send error message to client
                if let Err(e) = client.send_raw(&format!(":{} ERROR :{}", 
                    client.nickname().unwrap_or("unknown"), reason)) {
                    tracing::warn!("Failed to send error message to {}: {}", client.nickname().unwrap_or("unknown"), e);
                }
                Ok(ModuleResult::Rejected(reason))
            }
            super::MessagingResult::NotHandled => Ok(ModuleResult::NotHandled),
        }
    }
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message) -> Result<ModuleResult> {
        // Messaging modules typically don't handle server messages
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, _user: &User) -> Result<()> {
        // No special handling needed for user registration
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, _user: &User) -> Result<()> {
        // No special handling needed for user disconnection
        Ok(())
    }
    
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "message_handler".to_string(),
            "messaging".to_string(),
        ]
    }
    
    fn supports_capability(&self, capability: &str) -> bool {
        matches!(capability, "message_handler" | "messaging")
    }
    
    fn get_numeric_replies(&self) -> Vec<u16> {
        // Messaging modules don't typically handle numeric replies
        vec![]
    }
    
    fn handles_numeric_reply(&self, _numeric: u16) -> bool {
        false
    }
    
    async fn handle_numeric_reply(&mut self, _numeric: u16, _params: Vec<String>) -> Result<()> {
        // No numeric replies to handle
        Ok(())
    }
    
    async fn handle_stats_query(&mut self, query: &str, _client_id: uuid::Uuid, _server: Option<&Server>) -> Result<Vec<ModuleStatsResponse>> {
        let mut responses = Vec::new();
        
        if query == "m" {
            // Return messaging module statistics
            let commands = self.manager.get_commands();
            responses.push(ModuleStatsResponse::ModuleStats(
                "m".to_string(),
                format!("Messaging modules: {}", commands.join(", "))
            ));
        }
        
        Ok(responses)
    }
    
    fn get_stats_queries(&self) -> Vec<String> {
        vec!["m".to_string()]
    }
}

/// Create a default messaging wrapper with wallops support
pub fn create_default_messaging_module() -> MessagingWrapper {
    let mut wrapper = MessagingWrapper::new(
        "messaging".to_string(),
        "1.0.0".to_string(),
        "IRC messaging commands (wallops, etc.)".to_string(),
    );
    
    // Register the wallops module
    wrapper.register_messaging_module(Box::new(super::WallopsModule::new()));
    
    wrapper
}
