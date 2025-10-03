//! Wallops messaging module
//!
//! Implements the WALLOPS command which allows operators to send messages
//! to all users with the wallops mode (+w) set.

use async_trait::async_trait;
use rustircd_core::{Client, Message, Result, UserMode};
use super::{MessagingModule, MessagingResult};

/// Wallops messaging module implementation
pub struct WallopsModule;

impl WallopsModule {
    /// Create a new wallops module
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MessagingModule for WallopsModule {
    fn command(&self) -> &str {
        "WALLOPS"
    }
    
    fn sender_mode_required(&self) -> Option<UserMode> {
        Some(UserMode::Operator)
    }
    
    fn receiver_mode_required(&self) -> Option<UserMode> {
        Some(UserMode::Wallops)
    }
    
    async fn handle_command(
        &mut self,
        sender: &Client,
        message: &Message,
        all_clients: &[&Client],
    ) -> Result<MessagingResult> {
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
        
        // Send to all clients with wallops mode
        let mut sent_count = 0;
        for client in all_clients {
            if let Some(user) = &client.user {
                if user.has_mode(UserMode::Wallops.to_char()) {
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
