//! Globops messaging module
//!
//! Implements the GLOBOPS command which allows operators to send messages
//! to all users with the globops mode (+g) set.

use async_trait::async_trait;
use rustircd_core::{Client, Message, Result, UserMode, CustomUserMode, register_custom_mode, unregister_custom_mode};
use super::{MessagingModule, MessagingResult};

/// Globops messaging module implementation
pub struct GlobopsModule;

impl GlobopsModule {
    /// Create a new globops module
    pub fn new() -> Self {
        Self
    }
    
    /// Initialize the globops module and register the +g mode
    pub fn initialize() -> Result<()> {
        let globops_mode = CustomUserMode {
            character: 'g',
            description: "Receive global operator notices (globops)".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "globops".to_string(),
        };
        
        register_custom_mode(globops_mode).map_err(|e| e.into())
    }
    
    /// Cleanup the globops module and unregister the +g mode
    pub fn cleanup() -> Result<()> {
        unregister_custom_mode('g', "globops").map_err(|e| e.into())
    }
    
    /// Check if user has globops mode (+g)
    fn has_globops_mode(user: &rustircd_core::User) -> bool {
        user.has_mode('g')
    }
    
    /// Check if user is an operator (has +o mode and operator privileges)
    fn is_operator(user: &rustircd_core::User) -> bool {
        user.is_operator && user.has_mode('o')
    }
}

#[async_trait]
impl MessagingModule for GlobopsModule {
    fn command(&self) -> &str {
        "GLOBOPS"
    }
    
    fn sender_mode_required(&self) -> Option<UserMode> {
        // GLOBOPS requires operator privileges, not a specific mode
        // We'll handle this in the handle_command method
        None
    }
    
    fn receiver_mode_required(&self) -> Option<UserMode> {
        // GLOBOPS requires +g mode, but we'll handle this manually
        // since +g is not in the core UserMode enum
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
                    "You must be registered to use GLOBOPS".to_string()
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
                "GLOBOPS :No message provided".to_string()
            ));
        }
        
        // Get the globops message (all parameters joined)
        let globops_message = message.params.join(" ");
        
        // Create the globops message format
        let globops_msg = format!(":{} GLOBOPS :{}", sender.nickname().unwrap_or("unknown"), globops_message);
        
        // Send to all clients with globops mode (+g)
        let mut sent_count = 0;
        for client in all_clients {
            if let Some(user) = &client.user {
                if Self::has_globops_mode(user) {
                    if let Err(e) = client.send_raw(&globops_msg) {
                        tracing::warn!("Failed to send globops to {}: {}", client.nickname().unwrap_or("unknown"), e);
                    } else {
                        sent_count += 1;
                    }
                }
            }
        }
        
        tracing::info!(
            "Globops sent by {} to {} recipients: {}",
            sender.nickname().unwrap_or("unknown"),
            sent_count,
            globops_message
        );
        
        Ok(MessagingResult::Handled)
    }
    
    fn help_text(&self) -> &str {
        "GLOBOPS <message> - Send a message to all users with globops mode (+g). Requires operator privileges."
    }
}

impl Default for GlobopsModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustircd_core::{Client, User, Prefix};
    use std::collections::HashSet;
    use uuid::Uuid;
    use chrono::Utc;

    fn create_test_user(nick: &str, is_operator: bool, modes: HashSet<char>) -> User {
        let mut user = User::new(
            nick.to_string(),
            "username".to_string(),
            "Real Name".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        user.is_operator = is_operator;
        user.modes = modes;
        user
    }

    fn create_test_client(user: User) -> Client {
        // This is a simplified test client - in reality, Client creation is more complex
        // For testing purposes, we'll create a mock client
        todo!("Need to implement test client creation")
    }

    #[test]
    fn test_globops_module_creation() {
        let module = GlobopsModule::new();
        assert_eq!(module.command(), "GLOBOPS");
        assert_eq!(module.sender_mode_required(), None);
        assert_eq!(module.receiver_mode_required(), None);
        assert!(module.help_text().contains("GLOBOPS"));
    }

    #[test]
    fn test_has_globops_mode() {
        let mut modes = HashSet::new();
        modes.insert('g');
        let user = create_test_user("testuser", false, modes);
        
        assert!(GlobopsModule::has_globops_mode(&user));
        
        let user_no_mode = create_test_user("testuser2", false, HashSet::new());
        assert!(!GlobopsModule::has_globops_mode(&user_no_mode));
    }

    #[test]
    fn test_is_operator() {
        let user_oper = create_test_user("operuser", true, HashSet::new());
        assert!(GlobopsModule::is_operator(&user_oper));
        
        let user_non_oper = create_test_user("normaluser", false, HashSet::new());
        assert!(!GlobopsModule::is_operator(&user_non_oper));
    }
}
