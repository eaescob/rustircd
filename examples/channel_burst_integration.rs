//! Example showing how to integrate ChannelBurstExtension with a real channel module
//!
//! This example demonstrates:
//! 1. How to create a channel burst extension
//! 2. How to register it with the extension manager
//! 3. How to handle channel data synchronization
//! 4. How to extend the burst system for custom data

use rustircd_core::{
    Config, Server, Result, Message, MessageType, Database, 
    extensions::{BurstExtension, BurstType}, ChannelInfo
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use async_trait::async_trait;

/// Example channel data structure
#[derive(Debug, Clone)]
pub struct ExampleChannel {
    pub id: Uuid,
    pub name: String,
    pub topic: Option<String>,
    pub topic_setter: Option<String>,
    pub topic_time: Option<DateTime<Utc>>,
    pub modes: HashSet<char>,
    pub key: Option<String>,
    pub user_limit: Option<usize>,
    pub members: HashMap<Uuid, ExampleChannelMember>,
    pub ban_masks: HashSet<String>,
    pub exception_masks: HashSet<String>,
    pub invite_masks: HashSet<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_local: bool,
}

/// Example channel member
#[derive(Debug, Clone)]
pub struct ExampleChannelMember {
    pub user_id: Uuid,
    pub modes: HashSet<char>,
}

impl ExampleChannel {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            topic: None,
            topic_setter: None,
            topic_time: None,
            modes: HashSet::new(),
            key: None,
            user_limit: None,
            members: HashMap::new(),
            ban_masks: HashSet::new(),
            exception_masks: HashSet::new(),
            invite_masks: HashSet::new(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            is_local: true,
        }
    }
    
    pub fn set_topic(&mut self, topic: String, setter: String) {
        self.topic = Some(topic);
        self.topic_setter = Some(setter);
        self.topic_time = Some(Utc::now());
        self.last_activity = Utc::now();
    }
    
    pub fn add_mode(&mut self, mode: char) {
        self.modes.insert(mode);
        self.last_activity = Utc::now();
    }
    
    pub fn remove_mode(&mut self, mode: char) {
        self.modes.remove(&mode);
        self.last_activity = Utc::now();
    }
    
    pub fn add_member(&mut self, user_id: Uuid) {
        self.members.insert(user_id, ExampleChannelMember {
            user_id,
            modes: HashSet::new(),
        });
        self.last_activity = Utc::now();
    }
    
    pub fn remove_member(&mut self, user_id: &Uuid) {
        self.members.remove(user_id);
        self.last_activity = Utc::now();
    }
}

/// Example channel burst extension implementation
pub struct ExampleChannelBurstExtension {
    channels: Arc<RwLock<HashMap<String, ExampleChannel>>>,
    database: Arc<Database>,
    server_name: String,
}

impl ExampleChannelBurstExtension {
    pub fn new(channels: Arc<RwLock<HashMap<String, ExampleChannel>>>, database: Arc<Database>, server_name: String) -> Self {
        Self {
            channels,
            database,
            server_name,
        }
    }
}

