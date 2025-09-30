//! Core burst extensions for server synchronization

use crate::{extensions::{BurstExtension, BurstType}, User, Message, Result, Database};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Core user burst extension
pub struct CoreUserBurstExtension {
    database: Arc<Database>,
    server_name: String,
}

impl CoreUserBurstExtension {
    pub fn new(database: Arc<Database>, server_name: String) -> Self {
        Self {
            database,
            server_name,
        }
    }
}

#[async_trait]
impl BurstExtension for CoreUserBurstExtension {
    async fn on_prepare_burst(&self, _target_server: &str, burst_type: &BurstType) -> Result<Vec<Message>> {
        if !matches!(burst_type, BurstType::User) {
            return Ok(Vec::new());
        }
        
        let mut messages = Vec::new();
        let database = &*self.database;
        
        // Get all local users
        for user in database.get_all_users() {
            if let Some(user) = database.get_user(&user.id) {
                // Only send local users (not users from other servers)
                if user.server == self.server_name {
                    let user_burst = Message::new(
                        crate::MessageType::UserBurst,
                        vec![
                            user.nick.clone(),
                            user.username.clone(),
                            user.host.clone(),
                            user.realname.clone(),
                            user.server.clone(),
                            user.id.to_string(),
                            user.registered_at.to_rfc3339(),
                        ]
                    );
                    messages.push(user_burst);
                }
            }
        }
        
        Ok(messages)
    }
    
    async fn on_receive_burst(&self, source_server: &str, burst_type: &BurstType, messages: &[Message]) -> Result<()> {
        if !matches!(burst_type, BurstType::User) {
            return Ok(());
        }
        
        let database = &*self.database;
        
        for message in messages {
            if message.params.len() >= 7 {
                let nick = &message.params[0];
                let user = &message.params[1];
                let host = &message.params[2];
                let realname = &message.params[3];
                let user_server = &message.params[4];
                let user_id_str = &message.params[5];
                let connected_at_str = &message.params[6];
                
                // Parse user ID
                if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                    // Parse connected time
                    if let Ok(connected_at) = chrono::DateTime::parse_from_rfc3339(connected_at_str) {
                        // Create remote user
                        let remote_user = User {
                            id: user_id,
                            nick: nick.clone(),
                            username: user.clone(),
                            host: host.clone(),
                            realname: realname.clone(),
                            server: user_server.clone(),
                            registered_at: connected_at.with_timezone(&chrono::Utc),
                            last_activity: chrono::Utc::now(),
                            modes: std::collections::HashSet::new(),
                            channels: std::collections::HashSet::new(),
                            registered: true,
                            is_operator: false,
                            operator_flags: std::collections::HashSet::new(),
                            is_bot: false,
                            bot_info: None,
                            away_message: None,
                        };
                        
                        // Add to database
                        if let Err(e) = database.add_user(remote_user) {
                            tracing::warn!("Failed to add remote user {} from server {}: {}", nick, source_server, e);
                        } else {
                            tracing::debug!("Added remote user {} from server {}", nick, source_server);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn on_server_connect_burst(&self, target_server: &str) -> Result<Vec<Message>> {
        // Use the same logic as prepare_burst for server connect
        self.on_prepare_burst(target_server, &BurstType::User).await
    }
    
    async fn on_server_disconnect_cleanup(&self, source_server: &str) -> Result<()> {
        let database = &*self.database;
        
        // Remove all users from the disconnected server
        let users_to_remove: Vec<Uuid> = database.get_all_users()
            .into_iter()
            .filter(|user| user.server == source_server)
            .map(|user| user.id)
            .collect();
        
        for user_id in users_to_remove {
            if let Err(e) = database.remove_user(user_id) {
                tracing::warn!("Failed to remove user {} from disconnected server {}: {}", user_id, source_server, e);
            }
        }
        
        tracing::info!("Cleaned up users from disconnected server {}", source_server);
        Ok(())
    }
    
    fn get_supported_burst_types(&self) -> Vec<BurstType> {
        vec![BurstType::User]
    }
    
    fn handles_burst_type(&self, burst_type: &BurstType) -> bool {
        matches!(burst_type, BurstType::User)
    }
}

/// Core server burst extension
pub struct CoreServerBurstExtension {
    server_name: String,
    server_description: String,
    server_version: String,
}

impl CoreServerBurstExtension {
    pub fn new(server_name: String, server_description: String, server_version: String) -> Self {
        Self {
            server_name,
            server_description,
            server_version,
        }
    }
}

#[async_trait]
impl BurstExtension for CoreServerBurstExtension {
    async fn on_prepare_burst(&self, _target_server: &str, burst_type: &BurstType) -> Result<Vec<Message>> {
        if !matches!(burst_type, BurstType::Server) {
            return Ok(Vec::new());
        }
        
        let server_burst = Message::new(
            crate::MessageType::ServerBurst,
            vec![
                self.server_name.clone(),
                self.server_description.clone(),
                "1".to_string(), // hop count
                self.server_version.clone(),
            ]
        );
        
        Ok(vec![server_burst])
    }
    
    async fn on_receive_burst(&self, source_server: &str, burst_type: &BurstType, messages: &[Message]) -> Result<()> {
        if !matches!(burst_type, BurstType::Server) {
            return Ok(());
        }
        
        for message in messages {
            if message.params.len() >= 4 {
                let server_name = &message.params[0];
                let _description = &message.params[1];
                let hop_count = &message.params[2];
                let version = &message.params[3];
                
                tracing::info!("Received server info from {}: {} (hop: {}, version: {})", 
                    source_server, server_name, hop_count, version);
                
                // TODO: Store server information in a server registry
                // This would be useful for network topology management
            }
        }
        
        Ok(())
    }
    
    async fn on_server_connect_burst(&self, target_server: &str) -> Result<Vec<Message>> {
        self.on_prepare_burst(target_server, &BurstType::Server).await
    }
    
    async fn on_server_disconnect_cleanup(&self, source_server: &str) -> Result<()> {
        tracing::info!("Cleaning up server information for disconnected server {}", source_server);
        // TODO: Remove server from server registry
        Ok(())
    }
    
    fn get_supported_burst_types(&self) -> Vec<BurstType> {
        vec![BurstType::Server]
    }
    
    fn handles_burst_type(&self, burst_type: &BurstType) -> bool {
        matches!(burst_type, BurstType::Server)
    }
}
