//! IRCv3 capabilities and extensions

pub mod capability_negotiation;
pub mod message_tags;
pub mod account_tracking;
pub mod away_notification;
pub mod batch;
pub mod bot_mode;
pub mod channel_rename;
pub mod user_properties;
pub mod core_integration;

use rustircd_core::{Module, module::ModuleResult, Client, Message, User, Result};
use async_trait::async_trait;
use std::collections::HashSet;

/// IRCv3 support module
pub struct Ircv3Module {
    name: String,
    version: String,
    description: String,
    capabilities: HashSet<String>,
    capability_negotiation: capability_negotiation::CapabilityNegotiation,
    message_tags: message_tags::MessageTags,
    account_tracking: account_tracking::AccountTracking,
    away_notification: away_notification::AwayNotification,
    batch: batch::Batch,
    bot_mode: bot_mode::BotMode,
    channel_rename: channel_rename::ChannelRename,
    user_properties: user_properties::UserProperties,
}

impl Ircv3Module {
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
            name: "ircv3".to_string(),
            version: "1.0.0".to_string(),
            description: "IRCv3 capability negotiation and extensions".to_string(),
            capabilities,
            capability_negotiation: capability_negotiation::CapabilityNegotiation::new(),
            message_tags: message_tags::MessageTags::new(),
            account_tracking: account_tracking::AccountTracking::new(),
            away_notification: away_notification::AwayNotification::new(),
            batch: batch::Batch::new(),
            bot_mode: bot_mode::BotMode::new(),
            channel_rename: channel_rename::ChannelRename::new(),
            user_properties: user_properties::UserProperties::new(),
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
        
        // Initialize all capabilities
        self.capability_negotiation.init().await?;
        self.message_tags.init().await?;
        self.account_tracking.init().await?;
        self.away_notification.init().await?;
        self.batch.init().await?;
        self.bot_mode.init().await?;
        self.channel_rename.init().await?;
        self.user_properties.init().await?;
        
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up IRCv3 module");
        
        // Cleanup all capabilities
        self.capability_negotiation.cleanup().await?;
        self.message_tags.cleanup().await?;
        self.account_tracking.cleanup().await?;
        self.away_notification.cleanup().await?;
        self.batch.cleanup().await?;
        self.bot_mode.cleanup().await?;
        self.channel_rename.cleanup().await?;
        self.user_properties.cleanup().await?;
        
        Ok(())
    }
    
    async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ModuleResult> {
        match &message.command {
            rustircd_core::MessageType::Cap => {
                self.capability_negotiation.handle_cap(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Custom(cmd) => {
                match cmd.as_str() {
                    "TAGMSG" => {
                        self.message_tags.handle_tagmsg(client, message).await?;
                        Ok(ModuleResult::Handled)
                    }
                    _ => Ok(ModuleResult::NotHandled),
                }
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, user: &User) -> Result<()> {
        self.account_tracking.handle_user_registration(user).await?;
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, user: &User) -> Result<()> {
        self.account_tracking.handle_user_disconnection(user).await?;
        self.away_notification.handle_user_disconnection(user).await?;
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
