//! IRCv3 Bot Mode Registration

use rustircd_core::{Error, Result};
use std::collections::HashMap;
use uuid::Uuid;

/// Bot mode handler
pub struct BotMode {
    /// Bot users by user ID
    bot_users: HashMap<Uuid, BotInfo>,
}

/// Information about a bot user
#[derive(Debug, Clone)]
pub struct BotInfo {
    /// Bot name
    pub name: String,
    /// Bot description
    pub description: Option<String>,
    /// Bot version
    pub version: Option<String>,
    /// Bot capabilities
    pub capabilities: Vec<String>,
    /// Registration time
    pub registered_at: chrono::DateTime<chrono::Utc>,
}

impl BotMode {
    pub fn new() -> Self {
        Self {
            bot_users: HashMap::new(),
        }
    }
    
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing bot mode");
        Ok(())
    }
    
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up bot mode");
        Ok(())
    }
    
    /// Register a bot
    pub fn register_bot(&mut self, user_id: Uuid, name: String, description: Option<String>, version: Option<String>, capabilities: Vec<String>) -> Result<()> {
        if self.bot_users.contains_key(&user_id) {
            return Err(Error::User("User is already registered as bot".to_string()));
        }
        
        let bot_info = BotInfo {
            name,
            description,
            version,
            capabilities,
            registered_at: chrono::Utc::now(),
        };
        
        self.bot_users.insert(user_id, bot_info);
        tracing::info!("Registered bot for user {}", user_id);
        Ok(())
    }
    
    /// Unregister a bot
    pub fn unregister_bot(&mut self, user_id: &Uuid) -> Result<Option<BotInfo>> {
        if let Some(bot_info) = self.bot_users.remove(user_id) {
            tracing::info!("Unregistered bot for user {}", user_id);
            Ok(Some(bot_info))
        } else {
            Err(Error::User("User is not registered as bot".to_string()))
        }
    }
    
    /// Check if user is a bot
    pub fn is_bot(&self, user_id: &Uuid) -> bool {
        self.bot_users.contains_key(user_id)
    }
    
    /// Get bot info
    pub fn get_bot_info(&self, user_id: &Uuid) -> Option<&BotInfo> {
        self.bot_users.get(user_id)
    }
    
    /// Get all bots
    pub fn get_all_bots(&self) -> &HashMap<Uuid, BotInfo> {
        &self.bot_users
    }
    
    /// Get bot count
    pub fn get_bot_count(&self) -> usize {
        self.bot_users.len()
    }
    
    /// Update bot capabilities
    pub fn update_bot_capabilities(&mut self, user_id: &Uuid, capabilities: Vec<String>) -> Result<()> {
        if let Some(bot_info) = self.bot_users.get_mut(user_id) {
            bot_info.capabilities = capabilities;
            Ok(())
        } else {
            Err(Error::User("User is not registered as bot".to_string()))
        }
    }
    
    /// Update bot description
    pub fn update_bot_description(&mut self, user_id: &Uuid, description: Option<String>) -> Result<()> {
        if let Some(bot_info) = self.bot_users.get_mut(user_id) {
            bot_info.description = description;
            Ok(())
        } else {
            Err(Error::User("User is not registered as bot".to_string()))
        }
    }
    
    /// Update bot version
    pub fn update_bot_version(&mut self, user_id: &Uuid, version: Option<String>) -> Result<()> {
        if let Some(bot_info) = self.bot_users.get_mut(user_id) {
            bot_info.version = version;
            Ok(())
        } else {
            Err(Error::User("User is not registered as bot".to_string()))
        }
    }
    
    /// Generate bot tag for message
    pub fn generate_bot_tag(&self, user_id: &Uuid) -> Option<String> {
        if self.is_bot(user_id) {
            Some("bot".to_string())
        } else {
            None
        }
    }
    
    /// Validate bot name
    pub fn is_valid_bot_name(name: &str) -> bool {
        !name.is_empty() && name.len() <= 50 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }
    
    /// Get bot statistics
    pub fn get_bot_stats(&self) -> BotStats {
        let mut total_capabilities = 0;
        let mut capability_counts = std::collections::HashMap::new();
        
        for bot in self.bot_users.values() {
            total_capabilities += bot.capabilities.len();
            for capability in &bot.capabilities {
                *capability_counts.entry(capability.clone()).or_insert(0) += 1;
            }
        }
        
        BotStats {
            total_bots: self.bot_users.len(),
            total_capabilities,
            capability_counts,
        }
    }
}

/// Bot statistics
#[derive(Debug, Clone)]
pub struct BotStats {
    pub total_bots: usize,
    pub total_capabilities: usize,
    pub capability_counts: std::collections::HashMap<String, usize>,
}
