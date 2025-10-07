//! Services framework for IRC daemon

use rustircd_core::{Client, Message, User, Result, Database, ServerConnectionManager};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Service context providing access to core functionality
pub struct ServiceContext {
    /// Database access
    pub database: Arc<Database>,
    /// Server connection manager for broadcasting
    pub server_connections: Arc<ServerConnectionManager>,
}

impl ServiceContext {
    /// Create a new service context
    pub fn new(database: Arc<Database>, server_connections: Arc<ServerConnectionManager>) -> Self {
        Self {
            database,
            server_connections,
        }
    }
    
    /// Add a user to the database
    pub async fn add_user(&self, user: User) -> Result<()> {
        self.database.add_user(user)
    }
    
    /// Get a user by nickname
    pub async fn get_user_by_nick(&self, nick: &str) -> Option<User> {
        self.database.get_user_by_nick(nick)
    }
    
    /// Update a user in the database
    pub async fn update_user(&self, user: User) -> Result<()> {
        self.database.add_user(user)
    }
    
    /// Remove a user from the database
    pub async fn remove_user(&self, user_id: uuid::Uuid) -> Result<()> {
        self.database.remove_user(user_id).map(|_| ())
    }
    
    /// Add a channel to the database
    pub async fn add_channel(&self, channel: rustircd_core::ChannelInfo) -> Result<()> {
        self.database.add_channel(channel)
    }
    
    /// Get channel users
    pub async fn get_channel_users(&self, name: &str) -> Vec<String> {
        self.database.get_channel_users(name)
    }
    
    /// Add a user to a channel
    pub async fn add_user_to_channel(&self, nick: &str, channel: &str) -> Result<()> {
        self.database.add_user_to_channel(nick, channel)
    }
    
    /// Remove a user from a channel
    pub async fn remove_user_from_channel(&self, nick: &str, channel: &str) -> Result<()> {
        self.database.remove_user_from_channel(nick, channel)
    }
    
    /// Broadcast a message to all servers
    pub async fn broadcast_to_servers(&self, message: Message) -> Result<()> {
        self.server_connections.broadcast_to_servers(message).await
    }
    
    /// Send a message to a specific server
    pub async fn send_to_server(&self, server_name: &str, message: Message) -> Result<()> {
        self.server_connections.send_to_server(server_name, message).await
    }
    
    /// Send a message to a local user
    pub async fn send_to_user(&self, nick: &str, message: Message) -> Result<()> {
        // This would need to be implemented in the core to find the user's client
        // For now, we'll just log it
        tracing::debug!("Would send message to user {}: {:?}", nick, message);
        Ok(())
    }
    
    /// Send a message to a channel
    pub async fn send_to_channel(&self, channel: &str, message: Message) -> Result<()> {
        // This would need to be implemented in the core to find channel members
        // For now, we'll just log it
        tracing::debug!("Would send message to channel {}: {:?}", channel, message);
        Ok(())
    }
}

/// Service trait that all services must implement
#[async_trait]
pub trait Service: Send + Sync {
    /// Service name
    fn name(&self) -> &str;
    
    /// Service version
    fn version(&self) -> &str;
    
    /// Service description
    fn description(&self) -> &str;
    
    /// Initialize the service
    async fn init(&mut self) -> Result<()>;
    
    /// Cleanup the service
    async fn cleanup(&mut self) -> Result<()>;
    
    /// Handle a message from a client
    async fn handle_message(&mut self, client: &Client, message: &Message, context: &ServiceContext) -> Result<ServiceResult>;
    
    /// Handle a message from a server
    async fn handle_server_message(&mut self, server: &str, message: &Message, context: &ServiceContext) -> Result<ServiceResult>;
    
    /// Handle user registration
    async fn handle_user_registration(&mut self, user: &User, context: &ServiceContext) -> Result<()>;
    
    /// Handle user disconnection
    async fn handle_user_disconnection(&mut self, user: &User, context: &ServiceContext) -> Result<()>;
    
    
    /// Get service capabilities
    fn get_capabilities(&self) -> Vec<String>;
    
    /// Check if service supports a capability
    fn supports_capability(&self, capability: &str) -> bool;
}

/// Result of service message handling
#[derive(Debug, Clone)]
pub enum ServiceResult {
    /// Message was handled, continue processing
    Handled,
    /// Message was handled, stop processing
    HandledStop,
    /// Message was not handled, continue to next service
    NotHandled,
    /// Message was rejected, send error
    Rejected(String),
}

/// Service manager for loading and managing services
pub struct ServiceManager {
    services: HashMap<String, Box<dyn Service>>,
    message_handlers: Vec<String>,
    server_message_handlers: Vec<String>,
    user_handlers: Vec<String>,
    context: ServiceContext,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new(database: Arc<Database>, server_connections: Arc<ServerConnectionManager>) -> Self {
        Self {
            services: HashMap::new(),
            message_handlers: Vec::new(),
            server_message_handlers: Vec::new(),
            user_handlers: Vec::new(),
            context: ServiceContext::new(database, server_connections),
        }
    }
    
