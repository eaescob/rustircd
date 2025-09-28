//! Example service implementation

use crate::{Service, ServiceResult};
use rustircd_core::{Client, Message, User, Result};
use async_trait::async_trait;

/// Example service that demonstrates the service framework
pub struct ExampleService {
    name: String,
    version: String,
    description: String,
}

impl ExampleService {
    pub fn new() -> Self {
        Self {
            name: "example".to_string(),
            version: "1.0.0".to_string(),
            description: "Example service implementation".to_string(),
        }
    }
}

#[async_trait]
impl Service for ExampleService {
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
        tracing::info!("Initializing example service");
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up example service");
        Ok(())
    }
    
    async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ServiceResult> {
        // Example: Handle custom service commands
        if let rustircd_core::MessageType::Custom(cmd) = &message.command {
            match cmd.as_str() {
                "SERVICES" => {
                    self.handle_services_command(client, message).await?;
                    Ok(ServiceResult::Handled)
                }
                "HELP" => {
                    self.handle_help_command(client, message).await?;
                    Ok(ServiceResult::Handled)
                }
                _ => Ok(ServiceResult::NotHandled),
            }
        } else {
            Ok(ServiceResult::NotHandled)
        }
    }
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message) -> Result<ServiceResult> {
        Ok(ServiceResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, user: &User) -> Result<()> {
        tracing::info!("User {} registered (example service)", user.nick);
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, user: &User) -> Result<()> {
        tracing::info!("User {} disconnected (example service)", user.nick);
        Ok(())
    }
    
    
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "message_handler".to_string(),
            "user_handler".to_string(),
        ]
    }
    
    fn supports_capability(&self, capability: &str) -> bool {
        matches!(capability, "message_handler" | "user_handler")
    }
}

impl ExampleService {
    async fn handle_services_command(&self, client: &Client, _message: &Message) -> Result<()> {
        tracing::info!("Client {} requested services list", client.id);
        
        // TODO: Send services list to client
        let _response = Message::new(
            rustircd_core::MessageType::Custom("SERVICES".to_string()),
            vec!["Available services: example, help".to_string()],
        );
        
        // TODO: Send response to client
        Ok(())
    }
    
    async fn handle_help_command(&self, client: &Client, message: &Message) -> Result<()> {
        tracing::info!("Client {} requested help", client.id);
        
        let help_text = if message.params.is_empty() {
            "Available commands: SERVICES, HELP".to_string()
        } else {
            format!("Help for: {}", message.params.join(" "))
        };
        
        let _response = Message::new(
            rustircd_core::MessageType::Custom("HELP".to_string()),
            vec![help_text],
        );
        
        // TODO: Send response to client
        Ok(())
    }
}
