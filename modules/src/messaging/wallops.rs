//! Wallops messaging module
//!
//! Implements the WALLOPS command which allows operators to send messages
//! to all users with the wallops mode (+w) set.

use async_trait::async_trait;
use rustircd_core::{Client, Message, Result, UserMode, CustomUserMode, register_custom_mode, unregister_custom_mode};
use super::{MessagingModule, MessagingResult};

/// Wallops messaging module implementation
pub struct WallopsModule;

impl WallopsModule {
    /// Create a new wallops module
    pub fn new() -> Self {
        Self
    }
    
    /// Initialize the wallops module and register the +w mode
    pub fn initialize() -> Result<()> {
        let wallops_mode = CustomUserMode {
            character: 'w',
            description: "Receive wallop messages".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "wallops".to_string(),
        };
        
        register_custom_mode(wallops_mode).map_err(|e| e.into())
    }
    
    /// Cleanup the wallops module and unregister the +w mode
    pub fn cleanup() -> Result<()> {
        unregister_custom_mode('w', "wallops").map_err(|e| e.into())
    }
    
    /// Check if user has wallops mode (+w)
    fn has_wallops_mode(user: &rustircd_core::User) -> bool {
        user.has_mode('w')
    }
    
    /// Check if user is an operator (has +o mode and operator privileges)
    fn is_operator(user: &rustircd_core::User) -> bool {
        user.is_operator && user.has_mode('o')
    }
}

#[async_trait]
impl MessagingModule for WallopsModule {
    fn command(&self) -> &str {
        "WALLOPS"
    }
    
    fn sender_mode_required(&self) -> Option<UserMode> {
        // WALLOPS requires operator privileges, but we'll handle this manually
        // since we need to check both operator status and +o mode
        None
    }
    
    fn receiver_mode_required(&self) -> Option<UserMode> {
        // WALLOPS requires +w mode, but we'll handle this manually
        // since +w is now a custom mode
        None
    }
    
    async fn handle_command(
        &mut self,
        sender: &Client,
        message: &Message,
        all_clients: &[&Client],
    ) -> Result<MessagingResult> {
        // Check if sender is registered
        let user = match &sender.user {
            Some(user) => user,
            None => {
                return Ok(MessagingResult::Rejected(
                    "You must be registered to use WALLOPS".to_string()
                ));
            }
        };
        
        // Check if sender is an operator
        if !Self::is_operator(user) {
            return Ok(MessagingResult::Rejected(
                "Permission denied: Operator privileges required".to_string()
            ));
        }
        
        // Check if message has parameters
        if message.params.is_empty() {
            return Ok(MessagingResult::Rejected(
                "WALLOPS :No message provided".to_string()
            ));
        }
        
        // Get the wallops message (all parameters joined)
        let wallops_message = message.params.join(" ");
        
        // Create the wallops message format
        let wallops_msg = format!(":{} WALLOPS :{}", sender.nickname().unwrap_or("unknown"), wallops_message);
        
        // Send to all clients with wallops mode (+w)
        let mut sent_count = 0;
        for client in all_clients {
            if let Some(user) = &client.user {
                if Self::has_wallops_mode(user) {
                    if let Err(e) = client.send_raw(&wallops_msg) {
                        tracing::warn!("Failed to send wallops to {}: {}", client.nickname().unwrap_or("unknown"), e);
                    } else {
                        sent_count += 1;
                    }
                }
            }
        }
        
        tracing::info!(
            "Wallops sent by {} to {} recipients: {}",
            sender.nickname().unwrap_or("unknown"),
            sent_count,
            wallops_message
        );
        
        Ok(MessagingResult::Handled)
    }
    
    fn help_text(&self) -> &str {
        "WALLOPS <message> - Send a message to all users with wallops mode (+w). Requires operator privileges."
    }
}

impl Default for WallopsModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustircd_core::{Client, Message, UserModeManager};
    use std::collections::HashSet;
    
    #[tokio::test]
    async fn test_wallops_command_handling() {
        let mut wallops = WallopsModule::new();
        
        // Test command name
        assert_eq!(wallops.command(), "WALLOPS");
        
        // Test mode requirements
        assert_eq!(wallops.sender_mode_required(), Some(UserMode::Operator));
        assert_eq!(wallops.receiver_mode_required(), Some(UserMode::Wallops));
        
        // Test operator requirement
        assert!(wallops.requires_operator());
    }
    
    #[tokio::test]
    async fn test_wallops_empty_message() {
        let mut wallops = WallopsModule::new();
        
        // Create a mock client
        let mut modes = UserModeManager::new();
        modes.set_operator(true);
        
        let sender = Client {
            nickname: "operator".to_string(),
            username: "op".to_string(),
            hostname: "localhost".to_string(),
            user_modes: modes,
            // ... other fields would be mocked
        };
        
        let message = Message {
            command: "WALLOPS".to_string(),
            params: vec![], // Empty message
        };
        
        let result = wallops.handle_command(&sender, &message, &[]).await.unwrap();
        
        match result {
            MessagingResult::Rejected(msg) => {
                assert!(msg.contains("No message provided"));
            }
            _ => panic!("Expected rejected result for empty message"),
        }
    }
}
