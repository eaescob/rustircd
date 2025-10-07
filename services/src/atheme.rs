//! Atheme services integration
//! 
//! This module provides integration with Atheme services package,
//! implementing the Charybdis protocol for seamless communication.

use rustircd_core::{User, Message, Client, Result, Error, NumericReply, Config, ServiceDefinition};
use std::collections::HashMap;
use uuid::Uuid;
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::framework::{Service, ServiceResult, ServiceContext};

/// Atheme services integration module
pub struct AthemeIntegration {
    /// Configuration
    config: AthemeConfig,
    /// Service definitions
    services: Arc<RwLock<HashMap<String, ServiceDefinition>>>,
    /// Active connections
    connections: Arc<RwLock<HashMap<String, AthemeConnection>>>,
    /// Service statistics
    stats: Arc<RwLock<AthemeStats>>,
    /// Current connection stream (for sending messages back)
    connection_stream: Arc<RwLock<Option<TcpStream>>>,
}

/// Configuration for Atheme integration
#[derive(Debug, Clone)]
pub struct AthemeConfig {
    /// Whether Atheme integration is enabled
    pub enabled: bool,
    /// Atheme service name
    pub service_name: String,
    /// Atheme hostname
    pub hostname: String,
    /// Atheme port
    pub port: u16,
    /// Connection password
    pub password: String,
    /// Whether to use TLS
    pub tls: bool,
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
    /// Reconnection interval in seconds
    pub reconnect_interval: u64,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
}

impl Default for AthemeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_name: "services.example.org".to_string(),
            hostname: "localhost".to_string(),
            port: 6666,
            password: "password".to_string(),
            tls: false,
            timeout_seconds: 30,
            reconnect_interval: 60,
            max_reconnect_attempts: 10,
        }
    }
}

/// Atheme connection information
#[derive(Debug, Clone)]
pub struct AthemeConnection {
    /// Connection ID
    pub id: String,
    /// Service name
    pub service_name: String,
    /// Connection state
    pub state: AthemeConnectionState,
    /// Last activity time
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Reconnection attempts
    pub reconnect_attempts: u32,
}

/// Atheme connection states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AthemeConnectionState {
    /// Disconnected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Authenticating
    Authenticating,
    /// Authenticated
    Authenticated,
    /// Error
    Error,
}

/// Atheme statistics
#[derive(Debug, Clone, Default)]
pub struct AthemeStats {
    /// Total connections made
    pub total_connections: u64,
    /// Successful authentications
    pub successful_auths: u64,
    /// Failed authentications
    pub failed_auths: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Last connection time
    pub last_connection: Option<chrono::DateTime<chrono::Utc>>,
    /// Last disconnection time
    pub last_disconnection: Option<chrono::DateTime<chrono::Utc>>,
}

/// Atheme protocol handler
pub struct AthemeProtocol {
    /// Integration instance
    integration: Arc<AthemeIntegration>,
}

impl AthemeIntegration {
    /// Create a new Atheme integration
    pub fn new(config: AthemeConfig) -> Self {
        Self {
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AthemeStats::default())),
            connection_stream: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Initialize the integration
    pub async fn initialize(&self, config: &Config) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Load service definitions from config
        let mut services = self.services.write().await;
        for service in &config.services.services {
            if service.service_type == "atheme" {
                services.insert(service.name.clone(), service.clone());
            }
        }
        
        // Start connection to Atheme
        self.connect_to_atheme().await?;
        