    /// Load a service
    pub async fn load_service(&mut self, mut service: Box<dyn Service>) -> Result<()> {
        let name = service.name().to_string();
        
        // Initialize the service
        service.init().await?;
        
        // Register handlers based on service capabilities
        if service.supports_capability("message_handler") {
            self.message_handlers.push(name.clone());
        }
        
        if service.supports_capability("server_message_handler") {
            self.server_message_handlers.push(name.clone());
        }
        
        if service.supports_capability("user_handler") {
            self.user_handlers.push(name.clone());
        }
        
        // Store the service
        self.services.insert(name, service);
        
        Ok(())
    }
    
    /// Unload a service
    pub async fn unload_service(&mut self, name: &str) -> Result<()> {
        if let Some(mut service) = self.services.remove(name) {
            service.cleanup().await?;
            
            // Remove from handler lists
            self.message_handlers.retain(|n| n != name);
            self.server_message_handlers.retain(|n| n != name);
            self.user_handlers.retain(|n| n != name);
        }
        
        Ok(())
    }
    
    /// Get a service by name
    pub fn get_service(&self, name: &str) -> Option<&dyn Service> {
        self.services.get(name).map(|s| s.as_ref())
    }
    
    // /// Get a mutable service by name
    // pub fn get_service_mut(&mut self, name: &str) -> Option<&mut (dyn Service + '_)> {
    //     self.services.get_mut(name).map(|s| s.as_mut())
    // }
    
    /// Handle a message from a client
    pub async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ServiceResult> {
        for service_name in &self.message_handlers {
            if let Some(service) = self.services.get_mut(service_name) {
                match service.handle_message(client, message, &self.context).await {
                    Ok(ServiceResult::HandledStop) => return Ok(ServiceResult::HandledStop),
                    Ok(ServiceResult::Rejected(reason)) => return Ok(ServiceResult::Rejected(reason)),
                    Ok(ServiceResult::Handled) => return Ok(ServiceResult::Handled),
                    Ok(ServiceResult::NotHandled) => continue,
                    Err(e) => {
                        tracing::error!("Error in service {}: {}", service_name, e);
                        continue;
                    }
                }
            }
        }
        
        Ok(ServiceResult::NotHandled)
    }
    
    /// Handle a message from a server
    pub async fn handle_server_message(&mut self, server: &str, message: &Message) -> Result<ServiceResult> {
        for service_name in &self.server_message_handlers {
            if let Some(service) = self.services.get_mut(service_name) {
                match service.handle_server_message(server, message, &self.context).await {
                    Ok(ServiceResult::HandledStop) => return Ok(ServiceResult::HandledStop),
                    Ok(ServiceResult::Rejected(reason)) => return Ok(ServiceResult::Rejected(reason)),
                    Ok(ServiceResult::Handled) => return Ok(ServiceResult::Handled),
                    Ok(ServiceResult::NotHandled) => continue,
                    Err(e) => {
                        tracing::error!("Error in service {}: {}", service_name, e);
                        continue;
                    }
                }
            }
        }
        
        Ok(ServiceResult::NotHandled)
    }
    
    /// Handle user registration
    pub async fn handle_user_registration(&mut self, user: &User) -> Result<()> {
        for service_name in &self.user_handlers {
            if let Some(service) = self.services.get_mut(service_name) {
                if let Err(e) = service.handle_user_registration(user, &self.context).await {
                    tracing::error!("Error in service {}: {}", service_name, e);
                }
            }
        }
        Ok(())
    }
    
    /// Handle user disconnection
    pub async fn handle_user_disconnection(&mut self, user: &User) -> Result<()> {
        for service_name in &self.user_handlers {
            if let Some(service) = self.services.get_mut(service_name) {
                if let Err(e) = service.handle_user_disconnection(user, &self.context).await {
                    tracing::error!("Error in service {}: {}", service_name, e);
                }
            }
        }
        Ok(())
    }
    
    
    /// Get all loaded services
    pub fn get_loaded_services(&self) -> Vec<&str> {
        self.services.keys().map(|k| k.as_str()).collect()
    }
    
    /// Get service capabilities
    pub fn get_all_capabilities(&self) -> Vec<String> {
        let mut capabilities = Vec::new();
        for service in self.services.values() {
            capabilities.extend(service.get_capabilities());
        }
        capabilities.sort();
        capabilities.dedup();
        capabilities
    }
    
    /// Check if any service supports a capability
    pub fn supports_capability(&self, capability: &str) -> bool {
        self.services.values().any(|s| s.supports_capability(capability))
    }
}

// Note: Default implementation removed as ServiceManager now requires database and server_connections
