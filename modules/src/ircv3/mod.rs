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
pub mod extended_join;
pub mod multi_prefix;

use rustircd_core::{Module, module::ModuleResult, Client, Message, User, Result, module::ModuleContext};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    extended_join: Arc<Mutex<extended_join::ExtendedJoin>>,
    multi_prefix: Arc<Mutex<multi_prefix::MultiPrefix>>,
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
            extended_join: Arc::new(Mutex::new(extended_join::ExtendedJoin::new())),
            multi_prefix: Arc::new(Mutex::new(multi_prefix::MultiPrefix::new())),
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
        
        // Set up capability callbacks
        let extended_join = Arc::clone(&self.extended_join);
        let multi_prefix = Arc::clone(&self.multi_prefix);
        
        self.capability_negotiation.set_on_capabilities_enabled(move |client_id, capabilities| {
            let extended_join = extended_join.clone();
            let multi_prefix = multi_prefix.clone();
            let capabilities = capabilities.to_vec();
            tokio::spawn(async move {
                for cap in capabilities {
                    match cap.as_str() {
                        "extended-join" => {
                            let mut ej = extended_join.lock().await;
                            ej.enable_for_client(client_id);
                        }
                        "multi-prefix" => {
                            let mut mp = multi_prefix.lock().await;
                            mp.enable_for_client(client_id);
                        }
                        _ => {}
                    }
                }
            });
        });
        
        let extended_join = Arc::clone(&self.extended_join);
        let multi_prefix = Arc::clone(&self.multi_prefix);
        
        self.capability_negotiation.set_on_capabilities_disabled(move |client_id, capabilities| {
            let extended_join = extended_join.clone();
            let multi_prefix = multi_prefix.clone();
            let capabilities = capabilities.to_vec();
            tokio::spawn(async move {
                for cap in capabilities {
                    match cap.as_str() {
                        "extended-join" => {
                            let mut ej = extended_join.lock().await;
                            ej.disable_for_client(client_id);
                        }
                        "multi-prefix" => {
                            let mut mp = multi_prefix.lock().await;
                            mp.disable_for_client(client_id);
                        }
                        _ => {}
                    }
                }
            });
        });
        
        // Initialize all capabilities
        self.capability_negotiation.init().await?;
        self.message_tags.init().await?;
        self.account_tracking.init().await?;
        self.away_notification.init().await?;
        self.batch.init().await?;
        self.bot_mode.init().await?;
        self.channel_rename.init().await?;
        self.user_properties.init().await?;
        {
            let mut ej = self.extended_join.lock().await;
            ej.init().await?;
        }
        {
            let mut mp = self.multi_prefix.lock().await;
            mp.init().await?;
        }
        
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
        {
            let mut ej = self.extended_join.lock().await;
            ej.cleanup().await?;
        }
        {
            let mut mp = self.multi_prefix.lock().await;
            mp.cleanup().await?;
        }

        Ok(())
    }

    fn register_numerics(&self, _manager: &mut rustircd_core::ModuleNumericManager) -> Result<()> {
        Ok(())
    }
    
    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
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
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, user: &User, _context: &ModuleContext) -> Result<()> {
        self.account_tracking.handle_user_registration(user).await?;
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, user: &User, _context: &ModuleContext) -> Result<()> {
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

    async fn handle_stats_query(&mut self, _query: &str, _client_id: uuid::Uuid, _server: Option<&rustircd_core::Server>) -> Result<Vec<rustircd_core::module::ModuleStatsResponse>> {
        // IRCv3 module doesn't provide STATS queries
        Ok(vec![])
    }

    fn get_stats_queries(&self) -> Vec<String> {
        // IRCv3 module doesn't provide STATS queries
        vec![]
    }
}

impl Ircv3Module {
    /// Enable extended join for a client
    pub async fn enable_extended_join(&mut self, client_id: uuid::Uuid) {
        let mut ej = self.extended_join.lock().await;
        ej.enable_for_client(client_id);
    }
    
    /// Disable extended join for a client
    pub async fn disable_extended_join(&mut self, client_id: uuid::Uuid) {
        let mut ej = self.extended_join.lock().await;
        ej.disable_for_client(client_id);
    }
    
    /// Check if extended join is enabled for a client
    pub async fn is_extended_join_enabled(&self, client_id: &uuid::Uuid) -> bool {
        let ej = self.extended_join.lock().await;
        ej.is_enabled_for_client(client_id)
    }
    
    /// Create an extended JOIN message
    pub async fn create_extended_join_message(
        &self,
        client: &Client,
        channel: &str,
        account_name: Option<&str>,
        real_name: Option<&str>,
    ) -> Result<Message> {
        let ej = self.extended_join.lock().await;
        ej.create_extended_join_message(client, channel, account_name, real_name)
    }
    
    /// Create a standard JOIN message
    pub async fn create_standard_join_message(
        &self,
        client: &Client,
        channel: &str,
    ) -> Result<Message> {
        let ej = self.extended_join.lock().await;
        ej.create_standard_join_message(client, channel)
    }
    
    /// Handle JOIN command with extended join support
    pub async fn handle_join(
        &self,
        client: &Client,
        channel: &str,
        account_name: Option<&str>,
        real_name: Option<&str>,
    ) -> Result<Message> {
        let ej = self.extended_join.lock().await;
        ej.handle_join(client, channel, account_name, real_name).await
    }
    
    /// Enable multi-prefix for a client
    pub async fn enable_multi_prefix(&mut self, client_id: uuid::Uuid) {
        let mut mp = self.multi_prefix.lock().await;
        mp.enable_for_client(client_id);
    }
    
    /// Disable multi-prefix for a client
    pub async fn disable_multi_prefix(&mut self, client_id: uuid::Uuid) {
        let mut mp = self.multi_prefix.lock().await;
        mp.disable_for_client(client_id);
    }
    
    /// Check if multi-prefix is enabled for a client
    pub async fn is_multi_prefix_enabled(&self, client_id: &uuid::Uuid) -> bool {
        let mp = self.multi_prefix.lock().await;
        mp.is_enabled_for_client(client_id)
    }
    
    /// Create a NAMES reply with multiple prefixes
    pub async fn create_names_reply(
        &self,
        client: &Client,
        channel: &str,
        names: &[String],
    ) -> Result<Message> {
        let mp = self.multi_prefix.lock().await;
        mp.create_names_reply(client, channel, names)
    }
    
    /// Create an end of NAMES reply
    pub async fn create_end_of_names_reply(&self, channel: &str) -> Result<Message> {
        let mp = self.multi_prefix.lock().await;
        mp.create_end_of_names_reply(channel)
    }
    
    /// Process channel members and format their names based on capability
    pub async fn process_channel_members(
        &self,
        client: &Client,
        members: &[(uuid::Uuid, std::collections::HashSet<char>)],
        user_database: &dyn Fn(&uuid::Uuid) -> Option<String>,
    ) -> Vec<String> {
        let mp = self.multi_prefix.lock().await;
        mp.process_channel_members(client, members, user_database)
    }
    
    /// Get account name from user data
    pub async fn get_account_name(&self, client: &Client) -> Option<String> {
        let ej = self.extended_join.lock().await;
        ej.get_account_name(client)
    }
    
    /// Get real name from user data
    pub async fn get_real_name(&self, client: &Client) -> Option<String> {
        let ej = self.extended_join.lock().await;
        ej.get_real_name(client)
    }
}
