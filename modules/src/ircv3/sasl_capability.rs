//! SASL IRCv3 Capability Extension
//! 
//! This module provides SASL authentication integration with IRCv3 capability negotiation.

use rustircd_core::{Client, Message, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// SASL capability extension for IRCv3
pub struct SaslCapability {
    /// Whether SASL capability is enabled for a client
    enabled_clients: std::sync::Arc<Mutex<std::collections::HashSet<Uuid>>>,
    /// Supported SASL mechanisms
    supported_mechanisms: Vec<String>,
}

impl SaslCapability {
    /// Create a new SASL capability extension
    pub fn new() -> Self {
        Self {
            enabled_clients: Arc::new(Mutex::new(std::collections::HashSet::new())),
            supported_mechanisms: vec!["PLAIN".to_string(), "EXTERNAL".to_string()],
        }
    }
    
    /// Enable SASL capability for a client
    pub async fn enable_for_client(&self, client_id: Uuid) {
        let mut clients = self.enabled_clients.lock().await;
        clients.insert(client_id);
        tracing::info!("SASL capability enabled for client {}", client_id);
    }
    
    /// Disable SASL capability for a client
    pub async fn disable_for_client(&self, client_id: Uuid) {
        let mut clients = self.enabled_clients.lock().await;
        clients.remove(&client_id);
        tracing::info!("SASL capability disabled for client {}", client_id);
    }
    
    /// Check if SASL capability is enabled for a client
    pub async fn is_enabled_for_client(&self, client_id: &Uuid) -> bool {
        let clients = self.enabled_clients.lock().await;
        clients.contains(client_id)
    }
    
    /// Get supported SASL mechanisms
    pub fn get_supported_mechanisms(&self) -> &[String] {
        &self.supported_mechanisms
    }
    
    /// Handle SASL capability negotiation
    pub async fn handle_capability_request(&self, client: &Client, requested_caps: &[String]) -> Result<Vec<String>> {
        let mut acked_caps = Vec::new();
        
        for cap in requested_caps {
            if cap == "sasl" {
                // Check if client supports SASL
                if self.client_supports_sasl(client).await? {
                    acked_caps.push("sasl".to_string());
                    self.enable_for_client(client.id).await;
                }
            }
        }
        
        Ok(acked_caps)
    }
    
    /// Check if client supports SASL (basic check)
    async fn client_supports_sasl(&self, _client: &Client) -> Result<bool> {
        // For now, assume all clients support SASL
        // In production, this could check client capabilities or version
        Ok(true)
    }
    
    /// Handle AUTHENTICATE command when SASL capability is enabled
    pub async fn handle_authenticate(&self, client: &Client, message: &Message) -> Result<()> {
        // Check if SASL capability is enabled for this client
        if !self.is_enabled_for_client(&client.id).await {
            let error_msg = Message::new(
                rustircd_core::MessageType::Custom("ERROR".to_string()),
                vec!["SASL capability not enabled".to_string()]
            );
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        if message.params.is_empty() {
            let error_msg = Message::new(
                rustircd_core::MessageType::Custom("ERROR".to_string()),
                vec!["AUTHENTICATE requires a mechanism".to_string()]
            );
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        let mechanism = &message.params[0];
        
        // Check if mechanism is supported
        if !self.supported_mechanisms.contains(mechanism) {
            let error_msg = Message::new(
                rustircd_core::MessageType::Custom("ERROR".to_string()),
                vec![format!("SASL mechanism '{}' not supported", mechanism)]
            );
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        // For now, just acknowledge the mechanism
        // In production, this would integrate with the actual SASL module
        let ack_msg = Message::new(
            rustircd_core::MessageType::Custom("AUTHENTICATE".to_string()),
            vec!["+".to_string()]
        );
        let _ = client.send(ack_msg);
        
        tracing::info!("SASL AUTHENTICATE received from client {} with mechanism {}", client.id, mechanism);
        
        Ok(())
    }
    
    /// Get SASL capability information for CAP LS response
    pub fn get_capability_info(&self) -> String {
        let mechanisms = self.supported_mechanisms.join(",");
        format!("sasl={}", mechanisms)
    }
}

impl Default for SaslCapability {
    fn default() -> Self {
        Self::new()
    }
}