#[async_trait]
impl BurstExtension for ExampleChannelBurstExtension {
    async fn on_prepare_burst(&self, _target_server: &str, burst_type: &BurstType) -> Result<Vec<Message>> {
        if !matches!(burst_type, BurstType::Channel) {
            return Ok(Vec::new());
        }
        
        let mut messages = Vec::new();
        let channels = self.channels.read().await;
        
        // Send all local channels
        for channel in channels.values() {
            if !channel.is_local {
                continue; // Skip remote channels
            }
            
            // Create channel burst message
            let mut params = vec![
                channel.name.clone(),
                channel.id.to_string(),
                self.server_name.clone(),
                channel.created_at.to_rfc3339(),
            ];
            
            // Add topic information
            if let Some(topic) = &channel.topic {
                params.push("TOPIC".to_string());
                params.push(topic.clone());
                if let Some(setter) = &channel.topic_setter {
                    params.push(setter.clone());
                } else {
                    params.push("unknown".to_string());
                }
                if let Some(time) = channel.topic_time {
                    params.push(time.to_rfc3339());
                } else {
                    params.push(Utc::now().to_rfc3339());
                }
            } else {
                params.push("NOTOPIC".to_string());
            }
            
            // Add modes
            let modes_str = if channel.modes.is_empty() {
                "".to_string()
            } else {
                format!("+{}", channel.modes.iter().collect::<String>())
            };
            params.push(modes_str);
            
            // Add channel key if present
            if let Some(key) = &channel.key {
                params.push("KEY".to_string());
                params.push(key.clone());
            }
            
            // Add user limit if present
            if let Some(limit) = channel.user_limit {
                params.push("LIMIT".to_string());
                params.push(limit.to_string());
            }
            
            // Add ban masks
            if !channel.ban_masks.is_empty() {
                params.push("BANMASKS".to_string());
                params.push(channel.ban_masks.iter().cloned().collect::<Vec<_>>().join(","));
            }
            
            // Add exception masks
            if !channel.exception_masks.is_empty() {
                params.push("EXCEPTMASKS".to_string());
                params.push(channel.exception_masks.iter().cloned().collect::<Vec<_>>().join(","));
            }
            
            // Add invite masks
            if !channel.invite_masks.is_empty() {
                params.push("INVITEMASKS".to_string());
                params.push(channel.invite_masks.iter().cloned().collect::<Vec<_>>().join(","));
            }
            
            // Add member count
            params.push("MEMBERS".to_string());
            params.push(channel.members.len().to_string());
            
            let channel_burst = Message::new(MessageType::ChannelBurst, params);
            messages.push(channel_burst);
        }
        
        Ok(messages)
    }
    
