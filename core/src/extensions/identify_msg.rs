//! Identify Message Extension
//! 
//! This extension adds account information to messages, similar to Solanum's identify-msg extension.
//! It adds message tags to indicate when a user is identified to services.

use crate::{User, Message, Client, Result, Error};
use std::collections::HashMap;
use async_trait::async_trait;

/// Identify message extension - adds account information to messages
/// This is similar to Solanum's identify-msg extension
pub struct IdentifyMessageExtension {
    /// Service name for account identification
    service_name: String,
    /// Whether to add account tags to all messages
    add_to_all_messages: bool,
}

impl IdentifyMessageExtension {
    /// Create a new identify message extension
    pub fn new(service_name: String, add_to_all_messages: bool) -> Self {
        Self {
            service_name,
            add_to_all_messages,
        }
    }
}

#[async_trait]
impl crate::extensions::MessageTagExtension for IdentifyMessageExtension {
    /// Process incoming message tags
    async fn process_incoming_tags(&self, client: &Client, tags: &HashMap<String, String>) -> Result<HashMap<String, String>> {
        // Pass through incoming tags without modification
        Ok(tags.clone())
    }
    
    /// Generate outgoing message tags
    async fn generate_outgoing_tags(&self, sender: &User, message: &Message) -> Result<HashMap<String, String>> {
        let mut tags = HashMap::new();
        
        // Add account tag if user is identified to services
        if let Some(account) = sender.get_account() {
            if !account.is_empty() {
                tags.insert("account".to_string(), account.clone());
            }
        }
        
        // Add identify-msg tag if user is identified
        if let Some(identified) = sender.get_identified() {
            if identified {
                tags.insert("identify-msg".to_string(), "1".to_string());
            }
        }
        
        Ok(tags)
    }
    
    /// Validate message tags
    async fn validate_tags(&self, tags: &HashMap<String, String>) -> Result<()> {
        // Validate account tag format
        if let Some(account) = tags.get("account") {
            if account.is_empty() || account.len() > 32 {
                return Err(Error::InvalidInput("Invalid account tag format".to_string()));
            }
        }
        
        // Validate identify-msg tag
        if let Some(identify_msg) = tags.get("identify-msg") {
            if identify_msg != "1" && identify_msg != "0" {
                return Err(Error::InvalidInput("Invalid identify-msg tag format".to_string()));
            }
        }
        
        Ok(())
    }
}

impl Default for IdentifyMessageExtension {
    fn default() -> Self {
        Self::new("services.example.org".to_string(), true)
    }
}
