//! IRCv3 Account Tracking

use rustircd_core::{User, Error, Result, Message, MessageType, module::ModuleContext};
use std::collections::HashMap;
use uuid::Uuid;

/// Account tracking handler
pub struct AccountTracking {
    /// User accounts by user ID
    user_accounts: HashMap<Uuid, String>,
    /// Account names by account
    account_users: HashMap<String, Uuid>,
}

impl AccountTracking {
    pub fn new() -> Self {
        Self {
            user_accounts: HashMap::new(),
            account_users: HashMap::new(),
        }
    }
    
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing account tracking");
        Ok(())
    }
    
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up account tracking");
        Ok(())
    }
    
    pub async fn handle_user_registration(&self, user: &User) -> Result<()> {
        tracing::info!("User {} registered (account tracking)", user.nick);
        Ok(())
    }
    
    pub async fn handle_user_disconnection(&self, user: &User) -> Result<()> {
        tracing::info!("User {} disconnected (account tracking)", user.nick);
        Ok(())
    }
    
    /// Set user account
    pub fn set_user_account(&mut self, user_id: Uuid, account: String) -> Result<()> {
        // Remove old account if exists
        if let Some(old_account) = self.user_accounts.get(&user_id) {
            self.account_users.remove(old_account);
        }
        
        // Check if account is already in use
        if self.account_users.contains_key(&account) {
            return Err(Error::User("Account already in use".to_string()));
        }
        
        // Set new account
        self.user_accounts.insert(user_id, account.clone());
        self.account_users.insert(account, user_id);
        
        Ok(())
    }
    
    /// Remove user account
    pub fn remove_user_account(&mut self, user_id: Uuid) -> Option<String> {
        if let Some(account) = self.user_accounts.remove(&user_id) {
            self.account_users.remove(&account);
            Some(account)
        } else {
            None
        }
    }
    
    /// Get user account
    pub fn get_user_account(&self, user_id: &Uuid) -> Option<&String> {
        self.user_accounts.get(user_id)
    }
    
    /// Get user by account
    pub fn get_user_by_account(&self, account: &str) -> Option<&Uuid> {
        self.account_users.get(account)
    }
    
    /// Check if user has account
    pub fn has_account(&self, user_id: &Uuid) -> bool {
        self.user_accounts.contains_key(user_id)
    }
    
    /// Check if account exists
    pub fn account_exists(&self, account: &str) -> bool {
        self.account_users.contains_key(account)
    }
    
    /// Get all accounts
    pub fn get_all_accounts(&self) -> Vec<&String> {
        self.user_accounts.values().collect()
    }
    
    /// Get account count
    pub fn get_account_count(&self) -> usize {
        self.user_accounts.len()
    }
    
    /// Validate account name
    pub fn is_valid_account_name(account: &str) -> bool {
        if account.is_empty() || account.len() > 20 {
            return false;
        }
        
        // Account names should be alphanumeric and may contain underscores
        account.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
    
    /// Generate account tag for message
    pub fn generate_account_tag(&self, user_id: &Uuid) -> Option<String> {
        if let Some(account) = self.get_user_account(user_id) {
            Some(format!("account={}", account))
        } else {
            None
        }
    }
    
    /// Broadcast account change to relevant channel members
    pub async fn broadcast_account_change(&self, user_id: Uuid, account: Option<&str>, context: &ModuleContext) -> Result<()> {
        // Get the user's nickname
        if let Some(user) = context.database.get_user(&user_id) {
            // Get all channels the user is in
            let channels = context.database.get_user_channels(&user.nick);
            
            for channel in channels {
                // Get all members of the channel
                let members = context.get_channel_users(&channel);
                
                // Create ACCOUNT message
                let account_str = account.unwrap_or("*").to_string();
                let account_msg = Message::with_prefix(
                    rustircd_core::Prefix::User {
                        nick: user.nick.clone(),
                        user: user.username().to_string(),
                        host: user.hostname().to_string(),
                    },
                    MessageType::Custom("ACCOUNT".to_string()),
                    vec![account_str],
                );
                
                // Send to all channel members
                for member_nick in members {
                    if member_nick != user.nick {
                        let _ = context.send_to_user(&member_nick, account_msg.clone()).await;
                    }
                }
            }
            
            tracing::info!("Broadcasted account change for user {} to channel members", user_id);
        }
        
        Ok(())
    }
    
    /// Set user account with database update and broadcasting
    pub async fn set_user_account_with_broadcast(&mut self, user_id: Uuid, account: String, context: &ModuleContext) -> Result<()> {
        // Set in local tracking
        self.set_user_account(user_id, account.clone())?;
        
        // Broadcast the change
        self.broadcast_account_change(user_id, Some(&account), context).await?;
        
        Ok(())
    }
    
    /// Remove user account with broadcasting
    pub async fn remove_user_account_with_broadcast(&mut self, user_id: Uuid, context: &ModuleContext) -> Result<Option<String>> {
        let account = self.remove_user_account(user_id);
        
        if account.is_some() {
            // Broadcast account removal (*)
            self.broadcast_account_change(user_id, None, context).await?;
        }
        
        Ok(account)
    }
}
