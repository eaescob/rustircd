//! IRCv3 Capability Negotiation (CAP)

use rustircd_core::{Client, Message, Error, Result};
use std::collections::HashSet;

/// Capability negotiation handler
pub struct CapabilityNegotiation {
    /// Available capabilities
    capabilities: HashSet<String>,
    /// Client capabilities being negotiated
    client_capabilities: std::collections::HashMap<uuid::Uuid, HashSet<String>>,
}

impl CapabilityNegotiation {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert("cap".to_string());
        capabilities.insert("message-tags".to_string());
        capabilities.insert("account-tag".to_string());
        capabilities.insert("away-notify".to_string());
        capabilities.insert("batch".to_string());
        capabilities.insert("bot-mode".to_string());
        capabilities.insert("channel-rename".to_string());
        capabilities.insert("chghost".to_string());
        capabilities.insert("echo-message".to_string());
        capabilities.insert("extended-join".to_string());
        capabilities.insert("invite-notify".to_string());
        capabilities.insert("multi-prefix".to_string());
        capabilities.insert("server-time".to_string());
        capabilities.insert("userhost-in-names".to_string());
        
        Self {
            capabilities,
            client_capabilities: std::collections::HashMap::new(),
        }
    }
    
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing capability negotiation");
        Ok(())
    }
    
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up capability negotiation");
        Ok(())
    }
    
    pub async fn handle_cap(&self, client: &Client, message: &Message) -> Result<()> {
        if message.params.is_empty() {
            return Err(Error::User("No CAP subcommand specified".to_string()));
        }
        
        let subcommand = &message.params[0];
        
        match subcommand.as_str() {
            "LS" => {
                self.handle_cap_ls(client, message).await?;
            }
            "REQ" => {
                self.handle_cap_req(client, message).await?;
            }
            "ACK" => {
                self.handle_cap_ack(client, message).await?;
            }
            "NAK" => {
                self.handle_cap_nak(client, message).await?;
            }
            "CLEAR" => {
                self.handle_cap_clear(client, message).await?;
            }
            "END" => {
                self.handle_cap_end(client, message).await?;
            }
            _ => {
                return Err(Error::User("Unknown CAP subcommand".to_string()));
            }
        }
        
        Ok(())
    }
    
    async fn handle_cap_ls(&self, client: &Client, _message: &Message) -> Result<()> {
        // Send available capabilities
        let capabilities = self.get_available_capabilities();
        let cap_list = capabilities.join(" ");
        
        let response = Message::new(
            rustircd_core::MessageType::Custom("CAP".to_string()),
            vec!["*".to_string(), "LS".to_string(), cap_list.clone()],
        );
        
        // TODO: Send response to client
        tracing::info!("Sending capabilities to client {}: {}", client.id, cap_list);
        
        Ok(())
    }
    
    async fn handle_cap_req(&self, client: &Client, message: &Message) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::User("No capabilities specified".to_string()));
        }
        
        let requested_caps: Vec<&str> = message.params[1].split_whitespace().collect();
        let mut acked_caps = Vec::new();
        let mut nacked_caps = Vec::new();
        
        for cap in requested_caps {
            if self.capabilities.contains(cap) {
                acked_caps.push(cap);
            } else {
                nacked_caps.push(cap);
            }
        }
        
        // Send ACK for supported capabilities
        if !acked_caps.is_empty() {
            let ack_msg = Message::new(
                rustircd_core::MessageType::Custom("CAP".to_string()),
                vec!["*".to_string(), "ACK".to_string(), acked_caps.join(" ")],
            );
            // TODO: Send response to client
            tracing::info!("ACK capabilities for client {}: {}", client.id, acked_caps.join(" "));
        }
        
        // Send NAK for unsupported capabilities
        if !nacked_caps.is_empty() {
            let nak_msg = Message::new(
                rustircd_core::MessageType::Custom("CAP".to_string()),
                vec!["*".to_string(), "NAK".to_string(), nacked_caps.join(" ")],
            );
            // TODO: Send response to client
            tracing::info!("NAK capabilities for client {}: {}", client.id, nacked_caps.join(" "));
        }
        
        Ok(())
    }
    
    async fn handle_cap_ack(&self, _client: &Client, _message: &Message) -> Result<()> {
        // Client acknowledged capabilities
        Ok(())
    }
    
    async fn handle_cap_nak(&self, _client: &Client, _message: &Message) -> Result<()> {
        // Client rejected capabilities
        Ok(())
    }
    
    async fn handle_cap_clear(&self, _client: &Client, _message: &Message) -> Result<()> {
        // Clear client capabilities
        Ok(())
    }
    
    async fn handle_cap_end(&self, _client: &Client, _message: &Message) -> Result<()> {
        // End capability negotiation
        Ok(())
    }
    
    fn get_available_capabilities(&self) -> Vec<String> {
        self.capabilities.iter().cloned().collect()
    }
}
