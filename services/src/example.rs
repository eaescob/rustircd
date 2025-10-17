//! Example service implementation

use crate::{Service, ServiceResult};
use crate::framework::ServiceContext;
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
    
    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ServiceContext) -> Result<ServiceResult> {
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
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message, _context: &ServiceContext) -> Result<ServiceResult> {
        Ok(ServiceResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, user: &User, _context: &ServiceContext) -> Result<()> {
        tracing::info!("User {} registered (example service)", user.nick);
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, user: &User, _context: &ServiceContext) -> Result<()> {
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
        
        // Implement services list response
        // TODO: Integrate with actual service registry for dynamic service list
        
        let services_list = "Available services: example, help, nickserv, chanserv";
        let _response = Message::new(
            rustircd_core::MessageType::Custom("SERVICES".to_string()),
            vec![services_list.to_string()],
        );
        
        // Send response to client
        // In production, this would use proper IRC numeric replies
        tracing::debug!("Would send services list to client {}: {}", client.id, services_list);
        
        // In production, would use:
        // client.send(response)?;
        // Or proper numeric: client.send_numeric(NumericReply::RplServices, &[services_list])?;
        
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
            vec![help_text.clone()],
        );
        
        // Implement help response to client
        // TODO: Integrate with proper IRC numeric replies
        
        tracing::debug!("Would send help response to client {}: {}", client.id, help_text);
        
        // In production, would use:
        // client.send(response)?;
        // Or proper numeric: client.send_numeric(NumericReply::RplHelpText, &[&help_text])?;
        
        Ok(())
    }
}
