//! Core extension system for IRCv3 capabilities and modules
//! 
//! This module provides hooks and extension points that allow modules
//! to extend core functionality without modifying the core itself.

use crate::{User, Message, Client, Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use async_trait::async_trait;

/// Extension point for user-related operations
#[async_trait]
pub trait UserExtension: Send + Sync {
    /// Called when a user registers
    async fn on_user_registration(&self, user: &User) -> Result<()>;
    
    /// Called when a user disconnects
    async fn on_user_disconnection(&self, user: &User) -> Result<()>;
    
    /// Called when user properties change
    async fn on_user_property_change(&self, user: &User, property: &str, old_value: &str, new_value: &str) -> Result<()>;
    
    /// Called when user joins a channel
    async fn on_user_join_channel(&self, user: &User, channel: &str) -> Result<()>;
    
    /// Called when user parts a channel
    async fn on_user_part_channel(&self, user: &User, channel: &str, reason: Option<&str>) -> Result<()>;
    
    /// Called when user changes nickname
    async fn on_user_nick_change(&self, user: &User, old_nick: &str, new_nick: &str) -> Result<()>;
    
    /// Called when user sets away status
    async fn on_user_away_change(&self, user: &User, away: bool, message: Option<&str>) -> Result<()>;
}

/// Extension point for message processing
#[async_trait]
pub trait MessageExtension: Send + Sync {
    /// Called before a message is processed
    async fn on_message_preprocess(&self, client: &Client, message: &Message) -> Result<Option<Message>>;
    
    /// Called after a message is processed
    async fn on_message_postprocess(&self, client: &Client, message: &Message, result: &crate::module::ModuleResult) -> Result<()>;
    
    /// Called when a message is sent to a user
    async fn on_message_send(&self, target_user: &User, message: &Message) -> Result<Option<Message>>;
    
    /// Called when a message is broadcasted
    async fn on_message_broadcast(&self, message: &Message, targets: &[Uuid]) -> Result<Option<Message>>;
}

/// Extension point for channel operations
#[async_trait]
pub trait ChannelExtension: Send + Sync {
    /// Called when a channel is created
    async fn on_channel_create(&self, channel: &str, creator: &User) -> Result<()>;
    
    /// Called when a channel is destroyed
    async fn on_channel_destroy(&self, channel: &str) -> Result<()>;
    
    /// Called when a channel is renamed
    async fn on_channel_rename(&self, old_name: &str, new_name: &str, renamer: &User) -> Result<()>;
    
    /// Called when channel modes change
    async fn on_channel_mode_change(&self, channel: &str, modes: &str, setter: &User) -> Result<()>;
}

/// Extension point for server operations
#[async_trait]
pub trait ServerExtension: Send + Sync {
    /// Called when a server connects
    async fn on_server_connect(&self, server_name: &str) -> Result<()>;
    
    /// Called when a server disconnects
    async fn on_server_disconnect(&self, server_name: &str) -> Result<()>;
    
    /// Called when server information changes
    async fn on_server_info_change(&self, server_name: &str, info: &str) -> Result<()>;
}

/// Extension point for capability negotiation
#[async_trait]
pub trait CapabilityExtension: Send + Sync {
    /// Get supported capabilities
    fn get_capabilities(&self) -> Vec<String>;
    
    /// Check if a capability is supported
    fn supports_capability(&self, capability: &str) -> bool;
    
    /// Handle capability negotiation
    async fn handle_capability_negotiation(&self, client: &Client, capability: &str, action: CapabilityAction) -> Result<CapabilityResult>;
    
    /// Called when capabilities are enabled for a client
    async fn on_capabilities_enabled(&self, client: &Client, capabilities: &[String]) -> Result<()>;
    
    /// Called when capabilities are disabled for a client
    async fn on_capabilities_disabled(&self, client: &Client, capabilities: &[String]) -> Result<()>;
}

/// Capability negotiation actions
#[derive(Debug, Clone)]
pub enum CapabilityAction {
    List,
    Request(Vec<String>),
    Acknowledge(Vec<String>),
    End,
}

/// Capability negotiation results
#[derive(Debug, Clone)]
pub enum CapabilityResult {
    Supported,
    NotSupported,
    RequiresParameter(String),
    CustomResponse(String),
}

/// Extension point for message tags
#[async_trait]
pub trait MessageTagExtension: Send + Sync {
    /// Process incoming message tags
    async fn process_incoming_tags(&self, client: &Client, tags: &HashMap<String, String>) -> Result<HashMap<String, String>>;
    
    /// Generate outgoing message tags
    async fn generate_outgoing_tags(&self, sender: &User, message: &Message) -> Result<HashMap<String, String>>;
    
    /// Validate message tags
    async fn validate_tags(&self, tags: &HashMap<String, String>) -> Result<()>;
}

/// Core extension manager
// #[derive(Debug)] // Commented out - trait objects don't implement Debug
pub struct ExtensionManager {
    /// User extensions
    user_extensions: Arc<RwLock<Vec<Box<dyn UserExtension>>>>,
    /// Message extensions
    message_extensions: Arc<RwLock<Vec<Box<dyn MessageExtension>>>>,
    /// Channel extensions
    channel_extensions: Arc<RwLock<Vec<Box<dyn ChannelExtension>>>>,
    /// Server extensions
    server_extensions: Arc<RwLock<Vec<Box<dyn ServerExtension>>>>,
    /// Capability extensions
    capability_extensions: Arc<RwLock<Vec<Box<dyn CapabilityExtension>>>>,
    /// Message tag extensions
    message_tag_extensions: Arc<RwLock<Vec<Box<dyn MessageTagExtension>>>>,
}

impl ExtensionManager {
    /// Create a new extension manager
    pub fn new() -> Self {
        Self {
            user_extensions: Arc::new(RwLock::new(Vec::new())),
            message_extensions: Arc::new(RwLock::new(Vec::new())),
            channel_extensions: Arc::new(RwLock::new(Vec::new())),
            server_extensions: Arc::new(RwLock::new(Vec::new())),
            capability_extensions: Arc::new(RwLock::new(Vec::new())),
            message_tag_extensions: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    // User extension management
    
    /// Register a user extension
    pub async fn register_user_extension(&self, extension: Box<dyn UserExtension>) -> Result<()> {
        let mut extensions = self.user_extensions.write().await;
        extensions.push(extension);
        Ok(())
    }
    
    /// Call user registration hooks
    pub async fn on_user_registration(&self, user: &User) -> Result<()> {
        let extensions = self.user_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_user_registration(user).await {
                tracing::warn!("User extension error on registration: {}", e);
            }
        }
        Ok(())
    }
    
    /// Call user disconnection hooks
    pub async fn on_user_disconnection(&self, user: &User) -> Result<()> {
        let extensions = self.user_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_user_disconnection(user).await {
                tracing::warn!("User extension error on disconnection: {}", e);
            }
        }
        Ok(())
    }
    
    /// Call user property change hooks
    pub async fn on_user_property_change(&self, user: &User, property: &str, old_value: &str, new_value: &str) -> Result<()> {
        let extensions = self.user_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_user_property_change(user, property, old_value, new_value).await {
                tracing::warn!("User extension error on property change: {}", e);
            }
        }
        Ok(())
    }
    
    /// Call user join channel hooks
    pub async fn on_user_join_channel(&self, user: &User, channel: &str) -> Result<()> {
        let extensions = self.user_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_user_join_channel(user, channel).await {
                tracing::warn!("User extension error on join channel: {}", e);
            }
        }
        Ok(())
    }
    
    /// Call user part channel hooks
    pub async fn on_user_part_channel(&self, user: &User, channel: &str, reason: Option<&str>) -> Result<()> {
        let extensions = self.user_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_user_part_channel(user, channel, reason).await {
                tracing::warn!("User extension error on part channel: {}", e);
            }
        }
        Ok(())
    }
    
    /// Call user nick change hooks
    pub async fn on_user_nick_change(&self, user: &User, old_nick: &str, new_nick: &str) -> Result<()> {
        let extensions = self.user_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_user_nick_change(user, old_nick, new_nick).await {
                tracing::warn!("User extension error on nick change: {}", e);
            }
        }
        Ok(())
    }
    
    /// Call user away change hooks
    pub async fn on_user_away_change(&self, user: &User, away: bool, message: Option<&str>) -> Result<()> {
        let extensions = self.user_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_user_away_change(user, away, message).await {
                tracing::warn!("User extension error on away change: {}", e);
            }
        }
        Ok(())
    }
    
    // Message extension management
    
    /// Register a message extension
    pub async fn register_message_extension(&self, extension: Box<dyn MessageExtension>) -> Result<()> {
        let mut extensions = self.message_extensions.write().await;
        extensions.push(extension);
        Ok(())
    }
    
    /// Call message preprocess hooks
    pub async fn on_message_preprocess(&self, client: &Client, message: &Message) -> Result<Option<Message>> {
        let extensions = self.message_extensions.read().await;
        let mut processed_message = Some(message.clone());
        
        for extension in extensions.iter() {
            if let Some(ref msg) = processed_message {
                match extension.on_message_preprocess(client, msg).await {
                    Ok(Some(new_msg)) => processed_message = Some(new_msg),
                    Ok(None) => processed_message = None,
                    Err(e) => {
                        tracing::warn!("Message extension error on preprocess: {}", e);
                    }
                }
            }
        }
        
        Ok(processed_message)
    }
    
    /// Call message postprocess hooks
    pub async fn on_message_postprocess(&self, client: &Client, message: &Message, result: &crate::module::ModuleResult) -> Result<()> {
        let extensions = self.message_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_message_postprocess(client, message, result).await {
                tracing::warn!("Message extension error on postprocess: {}", e);
            }
        }
        Ok(())
    }
    
    /// Call message send hooks
    pub async fn on_message_send(&self, target_user: &User, message: &Message) -> Result<Option<Message>> {
        let extensions = self.message_extensions.read().await;
        let mut processed_message = Some(message.clone());
        
        for extension in extensions.iter() {
            if let Some(ref msg) = processed_message {
                match extension.on_message_send(target_user, msg).await {
                    Ok(Some(new_msg)) => processed_message = Some(new_msg),
                    Ok(None) => processed_message = None,
                    Err(e) => {
                        tracing::warn!("Message extension error on send: {}", e);
                    }
                }
            }
        }
        
        Ok(processed_message)
    }
    
    // Capability extension management
    
    /// Register a capability extension
    pub async fn register_capability_extension(&self, extension: Box<dyn CapabilityExtension>) -> Result<()> {
        let mut extensions = self.capability_extensions.write().await;
        extensions.push(extension);
        Ok(())
    }
    
    /// Get all supported capabilities
    pub async fn get_all_capabilities(&self) -> Result<Vec<String>> {
        let extensions = self.capability_extensions.read().await;
        let mut capabilities = Vec::new();
        
        for extension in extensions.iter() {
            capabilities.extend(extension.get_capabilities());
        }
        
        Ok(capabilities)
    }
    
    /// Check if a capability is supported
    pub async fn supports_capability(&self, capability: &str) -> bool {
        let extensions = self.capability_extensions.read().await;
        extensions.iter().any(|ext| ext.supports_capability(capability))
    }
    
    /// Handle capability negotiation
    pub async fn handle_capability_negotiation(&self, client: &Client, capability: &str, action: CapabilityAction) -> Result<CapabilityResult> {
        let extensions = self.capability_extensions.read().await;
        
        for extension in extensions.iter() {
            if extension.supports_capability(capability) {
                return extension.handle_capability_negotiation(client, capability, action.clone()).await;
            }
        }
        
        Ok(CapabilityResult::NotSupported)
    }
    
    /// Call capabilities enabled hooks
    pub async fn on_capabilities_enabled(&self, client: &Client, capabilities: &[String]) -> Result<()> {
        let extensions = self.capability_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_capabilities_enabled(client, capabilities).await {
                tracing::warn!("Capability extension error on enabled: {}", e);
            }
        }
        Ok(())
    }
    
    /// Call capabilities disabled hooks
    pub async fn on_capabilities_disabled(&self, client: &Client, capabilities: &[String]) -> Result<()> {
        let extensions = self.capability_extensions.read().await;
        for extension in extensions.iter() {
            if let Err(e) = extension.on_capabilities_disabled(client, capabilities).await {
                tracing::warn!("Capability extension error on disabled: {}", e);
            }
        }
        Ok(())
    }
    
    // Message tag extension management
    
    /// Register a message tag extension
    pub async fn register_message_tag_extension(&self, extension: Box<dyn MessageTagExtension>) -> Result<()> {
        let mut extensions = self.message_tag_extensions.write().await;
        extensions.push(extension);
        Ok(())
    }
    
    /// Process incoming message tags
    pub async fn process_incoming_tags(&self, client: &Client, tags: &HashMap<String, String>) -> Result<HashMap<String, String>> {
        let extensions = self.message_tag_extensions.read().await;
        let mut processed_tags = tags.clone();
        
        for extension in extensions.iter() {
            match extension.process_incoming_tags(client, &processed_tags).await {
                Ok(new_tags) => processed_tags = new_tags,
                Err(e) => {
                    tracing::warn!("Message tag extension error on incoming: {}", e);
                }
            }
        }
        
        Ok(processed_tags)
    }
    
    /// Generate outgoing message tags
    pub async fn generate_outgoing_tags(&self, sender: &User, message: &Message) -> Result<HashMap<String, String>> {
        let extensions = self.message_tag_extensions.read().await;
        let mut tags = HashMap::new();
        
        for extension in extensions.iter() {
            match extension.generate_outgoing_tags(sender, message).await {
                Ok(new_tags) => {
                    for (key, value) in new_tags {
                        tags.insert(key, value);
                    }
                }
                Err(e) => {
                    tracing::warn!("Message tag extension error on outgoing: {}", e);
                }
            }
        }
        
        Ok(tags)
    }
    
    /// Validate message tags
    pub async fn validate_tags(&self, tags: &HashMap<String, String>) -> Result<()> {
        let extensions = self.message_tag_extensions.read().await;
        
        for extension in extensions.iter() {
            if let Err(e) = extension.validate_tags(tags).await {
                tracing::warn!("Message tag extension error on validation: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new()
    }
}
