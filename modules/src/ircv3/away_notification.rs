//! IRCv3 Away Notification

use rustircd_core::{User, Result};
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
    
    /// Implement away status change notifications
    /// TODO: Integrate with channel membership system for targeted notifications
    pub async fn notify_away_change(&self, user_id: Uuid, is_away: bool, message: Option<&str>) -> Result<()> {
        // Implement away notification system
        // In production, this would:
        // 1. Get all users in channels with the user who changed away status
        // 2. Send AWAY_NOTIFY message to users who have the away-notify capability
        // 3. Include user ID, away status, and optional away message
        
        if is_away {
            tracing::info!("User {} is now away: {:?}", user_id, message);
            
            // In production, would send:
            // for target_user in channel_members_with_away_notify_capability {
            //     let away_msg = Message::new(
            //         MessageType::Custom("AWAY_NOTIFY".to_string()),
            //         vec![user_id.to_string(), "1".to_string(), message.unwrap_or("").to_string()]
            //     );
            //     target_user.send(away_msg)?;
            // }
        } else {
            tracing::info!("User {} is no longer away", user_id);
            
            // In production, would send:
            // for target_user in channel_members_with_away_notify_capability {
            //     let away_msg = Message::new(
            //         MessageType::Custom("AWAY_NOTIFY".to_string()),
            //         vec![user_id.to_string(), "0".to_string(), "".to_string()]
            //     );
            //     target_user.send(away_msg)?;
            // }
        }
        
        tracing::debug!("Would send away notifications for user {} to channel members", user_id);
        Ok(())
    }
}
