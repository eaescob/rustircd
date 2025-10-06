//! Server Time Extension
//! 
//! This extension provides server time information via message tags,
//! similar to Solanum's server-time extension.

use crate::{User, Message, Client, Result, Error};
use std::collections::HashMap;
use async_trait::async_trait;

/// Server time extension - provides server time information
/// This is similar to Solanum's server-time extension
pub struct ServerTimeExtension {
    /// Whether to add server time to all messages
    add_to_all_messages: bool,
}

impl ServerTimeExtension {
    /// Create a new server time extension
    pub fn new(add_to_all_messages: bool) -> Self {
        Self {
            add_to_all_messages,
        }
    }
}

#[async_trait]
impl crate::extensions::MessageTagExtension for ServerTimeExtension {
    /// Process incoming message tags
    async fn process_incoming_tags(&self, _client: &Client, tags: &HashMap<String, String>) -> Result<HashMap<String, String>> {
        // Pass through incoming tags without modification
        Ok(tags.clone())
    }
    
    /// Generate outgoing message tags
    async fn generate_outgoing_tags(&self, _sender: &User, _message: &Message) -> Result<HashMap<String, String>> {
        let mut tags = HashMap::new();
        
        if self.add_to_all_messages {
            // Add server time tag
            let server_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
            tags.insert("time".to_string(), server_time);
        }
        
        Ok(tags)
    }
    
    /// Validate message tags
    async fn validate_tags(&self, tags: &HashMap<String, String>) -> Result<()> {
        // Validate time tag format
        if let Some(time) = tags.get("time") {
            if chrono::DateTime::parse_from_rfc3339(time).is_err() {
                return Err(Error::InvalidInput("Invalid time tag format".to_string()));
            }
        }
        
        Ok(())
    }
}

impl Default for ServerTimeExtension {
    fn default() -> Self {
        Self::new(true)
    }
}