    async fn on_receive_burst(&self, source_server: &str, burst_type: &BurstType, messages: &[Message]) -> Result<()> {
        if !matches!(burst_type, BurstType::Channel) {
            return Ok(());
        }
        
        let mut channels = self.channels.write().await;
        
        for message in messages {
            if message.params.len() < 4 {
                continue; // Skip malformed messages
            }
            
            let channel_name = &message.params[0];
            let channel_id_str = &message.params[1];
            let channel_server = &message.params[2];
            let created_at_str = &message.params[3];
            
            // Parse channel ID
            let channel_id = match Uuid::parse_str(channel_id_str) {
                Ok(id) => id,
                Err(_) => continue, // Skip malformed messages
            };
            
            // Parse creation time
            let created_at = match DateTime::parse_from_rfc3339(created_at_str) {
                Ok(time) => time.with_timezone(&Utc),
                Err(_) => Utc::now(), // Default to now if parsing fails
            };
            
            // Create remote channel
            let mut remote_channel = ExampleChannel {
                id: channel_id,
                name: channel_name.clone(),
                topic: None,
                topic_setter: None,
                topic_time: None,
                modes: HashSet::new(),
                key: None,
                user_limit: None,
                members: HashMap::new(),
                ban_masks: HashSet::new(),
                exception_masks: HashSet::new(),
                invite_masks: HashSet::new(),
                created_at,
                last_activity: Utc::now(),
                is_local: false, // This is a remote channel
            };
            
            // Parse additional parameters
            let mut i = 4;
            while i < message.params.len() {
                match message.params[i].as_str() {
                    "TOPIC" => {
                        if i + 3 < message.params.len() {
                            remote_channel.topic = Some(message.params[i + 1].clone());
                            remote_channel.topic_setter = Some(message.params[i + 2].clone());
                            if let Ok(time) = DateTime::parse_from_rfc3339(&message.params[i + 3]) {
                                remote_channel.topic_time = Some(time.with_timezone(&Utc));
                            }
                            i += 4;
                        } else {
                            i += 1;
                        }
                    }
                    "NOTOPIC" => {
                        remote_channel.topic = None;
                        i += 1;
                    }
                    "+" if i + 1 < message.params.len() => {
                        // Parse modes
                        for mode_char in message.params[i + 1].chars() {
                            remote_channel.modes.insert(mode_char);
                        }
                        i += 2;
                    }
                    "KEY" if i + 1 < message.params.len() => {
                        remote_channel.key = Some(message.params[i + 1].clone());
                        i += 2;
                    }
                    "LIMIT" if i + 1 < message.params.len() => {
                        if let Ok(limit) = message.params[i + 1].parse::<usize>() {
                            remote_channel.user_limit = Some(limit);
                        }
                        i += 2;
                    }
                    "BANMASKS" if i + 1 < message.params.len() => {
                        let masks = message.params[i + 1].split(',').map(|s| s.to_string()).collect();
                        remote_channel.ban_masks = masks;
                        i += 2;
                    }
                    "EXCEPTMASKS" if i + 1 < message.params.len() => {
                        let masks = message.params[i + 1].split(',').map(|s| s.to_string()).collect();
                        remote_channel.exception_masks = masks;
                        i += 2;
                    }
                    "INVITEMASKS" if i + 1 < message.params.len() => {
                        let masks = message.params[i + 1].split(',').map(|s| s.to_string()).collect();
                        remote_channel.invite_masks = masks;
                        i += 2;
                    }
                    "MEMBERS" if i + 1 < message.params.len() => {
                        // Note: We don't sync member lists in channel bursts
                        // Member synchronization is handled separately via user bursts
                        i += 2;
                    }
                    _ => {
                        i += 1; // Skip unknown parameters
                    }
                }
            }
            
            // Add remote channel to our channel list
            channels.insert(channel_name.clone(), remote_channel);
            
            // Also add to database for tracking
            if let Some(channel_info) = self.database.get_channel(channel_name) {
                // Update existing channel info
                // Note: We don't update the database channel info with remote data
                // as it might conflict with local state
            } else {
                // Add new channel info to database
                let channel_info = ChannelInfo {
                    name: channel_name.clone(),
                    topic: remote_channel.topic.clone(),
                    member_count: 0, // Will be updated when members join
                    created_at: remote_channel.created_at,
                };
                if let Err(e) = self.database.add_channel(channel_info) {
                    tracing::warn!("Failed to add remote channel {} to database: {}", channel_name, e);
                }
            }
        }
        
        Ok(())
    }
    
    async fn on_server_connect_burst(&self, target_server: &str) -> Result<Vec<Message>> {
        // Send all local channels when a server connects
        self.on_prepare_burst(target_server, &BurstType::Channel).await
    }
    
    async fn on_server_disconnect_cleanup(&self, source_server: &str) -> Result<()> {
        // Remove all channels from the disconnected server
        let mut channels = self.channels.write().await;
        channels.retain(|_name, channel| channel.is_local);
        
        tracing::info!("Cleaned up channels from disconnected server: {}", source_server);
        Ok(())
    }
    
    fn get_supported_burst_types(&self) -> Vec<BurstType> {
        vec![BurstType::Channel]
    }
    
