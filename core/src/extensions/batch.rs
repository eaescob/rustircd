//! Batch Extension
//! 
//! This extension handles message batching for efficient network communication,
//! similar to Solanum's batch extension.

use crate::{User, Message, Client, Result, Error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use async_trait::async_trait;

/// Batch extension - handles message batching
/// This is similar to Solanum's batch extension
pub struct BatchExtension {
    /// Active batches
    active_batches: Arc<RwLock<HashMap<String, BatchInfo>>>,
}

/// Information about an active batch
#[derive(Debug, Clone)]
pub struct BatchInfo {
    /// Batch ID
    pub id: String,
    /// Batch type
    pub batch_type: String,
    /// User who started the batch
    pub user_id: Uuid,
    /// When the batch was started
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Messages in the batch
    pub messages: Vec<Message>,
}

impl BatchExtension {
    /// Create a new batch extension
    pub fn new() -> Self {
        Self {
            active_batches: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start a new batch
    pub async fn start_batch(&self, batch_id: String, batch_type: String, user_id: Uuid) -> Result<()> {
        let mut batches = self.active_batches.write().await;
        batches.insert(batch_id.clone(), BatchInfo {
            id: batch_id,
            batch_type,
            user_id,
            start_time: chrono::Utc::now(),
            messages: Vec::new(),
        });
        Ok(())
    }
    
    /// Add message to batch
    pub async fn add_to_batch(&self, batch_id: &str, message: Message) -> Result<()> {
        let mut batches = self.active_batches.write().await;
        if let Some(batch) = batches.get_mut(batch_id) {
            batch.messages.push(message);
        }
        Ok(())
    }
    
    /// End a batch and return all messages
    pub async fn end_batch(&self, batch_id: &str) -> Result<Option<Vec<Message>>> {
        let mut batches = self.active_batches.write().await;
        if let Some(batch) = batches.remove(batch_id) {
            Ok(Some(batch.messages))
        } else {
            Ok(None)
        }
    }
    
    /// Check if a batch exists
    pub async fn has_batch(&self, batch_id: &str) -> bool {
        let batches = self.active_batches.read().await;
        batches.contains_key(batch_id)
    }
}

#[async_trait]
impl crate::extensions::MessageExtension for BatchExtension {
    /// Called before a message is processed
    async fn on_message_preprocess(&self, client: &Client, message: &Message) -> Result<Option<Message>> {
        // Check if this is a batch start message
        if message.command == "BATCH" && message.params.len() >= 2 {
            let batch_id = &message.params[0];
            let batch_type = &message.params[1];
            
            if batch_id.starts_with('+') {
                // Starting a batch
                let batch_id = batch_id[1..].to_string();
                self.start_batch(batch_id, batch_type.clone(), client.user_id).await?;
            } else if batch_id.starts_with('-') {
                // Ending a batch
                let batch_id = &batch_id[1..];
                if let Some(messages) = self.end_batch(batch_id).await? {
                    // Process all batched messages
                    for msg in messages {
                        // Here you would typically process each message in the batch
                        tracing::debug!("Processing batched message: {:?}", msg);
                    }
                }
            }
        }
        
        Ok(Some(message.clone()))
    }
    
    /// Called after a message is processed
    async fn on_message_postprocess(&self, _client: &Client, _message: &Message, _result: &crate::module::ModuleResult) -> Result<()> {
        // No special post-processing needed
        Ok(())
    }
    
    /// Called when a message is sent to a user
    async fn on_message_send(&self, _target_user: &User, message: &Message) -> Result<Option<Message>> {
        // Check if this message should be added to a batch
        if let Some(batch_id) = self.get_active_batch_for_message(message).await {
            self.add_to_batch(&batch_id, message.clone()).await?;
            return Ok(None); // Don't send the message immediately
        }
        
        Ok(Some(message.clone()))
    }
    
    /// Called when a message is broadcasted
    async fn on_message_broadcast(&self, message: &Message, _targets: &[Uuid]) -> Result<Option<Message>> {
        // Check if this message should be added to a batch
        if let Some(batch_id) = self.get_active_batch_for_message(message).await {
            self.add_to_batch(&batch_id, message.clone()).await?;
            return Ok(None); // Don't broadcast the message immediately
        }
        
        Ok(Some(message.clone()))
    }
}

impl BatchExtension {
    /// Get active batch ID for a message (simplified implementation)
    async fn get_active_batch_for_message(&self, _message: &Message) -> Option<String> {
        // This is a simplified implementation
        // In a real implementation, you would determine which batch a message belongs to
        // based on the message content, user context, etc.
        None
    }
}

impl Default for BatchExtension {
    fn default() -> Self {
        Self::new()
    }
}
