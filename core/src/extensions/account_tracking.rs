//! Account Tracking Extension
//! 
//! This extension tracks user account information and identification status,
//! similar to Solanum's account-tracking extension.

use crate::{User, Result, Error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use async_trait::async_trait;

/// Account tracking extension - tracks user account information
/// This is similar to Solanum's account-tracking extension
pub struct AccountTrackingExtension {
    /// Service name for account tracking
    service_name: String,
    /// Account information storage
    accounts: Arc<RwLock<HashMap<Uuid, AccountInfo>>>,
}

/// Account information for tracking
#[derive(Debug, Clone)]
pub struct AccountInfo {
    /// Account name
    pub account: String,
    /// Service name
    pub service: String,
    /// When the account was set
    pub set_time: chrono::DateTime<chrono::Utc>,
    /// Whether the user is identified
    pub identified: bool,
}

impl AccountTrackingExtension {
    /// Create a new account tracking extension
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            accounts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Set account for a user
    pub async fn set_account(&self, user_id: Uuid, account: String) -> Result<()> {
        let mut accounts = self.accounts.write().await;
        accounts.insert(user_id, AccountInfo {
            account: account.clone(),
            service: self.service_name.clone(),
            set_time: chrono::Utc::now(),
            identified: true,
        });
        Ok(())
    }
    
    /// Clear account for a user
    pub async fn clear_account(&self, user_id: Uuid) -> Result<()> {
        let mut accounts = self.accounts.write().await;
        accounts.remove(&user_id);
        Ok(())
    }
    
    /// Get account information for a user
    pub async fn get_account(&self, user_id: Uuid) -> Option<AccountInfo> {
        let accounts = self.accounts.read().await;
        accounts.get(&user_id).cloned()
    }
    
    /// Check if user is identified
    pub async fn is_identified(&self, user_id: Uuid) -> bool {
        let accounts = self.accounts.read().await;
        accounts.get(&user_id).map(|info| info.identified).unwrap_or(false)
    }
}

#[async_trait]
impl crate::extensions::UserExtension for AccountTrackingExtension {
    /// Called when a user registers
    async fn on_user_registration(&self, user: &User) -> Result<()> {
        // Initialize account tracking for new user
        let mut accounts = self.accounts.write().await;
        accounts.insert(user.id, AccountInfo {
            account: String::new(),
            service: self.service_name.clone(),
            set_time: chrono::Utc::now(),
            identified: false,
        });
        Ok(())
    }
    
    /// Called when a user disconnects
    async fn on_user_disconnection(&self, user: &User) -> Result<()> {
        // Clean up account information when user disconnects
        let mut accounts = self.accounts.write().await;
        accounts.remove(&user.id);
        Ok(())
    }
    
    /// Called when user properties change
    async fn on_user_property_change(&self, user: &User, property: &str, old_value: &str, new_value: &str) -> Result<()> {
        if property == "account" {
            if new_value.is_empty() {
                self.clear_account(user.id).await?;
            } else {
                self.set_account(user.id, new_value.to_string()).await?;
            }
        }
        Ok(())
    }
    
    /// Called when user joins a channel
    async fn on_user_join_channel(&self, _user: &User, _channel: &str) -> Result<()> {
        // No special handling needed for channel joins
        Ok(())
    }
    
    /// Called when user parts a channel
    async fn on_user_part_channel(&self, _user: &User, _channel: &str, _reason: Option<&str>) -> Result<()> {
        // No special handling needed for channel parts
        Ok(())
    }
    
    /// Called when user changes nickname
    async fn on_user_nick_change(&self, _user: &User, _old_nick: &str, _new_nick: &str) -> Result<()> {
        // Account information persists across nick changes
        Ok(())
    }
    
    /// Called when user sets away status
    async fn on_user_away_change(&self, _user: &User, _away: bool, _message: Option<&str>) -> Result<()> {
        // No special handling needed for away changes
        Ok(())
    }
}

impl Default for AccountTrackingExtension {
    fn default() -> Self {
        Self::new("services.example.org".to_string())
    }
}
