//! IRCv3 Away Notification

use rustircd_core::{User, Result, Message, MessageType, Prefix, module::ModuleContext};
use std::collections::HashMap;
use uuid::Uuid;

/// Away notification handler
pub struct AwayNotification {
    /// User away status by user ID
    user_away: HashMap<Uuid, Option<String>>,
}

impl AwayNotification {
    pub fn new() -> Self {
        Self {
            user_away: HashMap::new(),
        }
    }
    
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing away notification");
        Ok(())
    }
    
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up away notification");
        Ok(())
    }
    
    pub async fn handle_user_disconnection(&self, user: &User) -> Result<()> {
        tracing::info!("User {} disconnected (away notification)", user.nick);
        Ok(())
    }
    
    /// Set user away status
    pub fn set_user_away(&mut self, user_id: Uuid, away_message: Option<String>) {
        self.user_away.insert(user_id, away_message);
    }
    
    /// Get user away status
    pub fn get_user_away(&self, user_id: &Uuid) -> Option<&Option<String>> {
        self.user_away.get(user_id)
    }
    
    /// Check if user is away
    pub fn is_user_away(&self, user_id: &Uuid) -> bool {
        self.user_away.get(user_id)
            .map(|away| away.is_some())
            .unwrap_or(false)
    }
    
    /// Get away message
    pub fn get_away_message(&self, user_id: &Uuid) -> Option<&String> {
        self.user_away.get(user_id)
            .and_then(|away| away.as_ref())
    }
    
    /// Remove user away status
    pub fn remove_user_away(&mut self, user_id: &Uuid) -> Option<Option<String>> {
        self.user_away.remove(user_id)
    }
    
    /// Get all away users
    pub fn get_away_users(&self) -> Vec<Uuid> {
        self.user_away.iter()
            .filter(|(_, away)| away.is_some())
            .map(|(user_id, _)| *user_id)
            .collect()
    }
    
    /// Get away user count
    pub fn get_away_count(&self) -> usize {
        self.user_away.values()
            .filter(|away| away.is_some())
            .count()
    }
    
    /// Generate away tag for message
    pub fn generate_away_tag(&self, user_id: &Uuid) -> Option<String> {
        if self.is_user_away(user_id) {
            Some("away".to_string())
        } else {
            None
        }
    }
    
    /// Broadcast away status change to channel members with away-notify capability
    pub async fn notify_away_change(&self, user_id: Uuid, is_away: bool, message: Option<&str>, context: &ModuleContext) -> Result<()> {
        // Get the user's information
        if let Some(user) = context.database.get_user(&user_id) {
            // Get all channels the user is in
            let channels = context.database.get_user_channels(&user.nick);
            
            for channel in channels {
                // Get all members of the channel
                let members = context.get_channel_users(&channel);
                
                // Create AWAY message
                let away_params = if is_away {
                    vec![message.unwrap_or("").to_string()]
                } else {
                    vec![]
                };
                
                let away_msg = Message::with_prefix(
                    Prefix::User {
                        nick: user.nick.clone(),
                        user: user.username().to_string(),
                        host: user.hostname().to_string(),
                    },
                    MessageType::Away,
                    away_params,
                );
                
                // Send to all channel members (they should check if they have away-notify enabled)
                for member_nick in members {
                    if member_nick != user.nick {
                        let _ = context.send_to_user(&member_nick, away_msg.clone()).await;
                    }
                }
            }
            
            if is_away {
                tracing::info!("User {} is now away: {:?}", user_id, message);
            } else {
                tracing::info!("User {} is no longer away", user_id);
            }
        }
        
        Ok(())
    }
    
    /// Set user away status with broadcasting
    pub async fn set_user_away_with_broadcast(&mut self, user_id: Uuid, away_message: Option<String>, context: &ModuleContext) -> Result<()> {
        let is_away = away_message.is_some();
        self.set_user_away(user_id, away_message.clone());
        self.notify_away_change(user_id, is_away, away_message.as_deref(), context).await?;
        Ok(())
    }
    
    /// Remove user away status with broadcasting
    pub async fn remove_user_away_with_broadcast(&mut self, user_id: Uuid, context: &ModuleContext) -> Result<Option<Option<String>>> {
        let old_status = self.remove_user_away(&user_id);
        if old_status.is_some() {
            self.notify_away_change(user_id, false, None, context).await?;
        }
        Ok(old_status)
    }
}