        Ok(())
    }
    
    /// Connect to Atheme services
    pub async fn connect_to_atheme(&self) -> Result<()> {
        let connection = AthemeConnection {
            id: Uuid::new_v4().to_string(),
            service_name: self.config.service_name.clone(),
            state: AthemeConnectionState::Connecting,
            last_activity: chrono::Utc::now(),
            reconnect_attempts: 0,
        };
        
        let mut connections = self.connections.write().await;
        connections.insert(self.config.service_name.clone(), connection);
        
        // Start connection task
        let integration = Arc::new(self.clone());
        let config = self.config.clone();
        
        tokio::spawn(async move {
            if let Err(e) = integration.establish_connection(config).await {
                tracing::error!("Failed to establish Atheme connection: {}", e);
            }
        });
        
        Ok(())
    }
    
    /// Establish connection to Atheme
    async fn establish_connection(&self, config: AthemeConfig) -> Result<()> {
        let mut stream = TcpStream::connect(format!("{}:{}", config.hostname, config.port)).await
            .map_err(|e| Error::Network(format!("Failed to connect to Atheme: {}", e)))?;
        
        // Store the stream for sending messages back
        {
            let mut connection_stream = self.connection_stream.write().await;
            *connection_stream = Some(stream.try_clone().await
                .map_err(|e| Error::Network(format!("Failed to clone stream: {}", e)))?);
        }
        
        // Update connection state
        {
            let mut connections = self.connections.write().await;
            if let Some(connection) = connections.get_mut(&config.service_name) {
                connection.state = AthemeConnectionState::Connected;
                connection.last_activity = chrono::Utc::now();
            }
        }
        
        // Send SERVER command
        let server_cmd = format!("SERVER {} 1 :{}\r\n", config.service_name, "Atheme Services");
        stream.write_all(server_cmd.as_bytes()).await
            .map_err(|e| Error::Network(format!("Failed to send SERVER command: {}", e)))?;
        
        // Send PASS command
        let pass_cmd = format!("PASS {} TS 6 :{}\r\n", config.password, "Atheme");
        stream.write_all(pass_cmd.as_bytes()).await
            .map_err(|e| Error::Network(format!("Failed to send PASS command: {}", e)))?;
        
        // Update connection state
        {
            let mut connections = self.connections.write().await;
            if let Some(connection) = connections.get_mut(&config.service_name) {
                connection.state = AthemeConnectionState::Authenticated;
                connection.last_activity = chrono::Utc::now();
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_connections += 1;
            stats.last_connection = Some(chrono::Utc::now());
        }
        
        tracing::info!("Successfully connected to Atheme services");
        
        // Start message handling loop
        self.handle_messages(stream).await
    }
    
    /// Handle messages from Atheme
    async fn handle_messages(&self, mut stream: TcpStream) -> Result<()> {
        let mut buffer = [0; 4096];
        
        loop {
            let n = stream.read(&mut buffer).await
                .map_err(|e| Error::Network(format!("Failed to read from Atheme: {}", e)))?;
            
            if n == 0 {
                tracing::warn!("Atheme connection closed");
                break;
            }
            
            let data = String::from_utf8_lossy(&buffer[..n]);
            let messages = data.split("\r\n").filter(|s| !s.is_empty());
            
            for message_str in messages {
                if let Ok(message) = Message::parse(message_str) {
                    self.handle_atheme_message(&message).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle message from Atheme (legacy - use handle_atheme_message_with_context)
    async fn handle_atheme_message(&self, message: &Message) -> Result<()> {
        // This method is kept for backward compatibility but should not be used
        // Use handle_atheme_message_with_context instead
        tracing::warn!("Using legacy handle_atheme_message - consider using context-aware version");
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_received += 1;
        }
        
        // Just log the message since we don't have context
        tracing::debug!("Received Atheme message (no context): {:?}", message);
        
        Ok(())
    }
    
    /// Handle message from Atheme with service context
    async fn handle_atheme_message_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_received += 1;
        }
        
        match message.command.as_str() {
            "PING" => {
                self.handle_atheme_ping_with_context(message, context).await?;
            }
            "PONG" => {
                self.handle_atheme_pong(message).await?;
            }
            "SQUIT" => {
                self.handle_atheme_squit(message).await?;
            }
            "UID" => {
                self.handle_atheme_uid_with_context(message, context).await?;
            }
            "SJOIN" => {
                self.handle_atheme_sjoin_with_context(message, context).await?;
            }
            "SVSNICK" => {
                self.handle_atheme_svsnick_with_context(message, context).await?;
            }
            "SVSMODE" => {
                self.handle_atheme_svsmode_with_context(message, context).await?;
            }
            "SVSJOIN" => {
                self.handle_atheme_svsjoin_with_context(message, context).await?;
            }
            "SVSPART" => {
                self.handle_atheme_svspart_with_context(message, context).await?;
            }
            "SETHOST" => {
                self.handle_atheme_sethost_with_context(message, context).await?;
            }
            "SVS2MODE" => {
                self.handle_atheme_svs2mode_with_context(message, context).await?;
            }
            "NOTICE" => {
                self.handle_atheme_notice_with_context(message, context).await?;
            }
            "PRIVMSG" => {
                self.handle_atheme_privmsg_with_context(message, context).await?;
            }
            _ => {
                // Handle other Atheme messages
                tracing::debug!("Received Atheme message: {:?}", message);
            }
        }
        
        Ok(())
    }
    
    /// Send message to Atheme
    pub async fn send_message(&self, message: &Message) -> Result<()> {
        // TODO: Implement message sending to Atheme
        // This would involve finding the active connection and sending the message
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
        }
        
        Ok(())
    }
    
    /// Handle user registration with Atheme
    pub async fn handle_user_registration(&self, user: &User) -> Result<()> {
        // Send user information to Atheme
        let user_msg = format!("UID {} 1 {} {} {} {} :{}\r\n",
            user.nick,
            user.username,
            user.host,
            user.server,
            user.id,
            user.realname
        );
        
        // TODO: Send to Atheme
        tracing::debug!("Would send user registration to Atheme: {}", user_msg);
        
        Ok(())
    }
    
    /// Handle channel creation with Atheme
    pub async fn handle_channel_creation(&self, channel: &str, creator: &User) -> Result<()> {
        // Send channel information to Atheme
        let channel_msg = format!("SJOIN {} {} :{}\r\n",
            chrono::Utc::now().timestamp(),
            channel,
            format!("+{}", creator.nick)
        );
        
        // TODO: Send to Atheme
        tracing::debug!("Would send channel creation to Atheme: {}", channel_msg);
        
        Ok(())
    }
    
    /// Get connection status
    pub async fn get_connection_status(&self) -> AthemeConnectionState {
        let connections = self.connections.read().await;
        connections.get(&self.config.service_name)
            .map(|c| c.state.clone())
            .unwrap_or(AthemeConnectionState::Disconnected)
    }
    
    /// Get statistics
    pub async fn get_stats(&self) -> AthemeStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Check if Atheme is connected
    pub async fn is_connected(&self) -> bool {
        self.get_connection_status().await == AthemeConnectionState::Authenticated
    }
    
    // ============================================================================
    // Legacy Atheme Protocol Command Handlers (DEPRECATED)
    // ============================================================================
    // These handlers are kept for backward compatibility but should not be used.
    // Use the context-aware handlers instead.
    
    /// Handle PONG from Atheme (no context needed)
    async fn handle_atheme_pong(&self, message: &Message) -> Result<()> {
        tracing::debug!("Received PONG from Atheme: {:?}", message.params);
        Ok(())
    }
    
    // ============================================================================
    // Context-Aware Atheme Protocol Command Handlers
    // ============================================================================
    
    /// Handle PING from Atheme with context
    async fn handle_atheme_ping_with_context(&self, message: &Message, _context: &ServiceContext) -> Result<()> {
        let token = message.params.get(0).unwrap_or(&"".to_string());
        let pong_msg = format!("PONG :{}\r\n", token);
        
        // Send PONG response back to Atheme
        self.send_raw_message(&pong_msg).await?;
        tracing::debug!("Responded to Atheme PING with: {}", pong_msg.trim());
        Ok(())
    }
    
    /// Handle UID command from Atheme with context
    async fn handle_atheme_uid_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 6 {
            return Err(Error::MessageParse("UID command requires at least 6 parameters".to_string()));
        }
        
        let nick = &message.params[0];
        let _hopcount = &message.params[1];
        let username = &message.params[2];
        let host = &message.params[3];
        let _servertoken = &message.params[4];
        let _umodes = &message.params[5];
        let realname = message.params.get(6).map(|s| s.as_str()).unwrap_or("");
        
        tracing::info!("UID command from Atheme: {}!{}@{} ({})", nick, username, host, realname);
        
        // Create user from UID command
        let user = User::new(
            nick.clone(),
            username.clone(),
            realname.to_string(),
            host.clone(),
            self.config.service_name.clone(),
        );
        
        // Add user to database
        context.add_user(user).await?;
        
        // Broadcast UID to other servers
        let uid_message = Message::with_prefix(
            rustircd_core::Prefix::Server(self.config.service_name.clone()),
            rustircd_core::MessageType::Custom("UID".to_string()),
            message.params.clone()
        );
        context.broadcast_to_servers(uid_message).await?;
        
        Ok(())
    }
    
    /// Handle SJOIN command from Atheme with context
    async fn handle_atheme_sjoin_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 3 {
            return Err(Error::MessageParse("SJOIN command requires at least 3 parameters".to_string()));
        }
        
        let _timestamp = &message.params[0];
        let channel = &message.params[1];
        let modes = message.params.get(2).map(|s| s.as_str()).unwrap_or("");
        let members = message.params.get(3).map(|s| s.as_str()).unwrap_or("");
        
        tracing::info!("SJOIN command from Atheme: {} {} {} :{}", _timestamp, channel, modes, members);
        
        // Create channel with specified modes
        let mut channel_modes = std::collections::HashSet::new();
        if !modes.is_empty() {
            for mode in modes.chars() {
                if mode != '+' && mode != '-' {
                    channel_modes.insert(mode);
                }
            }
        }
        
        let channel_info = rustircd_core::ChannelInfo {
            name: channel.clone(),
            topic: None,
            user_count: members.split_whitespace().count(),
            modes: channel_modes,
        };
        context.add_channel(channel_info).await?;
        
        // Add members to channel
        for member in members.split_whitespace() {
            if !member.is_empty() {
                context.add_user_to_channel(member, channel).await?;
            }
        }
        
        // Broadcast SJOIN to other servers
        let sjoin_message = Message::with_prefix(
            rustircd_core::Prefix::Server(self.config.service_name.clone()),
            rustircd_core::MessageType::Custom("SJOIN".to_string()),
            message.params.clone()
        );
        context.broadcast_to_servers(sjoin_message).await?;
        
        Ok(())
    }
    
    /// Handle SVSNICK command from Atheme with context
    async fn handle_atheme_svsnick_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 3 {
            return Err(Error::MessageParse("SVSNICK command requires 3 parameters".to_string()));
        }
        
        let oldnick = &message.params[0];
        let newnick = &message.params[1];
        let _timestamp = &message.params[2];
        
        tracing::info!("SVSNICK command from Atheme: {} -> {}", oldnick, newnick);
        
        // Update user nickname in database
        if let Some(mut user) = context.get_user_by_nick(oldnick).await {
            user.nick = newnick.clone();
            context.update_user(user).await?;
        }
        
        // Broadcast SVSNICK to other servers
        let svsnick_message = Message::with_prefix(
            rustircd_core::Prefix::Server(self.config.service_name.clone()),
            rustircd_core::MessageType::Custom("SVSNICK".to_string()),
            message.params.clone()
        );
        context.broadcast_to_servers(svsnick_message).await?;
        
        Ok(())
    }
    
    /// Handle SVSMODE command from Atheme with context
    async fn handle_atheme_svsmode_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::MessageParse("SVSMODE command requires 2 parameters".to_string()));
        }
        
        let target = &message.params[0];
        let modes = &message.params[1];
        
        tracing::info!("SVSMODE command from Atheme: {} {}", target, modes);
        
        // TODO: Apply mode changes - this would need integration with the mode system
        // For now, just log and broadcast
        
        // Broadcast SVSMODE to other servers
        let svsmode_message = Message::with_prefix(
            rustircd_core::Prefix::Server(self.config.service_name.clone()),
            rustircd_core::MessageType::Custom("SVSMODE".to_string()),
            message.params.clone()
        );
        context.broadcast_to_servers(svsmode_message).await?;
        
        Ok(())
    }
    
    /// Handle SVSJOIN command from Atheme with context
    async fn handle_atheme_svsjoin_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::MessageParse("SVSJOIN command requires 2 parameters".to_string()));
        }
        
        let nick = &message.params[0];
        let channel = &message.params[1];
        
        tracing::info!("SVSJOIN command from Atheme: {} -> {}", nick, channel);
        
        // Add user to channel
        context.add_user_to_channel(nick, channel).await?;
        
        // Broadcast SVSJOIN to other servers
        let svsjoin_message = Message::with_prefix(
            rustircd_core::Prefix::Server(self.config.service_name.clone()),
            rustircd_core::MessageType::Custom("SVSJOIN".to_string()),
            message.params.clone()
        );
        context.broadcast_to_servers(svsjoin_message).await?;
        
        Ok(())
    }
    
    /// Handle SVSPART command from Atheme with context
    async fn handle_atheme_svspart_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::MessageParse("SVSPART command requires at least 2 parameters".to_string()));
        }
        
        let nick = &message.params[0];
        let channel = &message.params[1];
        let reason = message.params.get(2).map(|s| s.as_str()).unwrap_or("");
        
        tracing::info!("SVSPART command from Atheme: {} {} {}", nick, channel, reason);
        
        // Remove user from channel
        context.remove_user_from_channel(nick, channel).await?;
        
        // Broadcast SVSPART to other servers
        let svspart_message = Message::with_prefix(
            rustircd_core::Prefix::Server(self.config.service_name.clone()),
            rustircd_core::MessageType::Custom("SVSPART".to_string()),
            message.params.clone()
        );
        context.broadcast_to_servers(svspart_message).await?;
        
        Ok(())
    }
    
    /// Handle SETHOST command from Atheme with context
    async fn handle_atheme_sethost_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::MessageParse("SETHOST command requires 2 parameters".to_string()));
        }
        
        let nick = &message.params[0];
        let host = &message.params[1];
        
        tracing::info!("SETHOST command from Atheme: {} -> {}", nick, host);
        
        // Update user host in database
        if let Some(mut user) = context.get_user_by_nick(nick).await {
            user.host = host.clone();
            context.update_user(user).await?;
        }
        
        // Broadcast SETHOST to other servers
        let sethost_message = Message::with_prefix(
            rustircd_core::Prefix::Server(self.config.service_name.clone()),
            rustircd_core::MessageType::Custom("SETHOST".to_string()),
            message.params.clone()
        );
        context.broadcast_to_servers(sethost_message).await?;
        
        Ok(())
    }
    
    /// Handle SVS2MODE command from Atheme with context
    async fn handle_atheme_svs2mode_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::MessageParse("SVS2MODE command requires 2 parameters".to_string()));
        }
        
        let target = &message.params[0];
        let modes = &message.params[1];
        
        tracing::info!("SVS2MODE command from Atheme: {} {}", target, modes);
        
        // TODO: Apply mode changes (v2) - this would need integration with the mode system
        // For now, just log and broadcast
        
        // Broadcast SVS2MODE to other servers
        let svs2mode_message = Message::with_prefix(
            rustircd_core::Prefix::Server(self.config.service_name.clone()),
            rustircd_core::MessageType::Custom("SVS2MODE".to_string()),
            message.params.clone()
        );
        context.broadcast_to_servers(svs2mode_message).await?;
        
        Ok(())
    }
    
    /// Handle NOTICE from Atheme with context
    async fn handle_atheme_notice_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::MessageParse("NOTICE requires at least 2 parameters".to_string()));
        }
        
        let target = &message.params[0];
        let text = &message.params[1];
        
        tracing::debug!("NOTICE from Atheme to {}: {}", target, text);
        
        // Forward notice to target user/channel
        if target.starts_with('#') {
            context.send_to_channel(target, message.clone()).await?;
        } else {
            context.send_to_user(target, message.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Handle PRIVMSG from Atheme with context
    async fn handle_atheme_privmsg_with_context(&self, message: &Message, context: &ServiceContext) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::MessageParse("PRIVMSG requires at least 2 parameters".to_string()));
        }
        
        let target = &message.params[0];
        let text = &message.params[1];
        
        tracing::debug!("PRIVMSG from Atheme to {}: {}", target, text);
        
        // Forward message to target user/channel
        if target.starts_with('#') {
            context.send_to_channel(target, message.clone()).await?;
        } else {
            context.send_to_user(target, message.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Send raw message to Atheme
    async fn send_raw_message(&self, message: &str) -> Result<()> {
        let connection_stream = self.connection_stream.read().await;
        if let Some(ref mut stream) = *connection_stream {
            use tokio::io::AsyncWriteExt;
            stream.write_all(message.as_bytes()).await
                .map_err(|e| Error::Network(format!("Failed to send message to Atheme: {}", e)))?;
        } else {
            return Err(Error::Network("No active Atheme connection".to_string()));
        }
        Ok(())
    }
}

impl Clone for AthemeIntegration {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            services: self.services.clone(),
            connections: self.connections.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// Atheme services module
pub struct AthemeServicesModule {
    /// Atheme integration
    integration: Arc<AthemeIntegration>,
}

impl AthemeServicesModule {
    /// Create a new Atheme services module
    pub fn new(config: AthemeConfig) -> Self {
        Self {
            integration: Arc::new(AthemeIntegration::new(config)),
        }
    }
    
    /// Initialize the module
    pub async fn initialize(&self, config: &Config) -> Result<()> {
        self.integration.initialize(config).await
    }
    
    /// Handle user registration
    pub async fn handle_user_registration(&self, user: &User) -> Result<()> {
        if self.integration.is_connected().await {
            self.integration.handle_user_registration(user).await
        } else {
            Ok(())
        }
    }
    
    /// Handle channel creation
    pub async fn handle_channel_creation(&self, channel: &str, creator: &User) -> Result<()> {
        if self.integration.is_connected().await {
            self.integration.handle_channel_creation(channel, creator).await
        } else {
            Ok(())
        }
    }
    
    /// Get connection status
    pub async fn get_connection_status(&self) -> AthemeConnectionState {
        self.integration.get_connection_status().await
    }
    
    /// Get statistics
    pub async fn get_stats(&self) -> AthemeStats {
        self.integration.get_stats().await
    }
}

impl Default for AthemeServicesModule {
    fn default() -> Self {
        Self::new(AthemeConfig::default())
    }
}

#[async_trait]
impl Service for AthemeServicesModule {
    fn name(&self) -> &str {
        "atheme"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "Atheme IRC Services integration"
    }
    
    async fn init(&mut self) -> Result<()> {
        // Initialize Atheme connection
        self.integration.connect_to_atheme().await?;
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        // Cleanup Atheme connection
        tracing::info!("Cleaning up Atheme integration");
        Ok(())
    }
    
    async fn handle_message(&mut self, _client: &Client, message: &Message, _context: &ServiceContext) -> Result<ServiceResult> {
        // Atheme doesn't handle client messages directly
        Ok(ServiceResult::NotHandled)
    }
    
    async fn handle_server_message(&mut self, server: &str, message: &Message, context: &ServiceContext) -> Result<ServiceResult> {
        // Handle Atheme protocol messages
        if let MessageType::Custom(cmd) = &message.command {
            match cmd.as_str() {
                "UID" | "SJOIN" | "SVSNICK" | "SVSMODE" | "SVSJOIN" | "SVSPART" | 
                "SETHOST" | "SVS2MODE" | "NOTICE" | "PRIVMSG" | "PING" | "PONG" | "SQUIT" => {
                    // These are Atheme protocol commands
                    self.integration.handle_atheme_message_with_context(message, context).await?;
                    Ok(ServiceResult::Handled)
                }
                _ => Ok(ServiceResult::NotHandled),
            }
        } else {
            Ok(ServiceResult::NotHandled)
        }
    }
    
    async fn handle_user_registration(&mut self, user: &User, _context: &ServiceContext) -> Result<()> {
        // Notify Atheme of user registration
        self.integration.handle_user_registration(user).await
    }
    
    async fn handle_user_disconnection(&mut self, user: &User, _context: &ServiceContext) -> Result<()> {
        // Notify Atheme of user disconnection
        tracing::debug!("User {} disconnected", user.nick);
        Ok(())
    }
    
    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "server_message_handler".to_string(),
            "user_handler".to_string(),
        ]
    }
    
    fn supports_capability(&self, capability: &str) -> bool {
        matches!(capability, "server_message_handler" | "user_handler")
    }
}

/// Atheme configuration builder
pub struct AthemeConfigBuilder {
    config: AthemeConfig,
}

impl AthemeConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: AthemeConfig::default(),
        }
    }
    
    /// Set service name
    pub fn service_name(mut self, name: String) -> Self {
        self.config.service_name = name;
        self
    }
    
    /// Set hostname
    pub fn hostname(mut self, hostname: String) -> Self {
        self.config.hostname = hostname;
        self
    }
    
    /// Set port
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }
    
    /// Set password
    pub fn password(mut self, password: String) -> Self {
        self.config.password = password;
        self
    }
    
    /// Enable/disable TLS
    pub fn tls(mut self, enabled: bool) -> Self {
        self.config.tls = enabled;
        self
    }
    
    /// Set timeout
    pub fn timeout(mut self, timeout: u64) -> Self {
        self.config.timeout_seconds = timeout;
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> AthemeConfig {
        self.config
    }
}

impl Default for AthemeConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