    fn handles_burst_type(&self, burst_type: &BurstType) -> bool {
        matches!(burst_type, BurstType::Channel)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCD Channel Burst Integration Example");
    println!("==========================================");
    
    // Create server configuration
    let config = create_server_config();
    
    // Create and initialize the server
    let mut server = Server::new(config);
    server.init().await?;
    
    // Create example channel data
    let channels = Arc::new(RwLock::new(HashMap::new()));
    let database = server.database();
    
    // Add some example channels
    let mut channels_guard = channels.write().await;
    let mut general_channel = ExampleChannel::new("#general".to_string());
    general_channel.set_topic("Welcome to #general!".to_string(), "admin!admin@localhost".to_string());
    general_channel.add_mode('n');
    general_channel.add_mode('t');
    general_channel.add_member(Uuid::new_v4());
    general_channel.add_member(Uuid::new_v4());
    channels_guard.insert("#general".to_string(), general_channel);
    
    let mut private_channel = ExampleChannel::new("#private".to_string());
    private_channel.add_mode('i');
    private_channel.add_mode('k');
    private_channel.key = Some("secret123".to_string());
    private_channel.user_limit = Some(10);
    private_channel.add_member(Uuid::new_v4());
    channels_guard.insert("#private".to_string(), private_channel);
    drop(channels_guard);
    
    println!("Created example channels:");
    println!("  - #general (with topic and modes)");
    println!("  - #private (with key and limit)");
    println!();
    
    // Create and register the channel burst extension
    let channel_burst_extension = Box::new(ExampleChannelBurstExtension::new(
        channels.clone(),
        database,
        server.config().server.name.clone(),
    ));
    
    // Register with extension manager
    if let Err(e) = server.extension_manager().register_burst_extension(channel_burst_extension).await {
        println!("❌ Failed to register channel burst extension: {}", e);
        return Err(e);
    }
    
    println!("✅ Channel burst extension registered successfully");
    println!();
    
    // Demonstrate channel burst functionality
    demonstrate_integration(&server, &channels).await?;
    
    Ok(())
}

/// Create server configuration
fn create_server_config() -> Config {
    let mut config = Config::default();
    config.server.name = "integration.example.com".to_string();
    config.server.description = "Channel Burst Integration Test".to_string();
    config.modules.enabled_modules = vec!["channel".to_string()];
    config
}

/// Demonstrate the integration
async fn demonstrate_integration(
    server: &Server, 
    channels: &Arc<RwLock<HashMap<String, ExampleChannel>>>
) -> Result<()> {
    println!("Demonstrating Channel Burst Integration:");
    println!("=======================================");
    
    // Show current channels
    let channels_guard = channels.read().await;
    println!();
    println!("Current local channels:");
    for (name, channel) in channels_guard.iter() {
        if channel.is_local {
            println!("  {} - {} members, modes: +{}", 
                name, 
                channel.members.len(),
                channel.modes.iter().collect::<String>()
            );
            if let Some(topic) = &channel.topic {
                println!("    Topic: {}", topic);
            }
        }
    }
    drop(channels_guard);
    
    // Prepare channel burst
    println!();
    println!("Preparing channel burst for remote server...");
    match server.prepare_channel_burst("remote.example.com").await {
        Ok(messages) => {
            println!("✅ Prepared {} channel burst messages:", messages.len());
            for (i, message) in messages.iter().enumerate() {
                println!("  Message {}: {}", i + 1, message);
            }
        }
        Err(e) => {
            println!("❌ Failed to prepare channel burst: {}", e);
        }
    }
    
    // Simulate receiving channel burst
    println!();
    println!("Simulating channel burst reception...");
    let sample_burst = create_sample_burst();
    match server.handle_channel_burst("remote.example.com", &sample_burst).await {
        Ok(()) => {
            println!("✅ Successfully processed remote channel burst");
            
            // Show updated channels
            let channels_guard = channels.read().await;
            println!();
            println!("Updated channel list:");
            for (name, channel) in channels_guard.iter() {
                let source = if channel.is_local { "local" } else { "remote" };
                println!("  {} - {} members, modes: +{}, source: {}", 
                    name, 
                    channel.members.len(),
                    channel.modes.iter().collect::<String>(),
                    source
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to process channel burst: {}", e);
        }
    }
    
    Ok(())
}

/// Create sample burst for testing
fn create_sample_burst() -> Vec<Message> {
    vec![
        Message::new(
            MessageType::ChannelBurst,
            vec![
                "#remote".to_string(),
                Uuid::new_v4().to_string(),
                "remote.example.com".to_string(),
                Utc::now().to_rfc3339(),
                "TOPIC".to_string(),
                "Remote channel topic".to_string(),
                "remoteadmin!admin@remote.example.com".to_string(),
                Utc::now().to_rfc3339(),
                "+mn".to_string(),
                "MEMBERS".to_string(),
                "3".to_string(),
            ]
        )
    ]
}
