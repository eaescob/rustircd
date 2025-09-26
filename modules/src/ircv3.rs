//! IRCv3 support module

use rustircd_core::{Module, ModuleResult, Client, Message, User, Error, Result};
use async_trait::async_trait;
use std::collections::HashSet;

/// IRCv3 support module
pub struct Ircv3Module {
    name: String,
    version: String,
    description: String,
    capabilities: HashSet<String>,
}

impl Ircv3Module {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert("cap".to_string());
        capabilities.insert("sasl".to_string());
        capabilities.insert("account-tag".to_string());
        capabilities.insert("away-notify".to_string());
        capabilities.insert("batch".to_string());
        capabilities.insert("bot-mode".to_string());
        capabilities.insert("channel-rename".to_string());
        capabilities.insert("chghost".to_string());
        capabilities.insert("echo-message".to_string());
        capabilities.insert("extended-join".to_string());
        capabilities.insert("invite-notify".to_string());
        capabilities.insert("message-tags".to_string());
        capabilities.insert("multi-prefix".to_string());
        capabilities.insert("server-time".to_string());
        capabilities.insert("userhost-in-names".to_string());
        
        Self {
            name: "ircv3".to_string(),
            version: "1.0.0".to_string(),
            description: "IRCv3 capability negotiation and extensions".to_string(),
            capabilities,
        }
    }
}

#[async_trait]
impl Module for Ircv3Module {
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
        tracing::info!("Initializing IRCv3 module");
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up IRCv3 module");
        Ok(())
    }
    
    async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ModuleResult> {
        match message.command {
            rustircd_core::MessageType::Cap => {
                self.handle_cap(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Authenticate => {
                self.handle_authenticate(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, _user: &User) -> Result<()> {
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, _user: &User) -> Result<()> {
        Ok(())
    }
    
    
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "message_handler".to_string(),
            "capability_negotiation".to_string(),
        ]
    }
    
    fn supports_capability(&self, capability: &str) -> bool {
        matches!(capability, "message_handler" | "capability_negotiation")
    }
    
    fn get_numeric_replies(&self) -> Vec<u16> {
        vec![] // IRCv3 doesn't define specific numeric replies
    }
    
    fn handles_numeric_reply(&self, _numeric: u16) -> bool {
        false
    }
    
    async fn handle_numeric_reply(&mut self, _numeric: u16, _params: Vec<String>) -> Result<()> {
        Ok(())
    }
}

impl Ircv3Module {
    async fn handle_cap(&self, client: &Client, message: &Message) -> Result<()> {
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
            vec!["*".to_string(), "LS".to_string(), cap_list],
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
    
    async fn handle_authenticate(&self, client: &Client, message: &Message) -> Result<()> {
        if message.params.is_empty() {
            return Err(Error::User("No SASL mechanism specified".to_string()));
        }
        
        let mechanism = &message.params[0];
        
        match mechanism.as_str() {
            "PLAIN" => {
                self.handle_sasl_plain(client, message).await?;
            }
            "EXTERNAL" => {
                self.handle_sasl_external(client, message).await?;
            }
            _ => {
                return Err(Error::User("Unsupported SASL mechanism".to_string()));
            }
        }
        
        Ok(())
    }
    
    async fn handle_sasl_plain(&self, client: &Client, message: &Message) -> Result<()> {
        // TODO: Implement SASL PLAIN authentication
        tracing::info!("SASL PLAIN authentication for client {}", client.id);
        Ok(())
    }
    
    async fn handle_sasl_external(&self, client: &Client, message: &Message) -> Result<()> {
        // TODO: Implement SASL EXTERNAL authentication
        tracing::info!("SASL EXTERNAL authentication for client {}", client.id);
        Ok(())
    }
    
    fn get_available_capabilities(&self) -> Vec<String> {
        self.capabilities.iter().cloned().collect()
    }
}
