//! IRCv3 Channel Rename

use rustircd_core::{User, Error, Result};
use std::collections::HashMap;
use uuid::Uuid;

/// Channel rename handler
pub struct ChannelRename {
    /// Channel rename history
    rename_history: HashMap<String, Vec<RenameRecord>>,
    /// Pending renames
    pending_renames: HashMap<String, RenameRecord>,
}

/// Channel rename record
#[derive(Debug, Clone)]
pub struct RenameRecord {
    /// Old channel name
    pub old_name: String,
    /// New channel name
    pub new_name: String,
    /// User who initiated rename
    pub user_id: Uuid,
    /// Rename timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Rename reason
    pub reason: Option<String>,
}

impl ChannelRename {
    pub fn new() -> Self {
        Self {
            rename_history: HashMap::new(),
            pending_renames: HashMap::new(),
        }
    }
    
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing channel rename");
        Ok(())
    }
    
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up channel rename");
        Ok(())
    }
    
    /// Request channel rename
    pub fn request_rename(&mut self, old_name: String, new_name: String, user_id: Uuid, reason: Option<String>) -> Result<()> {
        if !Self::is_valid_channel_name(&old_name) {
            return Err(Error::User("Invalid old channel name".to_string()));
        }
        
        if !Self::is_valid_channel_name(&new_name) {
            return Err(Error::User("Invalid new channel name".to_string()));
        }
        
        if old_name == new_name {
            return Err(Error::User("Old and new channel names are the same".to_string()));
        }
        
        let rename_record = RenameRecord {
            old_name: old_name.clone(),
            new_name: new_name.clone(),
            user_id,
            timestamp: chrono::Utc::now(),
            reason,
        };
        
        self.pending_renames.insert(old_name, rename_record);
        tracing::info!("Channel rename requested: {} -> {}", old_name, new_name);
        Ok(())
    }
    
    /// Approve channel rename
    pub fn approve_rename(&mut self, old_name: &str) -> Result<RenameRecord> {
        if let Some(rename_record) = self.pending_renames.remove(old_name) {
            // Add to history
            self.rename_history.entry(old_name.clone())
                .or_insert_with(Vec::new)
                .push(rename_record.clone());
            
            tracing::info!("Channel rename approved: {} -> {}", old_name, rename_record.new_name);
            Ok(rename_record)
        } else {
            Err(Error::User("No pending rename for channel".to_string()))
        }
    }
    
    /// Reject channel rename
    pub fn reject_rename(&mut self, old_name: &str, reason: Option<String>) -> Result<()> {
        if let Some(rename_record) = self.pending_renames.remove(old_name) {
            tracing::info!("Channel rename rejected: {} -> {} (reason: {:?})", 
                          old_name, rename_record.new_name, reason);
            Ok(())
        } else {
            Err(Error::User("No pending rename for channel".to_string()))
        }
    }
    
    /// Get pending rename
    pub fn get_pending_rename(&self, old_name: &str) -> Option<&RenameRecord> {
        self.pending_renames.get(old_name)
    }
    
    /// Get all pending renames
    pub fn get_pending_renames(&self) -> &HashMap<String, RenameRecord> {
        &self.pending_renames
    }
    
    /// Get rename history for channel
    pub fn get_rename_history(&self, channel_name: &str) -> Option<&Vec<RenameRecord>> {
        self.rename_history.get(channel_name)
    }
    
    /// Get all rename history
    pub fn get_all_rename_history(&self) -> &HashMap<String, Vec<RenameRecord>> {
        &self.rename_history
    }
    
    /// Check if channel has pending rename
    pub fn has_pending_rename(&self, channel_name: &str) -> bool {
        self.pending_renames.contains_key(channel_name)
    }
    
    /// Check if channel has rename history
    pub fn has_rename_history(&self, channel_name: &str) -> bool {
        self.rename_history.contains_key(channel_name)
    }
    
    /// Get rename count for channel
    pub fn get_rename_count(&self, channel_name: &str) -> usize {
        self.rename_history.get(channel_name)
            .map(|history| history.len())
            .unwrap_or(0)
    }
    
    /// Get total rename count
    pub fn get_total_rename_count(&self) -> usize {
        self.rename_history.values()
            .map(|history| history.len())
            .sum()
    }
    
    /// Validate channel name
    pub fn is_valid_channel_name(name: &str) -> bool {
        if name.is_empty() || name.len() > 200 {
            return false;
        }
        
        // Channel name should start with #, &, +, or !
        if !name.starts_with('#') && !name.starts_with('&') && !name.starts_with('+') && !name.starts_with('!') {
            return false;
        }
        
        // Channel name should not contain spaces or control characters
        name.chars().all(|c| c.is_ascii() && !c.is_control() && c != ' ' && c != ',' && c != ':')
    }
    
    /// Generate rename notification message
    pub fn generate_rename_notification(&self, rename_record: &RenameRecord) -> String {
        let mut message = format!("Channel renamed from {} to {}", rename_record.old_name, rename_record.new_name);
        
        if let Some(ref reason) = rename_record.reason {
            message.push_str(&format!(" (reason: {})", reason));
        }
        
        message
    }
    
    /// Get rename statistics
    pub fn get_rename_stats(&self) -> RenameStats {
        let mut total_renames = 0;
        let mut user_rename_counts = HashMap::new();
        let mut reason_counts = HashMap::new();
        
        for history in self.rename_history.values() {
            total_renames += history.len();
            
            for record in history {
                *user_rename_counts.entry(record.user_id).or_insert(0) += 1;
                
                if let Some(ref reason) = record.reason {
                    *reason_counts.entry(reason.clone()).or_insert(0) += 1;
                }
            }
        }
        
        RenameStats {
            total_renames,
            pending_renames: self.pending_renames.len(),
            user_rename_counts,
            reason_counts,
        }
    }
}

/// Rename statistics
#[derive(Debug, Clone)]
pub struct RenameStats {
    pub total_renames: usize,
    pub pending_renames: usize,
    pub user_rename_counts: HashMap<Uuid, usize>,
    pub reason_counts: HashMap<String, usize>,
}
