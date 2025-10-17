//! Module system for extensible IRC daemon

use crate::{Client, Message, User, Result, ModuleNumericManager, Database, ServerConnectionManager, ChannelInfo, Config};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Context provided to modules for database and server access
pub struct ModuleContext {
    pub database: Arc<Database>,
    pub server_connections: Arc<ServerConnectionManager>,
    /// Client connection manager for sending messages to users
    pub client_connections: Arc<RwLock<HashMap<Uuid, Arc<Client>>>>,
}

impl ModuleContext {
    pub fn new(database: Arc<Database>, server_connections: Arc<ServerConnectionManager>) -> Self {
        Self {
            database,
            server_connections,
            client_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a user to the database
    pub fn add_user(&self, user: User) -> Result<()> {
        self.database.add_user(user)
    }
    
    /// Get a user by nickname
    pub fn get_user_by_nick(&self, nick: &str) -> Option<User> {
        self.database.get_user_by_nick(nick)
    }
    
    /// Update a user in the database
    pub fn update_user(&self, user: User) -> Result<()> {
        let user_id = user.id.clone();
        self.database.update_user(&user_id, user)
    }
    
    /// Remove a user from the database
    pub fn remove_user(&self, user_id: Uuid) -> Result<Option<User>> {
        self.database.remove_user(user_id)
    }
    
    /// Add a channel to the database
    pub fn add_channel(&self, channel: ChannelInfo) -> Result<()> {
        self.database.add_channel(channel)
    }
    
    /// Get channel users
    pub fn get_channel_users(&self, name: &str) -> Vec<String> {
        self.database.get_channel_users(name)
    }
    
    /// Remove a channel from the database
    pub fn remove_channel(&self, name: &str) -> Option<ChannelInfo> {
        self.database.remove_channel(name)
    }
    
    /// Add a user to a channel
    pub fn add_user_to_channel(&self, nick: &str, channel: &str) -> Result<()> {
        self.database.add_user_to_channel(nick, channel)
    }
    
    /// Remove a user from a channel
    pub fn remove_user_from_channel(&self, nick: &str, channel: &str) -> Result<()> {
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
    
    /// Send a message to a specific user
    pub async fn send_to_user(&self, nick: &str, message: Message) -> Result<()> {
        if let Some(user) = self.get_user_by_nick(nick) {
            let client_connections = self.client_connections.read().await;
            if let Some(client) = client_connections.get(&user.id) {
                client.send(message)?;
            }
        }
        Ok(())
    }
    
    /// Send a message to a channel
    pub async fn send_to_channel(&self, channel: &str, message: Message) -> Result<()> {
        let channel_users = self.get_channel_users(channel);
        let client_connections = self.client_connections.read().await;
        
        for nick in channel_users {
            if let Some(user) = self.get_user_by_nick(&nick) {
                if let Some(client) = client_connections.get(&user.id) {
                    let _ = client.send(message.clone());
                }
            }
        }
        Ok(())
    }
    
    /// Register a client connection for a user
    pub async fn register_client(&self, user_id: Uuid, client: Arc<Client>) -> Result<()> {
        let mut client_connections = self.client_connections.write().await;
        client_connections.insert(user_id, client);
        Ok(())
    }
    
    /// Unregister a client connection for a user
    pub async fn unregister_client(&self, user_id: Uuid) -> Result<()> {
        let mut client_connections = self.client_connections.write().await;
        client_connections.remove(&user_id);
        Ok(())
    }
    
    /// Get a client by ID
    pub async fn get_client_by_id(&self, client_id: Uuid) -> Option<Arc<Client>> {
        let client_connections = self.client_connections.read().await;
        client_connections.get(&client_id).cloned()
    }
    
    /// Get a user by ID
    pub async fn get_user_by_id(&self, user_id: Uuid) -> Option<User> {
        self.database.get_user(&user_id)
    }
}

/// Module trait that all modules must implement
#[async_trait]
pub trait Module: Send + Sync {
    /// Module name
    fn name(&self) -> &str;
    
    /// Module version
    fn version(&self) -> &str;
    
    /// Module description
    fn description(&self) -> &str;
    
    /// Initialize the module
    async fn init(&mut self) -> Result<()>;
    
    /// Cleanup the module
    async fn cleanup(&mut self) -> Result<()>;
    
    /// Handle a message from a client
    async fn handle_message(&mut self, client: &Client, message: &Message, context: &ModuleContext) -> Result<ModuleResult>;
    
    /// Handle a message from a client with server reference
    async fn handle_message_with_server(&mut self, client: &Client, message: &Message, _server: Option<&crate::Server>, context: &ModuleContext) -> Result<ModuleResult> {
        // Default implementation calls the original method
        self.handle_message(client, message, context).await
    }
    
    /// Handle a message from a server
    async fn handle_server_message(&mut self, server: &str, message: &Message, context: &ModuleContext) -> Result<ModuleResult>;
    
    /// Handle user registration
    async fn handle_user_registration(&mut self, user: &User, context: &ModuleContext) -> Result<()>;
    
    /// Handle user disconnection
    async fn handle_user_disconnection(&mut self, user: &User, context: &ModuleContext) -> Result<()>;
    
    
    /// Get module capabilities
    fn get_capabilities(&self) -> Vec<String>;
    
    /// Check if module supports a capability
    fn supports_capability(&self, capability: &str) -> bool;
    
    /// Get module-specific numeric replies
    fn get_numeric_replies(&self) -> Vec<u16>;
    
    /// Check if module handles a specific numeric reply
    fn handles_numeric_reply(&self, numeric: u16) -> bool;
    
    /// Handle a numeric reply (for modules that need to process them)
    async fn handle_numeric_reply(&mut self, numeric: u16, params: Vec<String>) -> Result<()>;
    
    /// Handle a STATS query for this module
    /// Returns a vector of STATS responses for the given query letter
    /// The server reference can be used to check operator privileges
    async fn handle_stats_query(&mut self, query: &str, client_id: uuid::Uuid, server: Option<&crate::Server>) -> Result<Vec<ModuleStatsResponse>>;
    
    /// Get the STATS query letters this module handles
    fn get_stats_queries(&self) -> Vec<String>;
    
    /// Register module-specific numeric replies
    fn register_numerics(&self, manager: &mut ModuleNumericManager) -> Result<()>;
}

/// Result of module message handling
#[derive(Debug, Clone)]
pub enum ModuleResult {
    /// Message was handled, continue processing
    Handled,
    /// Message was handled, stop processing
    HandledStop,
    /// Message was not handled, continue to next module
    NotHandled,
    /// Message was rejected, send error
    Rejected(String),
}

/// Module STATS response
#[derive(Debug, Clone)]
pub enum ModuleStatsResponse {
    /// Standard STATS response with query letter and data
    Stats(String, String),
    /// Module-specific STATS response
    ModuleStats(String, String),
}

/// Module manager for loading and managing modules
pub struct ModuleManager {
    modules: HashMap<String, Box<dyn Module>>,
    message_handlers: Vec<String>,
    server_message_handlers: Vec<String>,
    user_handlers: Vec<String>,
    context: ModuleContext,
}

impl ModuleManager {
    /// Create a new module manager
    pub fn new(database: Arc<Database>, server_connections: Arc<ServerConnectionManager>) -> Self {
        Self {
            modules: HashMap::new(),
            message_handlers: Vec::new(),
            server_message_handlers: Vec::new(),
            user_handlers: Vec::new(),
            context: ModuleContext::new(database, server_connections),
        }
    }
    
    /// Load a module
    pub async fn load_module(&mut self, mut module: Box<dyn Module>) -> Result<()> {
        let name = module.name().to_string();
        
        // Initialize the module
        module.init().await?;
        
        // Register handlers based on module capabilities
        if module.supports_capability("message_handler") {
            self.message_handlers.push(name.clone());
        }
        
        if module.supports_capability("server_message_handler") {
            self.server_message_handlers.push(name.clone());
        }
        
        if module.supports_capability("user_handler") {
            self.user_handlers.push(name.clone());
        }
        
        // Store the module
        self.modules.insert(name, module);
        
        Ok(())
    }
    
    /// Unload a module
    pub async fn unload_module(&mut self, name: &str) -> Result<()> {
        if let Some(mut module) = self.modules.remove(name) {
            module.cleanup().await?;
            
            // Remove from handler lists
            self.message_handlers.retain(|n| n != name);
            self.server_message_handlers.retain(|n| n != name);
            self.user_handlers.retain(|n| n != name);
        }
        
        Ok(())
    }
    
    /// Get a module by name
    pub fn get_module(&self, name: &str) -> Option<&dyn Module> {
        self.modules.get(name).map(|m| m.as_ref())
    }
    
    /// Get all loaded modules
    pub async fn get_modules(&self) -> Vec<(String, &dyn Module)> {
        self.modules.iter()
            .map(|(name, module)| (name.clone(), module.as_ref()))
            .collect()
    }
    
    /// Get a mutable module by name
    /// Note: This method is commented out due to lifetime issues with trait objects
    /// Use handle_message or other methods that work with the modules directly
    // pub fn get_module_mut(&mut self, name: &str) -> Option<&mut (dyn Module + '_)> {
    //     self.modules.get_mut(name).map(move |m| m.as_mut())
    // }
    
    /// Handle a message from a client
    pub async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ModuleResult> {
        for module_name in &self.message_handlers {
            if let Some(module) = self.modules.get_mut(module_name) {
                match module.handle_message(client, message, &self.context).await {
                    Ok(ModuleResult::HandledStop) => return Ok(ModuleResult::HandledStop),
                    Ok(ModuleResult::Rejected(reason)) => return Ok(ModuleResult::Rejected(reason)),
                    Ok(ModuleResult::Handled) => return Ok(ModuleResult::Handled),
                    Ok(ModuleResult::NotHandled) => continue,
                    Err(e) => {
                        tracing::error!("Error in module {}: {}", module_name, e);
                        continue;
                    }
                }
            }
        }
        
        Ok(ModuleResult::NotHandled)
    }
    
    /// Handle a message from a client with server reference
    pub async fn handle_message_with_server(&mut self, client: &Client, message: &Message, server: Option<&crate::Server>) -> Result<ModuleResult> {
        for module_name in &self.message_handlers {
            if let Some(module) = self.modules.get_mut(module_name) {
                match module.handle_message_with_server(client, message, server, &self.context).await {
                    Ok(ModuleResult::HandledStop) => return Ok(ModuleResult::HandledStop),
                    Ok(ModuleResult::Rejected(reason)) => return Ok(ModuleResult::Rejected(reason)),
                    Ok(ModuleResult::Handled) => return Ok(ModuleResult::Handled),
                    Ok(ModuleResult::NotHandled) => continue,
                    Err(e) => {
                        tracing::error!("Error in module {}: {}", module_name, e);
                        continue;
                    }
                }
            }
        }
        
        Ok(ModuleResult::NotHandled)
    }
    
    /// Handle a message from a server
    pub async fn handle_server_message(&mut self, server: &str, message: &Message) -> Result<ModuleResult> {
        for module_name in &self.server_message_handlers {
            if let Some(module) = self.modules.get_mut(module_name) {
                match module.handle_server_message(server, message, &self.context).await {
                    Ok(ModuleResult::HandledStop) => return Ok(ModuleResult::HandledStop),
                    Ok(ModuleResult::Rejected(reason)) => return Ok(ModuleResult::Rejected(reason)),
                    Ok(ModuleResult::Handled) => return Ok(ModuleResult::Handled),
                    Ok(ModuleResult::NotHandled) => continue,
                    Err(e) => {
                        tracing::error!("Error in module {}: {}", module_name, e);
                        continue;
                    }
                }
            }
        }
        
        Ok(ModuleResult::NotHandled)
    }
    
    /// Handle user registration
    pub async fn handle_user_registration(&mut self, user: &User) -> Result<()> {
        for module_name in &self.user_handlers {
            if let Some(module) = self.modules.get_mut(module_name) {
                if let Err(e) = module.handle_user_registration(user, &self.context).await {
                    tracing::error!("Error in module {}: {}", module_name, e);
                }
            }
        }
        Ok(())
    }
    
    /// Handle user disconnection
    pub async fn handle_user_disconnection(&mut self, user: &User) -> Result<()> {
        for module_name in &self.user_handlers {
            if let Some(module) = self.modules.get_mut(module_name) {
                if let Err(e) = module.handle_user_disconnection(user, &self.context).await {
                    tracing::error!("Error in module {}: {}", module_name, e);
                }
            }
        }
        Ok(())
    }
    
    
    /// Handle a STATS query through modules
    pub async fn handle_stats_query(&mut self, query: &str, client_id: uuid::Uuid, server: Option<&crate::Server>) -> Result<Vec<ModuleStatsResponse>> {
        let mut responses = Vec::new();
        
        for module_name in &self.message_handlers {
            if let Some(module) = self.modules.get_mut(module_name) {
                if module.get_stats_queries().contains(&query.to_string()) {
                    match module.handle_stats_query(query, client_id, server).await {
                        Ok(module_responses) => {
                            responses.extend(module_responses);
                        }
                        Err(e) => {
                            tracing::error!("Error in module {} stats query: {}", module_name, e);
                        }
                    }
                }
            }
        }
        
        Ok(responses)
    }
    
    /// Get all loaded modules
    pub fn get_loaded_modules(&self) -> Vec<&str> {
        self.modules.keys().map(|k| k.as_str()).collect()
    }
    
    /// Get module capabilities
    pub fn get_all_capabilities(&self) -> Vec<String> {
        let mut capabilities = Vec::new();
        for module in self.modules.values() {
            capabilities.extend(module.get_capabilities());
        }
        capabilities.sort();
        capabilities.dedup();
        capabilities
    }
    
    /// Check if any module supports a capability
    pub fn supports_capability(&self, capability: &str) -> bool {
        self.modules.values().any(|m| m.supports_capability(capability))
    }
    
    /// Clear all modules (for reloading)
    pub async fn clear_modules(&mut self) -> Result<()> {
        // Cleanup all modules before clearing
        for (name, mut module) in self.modules.drain() {
            if let Err(e) = module.cleanup().await {
                tracing::warn!("Failed to cleanup module {}: {}", name, e);
            }
        }
        
        // Clear handler lists
        self.message_handlers.clear();
        self.server_message_handlers.clear();
        self.user_handlers.clear();
        
        Ok(())
    }
}

impl Default for ModuleManager {
    fn default() -> Self {
        // This is a placeholder - in practice, ModuleManager should be created with proper database and server connections
        // For now, we'll create dummy Arc references, but this should be fixed in actual usage
        use std::sync::Arc;
        let database = Arc::new(Database::new(1000, 30)); // max_history_size: 1000, history_retention_days: 30
        let config = Arc::new(Config::default());
        let server_connections = Arc::new(ServerConnectionManager::new(config));
        Self::new(database, server_connections)
    }
}
