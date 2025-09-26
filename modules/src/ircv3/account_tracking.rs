//! IRCv3 Account Tracking

use rustircd_core::{User, Error, Result};
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
}
