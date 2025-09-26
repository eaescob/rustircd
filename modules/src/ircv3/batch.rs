//! IRCv3 Batch Messages

use rustircd_core::{Client, Message, Error, Result};
use std::collections::HashMap;
use uuid::Uuid;

/// Batch message handler
pub struct Batch {
    /// Active batches by batch ID
    active_batches: HashMap<String, BatchInfo>,
    /// Client batch subscriptions
    client_batches: HashMap<Uuid, Vec<String>>,
}

/// Information about an active batch
#[derive(Debug, Clone)]
pub struct BatchInfo {
    /// Batch type
    pub batch_type: String,
    /// Batch parameters
    pub params: Vec<String>,
    /// Creator client ID
    pub creator: Uuid,
    /// Messages in batch
    pub messages: Vec<Message>,
}

impl Batch {
    pub fn new() -> Self {
        Self {
            active_batches: HashMap::new(),
            client_batches: HashMap::new(),
        }
    }
    
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing batch messages");
        Ok(())
    }
    
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up batch messages");
        Ok(())
    }
    
    /// Start a new batch
    pub fn start_batch(&mut self, batch_id: String, batch_type: String, params: Vec<String>, creator: Uuid) -> Result<()> {
        if self.active_batches.contains_key(&batch_id) {
            return Err(Error::User("Batch ID already exists".to_string()));
        }
        
        let batch_info = BatchInfo {
            batch_type,
            params,
            creator,
            messages: Vec::new(),
        };
        
        self.active_batches.insert(batch_id.clone(), batch_info);
        
        // Subscribe creator to batch
        self.client_batches.entry(creator)
            .or_insert_with(Vec::new)
            .push(batch_id);
        
        tracing::info!("Started batch {} of type {}", batch_id, batch_info.batch_type);
        Ok(())
    }
    
    /// End a batch
    pub fn end_batch(&mut self, batch_id: &str) -> Result<Option<BatchInfo>> {
        if let Some(batch_info) = self.active_batches.remove(batch_id) {
            // Remove from client subscriptions
            for (_, batches) in self.client_batches.iter_mut() {
                batches.retain(|id| id != batch_id);
            }
            
            tracing::info!("Ended batch {} with {} messages", batch_id, batch_info.messages.len());
            Ok(Some(batch_info))
        } else {
            Err(Error::User("Batch not found".to_string()))
        }
    }
    
    /// Add message to batch
    pub fn add_to_batch(&mut self, batch_id: &str, message: Message) -> Result<()> {
        if let Some(batch_info) = self.active_batches.get_mut(batch_id) {
            batch_info.messages.push(message);
            Ok(())
        } else {
            Err(Error::User("Batch not found".to_string()))
        }
    }
    
    /// Check if batch exists
    pub fn batch_exists(&self, batch_id: &str) -> bool {
        self.active_batches.contains_key(batch_id)
    }
    
    /// Get batch info
    pub fn get_batch(&self, batch_id: &str) -> Option<&BatchInfo> {
        self.active_batches.get(batch_id)
    }
    
    /// Get all active batches
    pub fn get_active_batches(&self) -> &HashMap<String, BatchInfo> {
        &self.active_batches
    }
    
    /// Get client batches
    pub fn get_client_batches(&self, client_id: &Uuid) -> Option<&Vec<String>> {
        self.client_batches.get(client_id)
    }
    
    /// Generate batch ID
    pub fn generate_batch_id() -> String {
        format!("batch_{}", uuid::Uuid::new_v4().to_string().replace('-', ""))
    }
    
    /// Validate batch type
    pub fn is_valid_batch_type(batch_type: &str) -> bool {
        matches!(batch_type, 
            "chathistory" | 
            "netjoin" | 
            "netsplit" | 
            "playback" | 
            "znc.in/playback" |
            "draft/chathistory" |
            "draft/playback"
        )
    }
    
    /// Create batch message
    pub fn create_batch_message(batch_id: &str, batch_type: &str, params: &[String]) -> Message {
        let mut message_params = vec![batch_id.to_string(), batch_type.to_string()];
        message_params.extend(params.iter().cloned());
        
        Message::new(
            rustircd_core::MessageType::Custom("BATCH".to_string()),
            message_params,
        )
    }
    
    /// Create batch end message
    pub fn create_batch_end_message(batch_id: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("BATCH".to_string()),
            vec![format!("-{}", batch_id)],
        )
    }
    
    /// Parse batch message
    pub fn parse_batch_message(message: &Message) -> Result<Option<(String, String, Vec<String>)>> {
        if message.command != rustircd_core::MessageType::Custom("BATCH") {
            return Ok(None);
        }
        
        if message.params.is_empty() {
            return Err(Error::User("Invalid batch message".to_string()));
        }
        
        let batch_id = &message.params[0];
        
        if batch_id.starts_with('-') {
            // End batch
            Ok(Some((batch_id[1..].to_string(), "end".to_string(), Vec::new())))
        } else if message.params.len() >= 2 {
            // Start batch
            let batch_type = &message.params[1];
            let params = message.params[2..].to_vec();
            Ok(Some((batch_id.clone(), batch_type.clone(), params)))
        } else {
            Err(Error::User("Invalid batch message format".to_string()))
        }
    }
}
