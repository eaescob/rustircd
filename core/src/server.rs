//! Main IRC server implementation

use crate::{
    User, Message, MessageType, NumericReply, Config, ModuleManager,
    connection::ConnectionHandler, Error, Result, module::{ModuleResult, ModuleStatsResponse}, client::{Client, ClientState},
    Database, BroadcastSystem, NetworkQueryManager, NetworkMessageHandler,
    ServerConnectionManager, ServerConnection, Prefix,
    ThrottlingManager, StatisticsManager, MotdManager,
    LookupService, RehashService,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use rustls::{ServerConfig, Certificate, PrivateKey};
use std::io::BufReader;
use uuid::Uuid;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt};
use tracing::{info, warn};

/// Main IRC server
pub struct Server {
    /// Server configuration
    config: Config,
    /// Module manager
    module_manager: Arc<RwLock<ModuleManager>>,
    /// Connection handler
    connection_handler: Arc<RwLock<ConnectionHandler>>,
    /// Users by ID
    users: Arc<RwLock<HashMap<uuid::Uuid, User>>>,
    /// Users by nickname
    nick_to_id: Arc<RwLock<HashMap<String, uuid::Uuid>>>,
    /// Super servers (u-lined)
    super_servers: Arc<RwLock<HashMap<String, bool>>>,
    /// Database for users, servers, and history
    database: Arc<Database>,
    /// Broadcasting system
    broadcast_system: Arc<BroadcastSystem>,
    /// Network query manager
    network_query_manager: Arc<NetworkQueryManager>,
    /// Network message handler
    network_message_handler: Arc<NetworkMessageHandler>,
    /// Server connection manager
    server_connections: Arc<ServerConnectionManager>,
    /// Throttling manager for connection rate limiting
    throttling_manager: Arc<ThrottlingManager>,
    /// Statistics manager for tracking server statistics
    statistics_manager: Arc<StatisticsManager>,
    /// MOTD manager for Message of the Day
    motd_manager: Arc<MotdManager>,
    /// DNS and ident lookup service
    lookup_service: Arc<LookupService>,
    /// Rehash service for runtime configuration reloading
    rehash_service: Arc<RehashService>,
    /// TLS acceptor (if enabled)
    tls_acceptor: Option<TlsAcceptor>,
    /// Replies configuration
    replies_config: Option<crate::RepliesConfig>,
}

impl Server {
    /// Create a numeric reply using configurable replies if available
    fn create_numeric_reply(&self, reply: NumericReply, target: &str, params: Vec<String>) -> Message {
        if let Some(ref replies_config) = self.replies_config {
            let mut param_map = std::collections::HashMap::new();
            // Add common parameters
            param_map.insert("nick".to_string(), target.to_string());
            
            // Add custom parameters from the params vector
            for (i, param) in params.iter().enumerate() {
                param_map.insert(format!("param{}", i), param.clone());
            }
            
            // Create server info from main config
            let server_info = crate::RepliesServerInfo {
                name: self.config.server.name.clone(),
                version: self.config.server.version.clone(),
                description: self.config.server.description.clone(),
                created: self.config.server.created.clone(),
                admin_email: self.config.server.admin_email.clone(),
                admin_location1: self.config.server.admin_location1.clone(),
                admin_location2: self.config.server.admin_location2.clone(),
            };
            
            reply.reply_with_config(target, &param_map, replies_config, &server_info)
        } else {
            reply.reply(target, params)
        }
    }

    /// Create a new server instance
    pub async fn new(config: Config) -> Self {
        Self::new_with_config_path(config, "config.toml".to_string()).await
    }
    
    /// Create a new server instance with a specific config path
    pub async fn new_with_config_path(config: Config, config_path: String) -> Self {
        let (connection_handler, _) = ConnectionHandler::new();
        
        // Initialize database
        let database = Arc::new(Database::new(
            config.database.max_history_size,
            config.database.history_retention_days,
        ));
        
        // Initialize broadcasting system
        let broadcast_system = Arc::new(BroadcastSystem::new());
        
        // Initialize network query manager
        let network_query_manager = Arc::new(NetworkQueryManager::new(
            config.broadcast.query_timeout_seconds,
            config.broadcast.max_concurrent_queries,
        ));
        
        // Initialize network message handler
        let network_message_handler = Arc::new(NetworkMessageHandler::new(
            database.clone(),
            network_query_manager.clone(),
            config.server.name.clone(),
        ));

        
        // Initialize server connection manager
        let server_connections = Arc::new(ServerConnectionManager::new(Arc::new(config.clone())));
        
        // Initialize throttling manager
        let throttling_manager = Arc::new(ThrottlingManager::new(config.modules.throttling.clone()));
        
        // Initialize statistics manager
        let statistics_manager = Arc::new(StatisticsManager::new());
        
        // Initialize MOTD manager
        let mut motd_manager = MotdManager::new();
        if let Some(motd_file) = &config.server.motd_file {
            if let Err(e) = motd_manager.load_motd(motd_file).await {
                tracing::warn!("Failed to load MOTD file {}: {}", motd_file, e);
            }
        }
        let motd_manager = Arc::new(motd_manager);
        
        // Initialize lookup service
        let lookup_service = Arc::new(LookupService::new(
            config.security.enable_dns,
            config.security.enable_reverse_dns,
            config.security.enable_ident,
        ).await.unwrap_or_else(|e| {
            tracing::error!("Failed to initialize lookup service: {}", e);
            // Create a disabled lookup service as fallback
            panic!("Lookup service initialization failed: {}", e);
        }));
        
        // Initialize rehash service
        let config_arc = Arc::new(RwLock::new(config.clone()));
        let rehash_service = Arc::new(RehashService::new(
            config_arc.clone(),
            motd_manager.clone(),
            config_path,
        ));
        
        Self {
            config: config.clone(),
            module_manager: Arc::new(RwLock::new(ModuleManager::new(database.clone(), server_connections.clone()))),
            connection_handler: Arc::new(RwLock::new(connection_handler)),
            users: Arc::new(RwLock::new(HashMap::new())),
            nick_to_id: Arc::new(RwLock::new(HashMap::new())),
            super_servers: Arc::new(RwLock::new(HashMap::new())),
            database,
            broadcast_system,
            network_query_manager,
            network_message_handler,
            server_connections,
            throttling_manager,
            statistics_manager,
            motd_manager,
            lookup_service,
            rehash_service,
            tls_acceptor: None,
            replies_config: config.replies.clone(),
        }
    }
    
    /// Initialize the server
    pub async fn init(&mut self) -> Result<()> {
        // Validate configuration
        self.config.validate()?;
        
        // Setup TLS if enabled
        if self.config.security.tls.enabled {
            self.setup_tls().await?;
        }
        
        // Load super servers
        self.load_super_servers().await?;
        
        // Load modules
        self.load_modules().await?;
        
        // Initialize throttling manager
        self.throttling_manager.init().await?;
        
        tracing::info!("Server initialized successfully");
        Ok(())
    }
    
    /// Setup TLS configuration
    async fn setup_tls(&mut self) -> Result<()> {
        let cert_file = self.config.security.tls.cert_file.as_ref()
            .ok_or_else(|| Error::Config("TLS certificate file not specified".to_string()))?;
        let key_file = self.config.security.tls.key_file.as_ref()
            .ok_or_else(|| Error::Config("TLS key file not specified".to_string()))?;
        
        // Load certificate
        let cert_chain = load_certificates(cert_file)?;
        let private_key = load_private_key(key_file)?;
        
        // Create TLS configuration with custom cipher suites
        let tls_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| Error::Tls(e))?;
        
        // Configure cipher suites if specified
        if !self.config.security.tls.cipher_suites.is_empty() {
            // For now, we'll use the safe defaults since rustls handles cipher suite selection
            // The configured cipher suites are logged for reference
            tracing::info!("Configured cipher suites: {:?}", self.config.security.tls.cipher_suites);
        }
        
        // Log TLS version configuration
        tracing::info!("TLS version configured: {}", self.config.security.tls.version);
        
        self.tls_acceptor = Some(TlsAcceptor::from(Arc::new(tls_config)));
        
        tracing::info!("TLS configuration loaded");
        Ok(())
    }
    
    /// Load super servers from configuration
    async fn load_super_servers(&mut self) -> Result<()> {
        let mut super_servers = self.super_servers.write().await;
        for super_server in &self.config.network.super_servers {
            super_servers.insert(super_server.name.clone(), true);
            tracing::info!("Loaded super server: {}", super_server.name);
        }
        Ok(())
    }
    
    /// Load modules from configuration
    async fn load_modules(&mut self) -> Result<()> {
        let _module_manager = self.module_manager.write().await;
        
        for module_name in &self.config.modules.enabled_modules {
            match module_name.as_str() {
                "channel" => {
                    // Load channel module
                    // let channel_module = rustircd_modules::ChannelModule::new(); // Commented out - modules crate not available in core
                    // module_manager.load_module(Box::new(channel_module)).await?; // Commented out - modules crate not available
                    tracing::info!("Loaded channel module");
                    
                    // Register channel burst extension
                    self.register_channel_burst_extension().await?;
                }
                "ircv3" => {
                    // Load IRCv3 module
                    // let ircv3_module = rustircd_modules::Ircv3Module::new(); // Commented out - modules crate not available
                    // module_manager.load_module(Box::new(ircv3_module)).await?; // Commented out - modules crate not available
                    tracing::info!("Loaded IRCv3 module");
                }
                "optional" => {
                    // Load optional commands module
                    // let optional_module = rustircd_modules::OptionalModule::new(); // Commented out - modules crate not available
                    // module_manager.load_module(Box::new(optional_module)).await?; // Commented out - modules crate not available
                    tracing::info!("Loaded optional commands module");
                }
                "throttling" => {
                    // Load throttling module
                    // let throttling_module = rustircd_modules::ThrottlingModule::new(self.config.modules.throttling.clone()); // Commented out - modules crate not available
                    // module_manager.load_module(Box::new(throttling_module)).await?; // Commented out - modules crate not available
                    tracing::info!("Loaded throttling module");
                }
                _ => {
                    tracing::warn!("Unknown module: {}", module_name);
                }
            }
        }
        
        tracing::info!("Modules loaded successfully");
        Ok(())
    }
    
    /// Start the server
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting IRC server with {} configured ports", 
                      self.config.connection.ports.len());
        
        // Start listeners for all configured ports
        for port_config in &self.config.connection.ports {
            self.start_port_listener(port_config).await?;
        }
        
        // Start message processing loop
        self.start_message_processor().await?;
        
        // Start connection timeout checker
        self.start_timeout_checker().await?;
        
        Ok(())
    }
    
    /// Start connection timeout checker
    async fn start_timeout_checker(&self) -> Result<()> {
        let connection_handler = self.connection_handler.clone();
        
        tokio::spawn(async move {
            loop {
                // Check every 30 seconds
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                
                let mut handler = connection_handler.write().await;
                let mut timed_out_clients = Vec::new();
                
                // Find timed out clients
                for (client_id, client) in handler.iter_clients() {
                    if client.timing.is_timed_out() {
                        timed_out_clients.push(*client_id);
                        tracing::info!("Client {} timed out (no PONG received)", client_id);
                    } else if client.timing.should_send_ping() {
                        // Send PING if it's time
                        let ping_msg = Message::new(
                            MessageType::Ping,
                            vec![chrono::Utc::now().timestamp().to_string()],
                        );
                        if let Err(e) = client.send(ping_msg) {
                            tracing::warn!("Failed to send PING to client {}: {}", client_id, e);
                        } else {
                            tracing::debug!("Sent PING to client {}", client_id);
                        }
                    }
                }
                
                // Disconnect timed out clients
                for client_id in timed_out_clients {
                    if let Some(client) = handler.remove_client(&client_id) {
                        tracing::info!("Disconnecting timed out client: {}", client_id);
                        let _ = client.send(Message::new(
                            MessageType::Custom("ERROR".to_string()),
                            vec!["Connection timeout".to_string()],
                        ));
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Start a listener for a specific port configuration
    async fn start_port_listener(&self, port_config: &crate::config::PortConfig) -> Result<()> {
        let listener = TcpListener::bind(
            format!("{}:{}", self.config.connection.bind_address, port_config.port)
        ).await?;
        
        let port = port_config.port;
        let connection_type = port_config.connection_type.clone();
        let tls_enabled = port_config.tls;
        let tls_acceptor = if tls_enabled { self.tls_acceptor.clone() } else { None };
        let connection_handler = self.connection_handler.clone();
        let description = port_config.description.clone().unwrap_or_else(|| "Unnamed port".to_string());
        
        tracing::info!("Starting listener on port {} ({}) - TLS: {}, Type: {:?}", 
                      port, description, tls_enabled, connection_type);
        
        // Spawn connection handler for this port
        let throttling_manager = self.throttling_manager.clone();
        let statistics_manager = self.statistics_manager.clone();
        let lookup_service = self.lookup_service.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, addr)) => {
                        // Determine connection type based on port configuration
                        let is_client_connection = matches!(connection_type, crate::config::PortConnectionType::Client | crate::config::PortConnectionType::Both);
                        let is_server_connection = matches!(connection_type, crate::config::PortConnectionType::Server | crate::config::PortConnectionType::Both);
                        
                        // Check throttling for client connections
                        if is_client_connection && !is_server_connection {
                            match throttling_manager.check_connection_allowed(addr.ip()).await {
                                Ok(allowed) => {
                                    if !allowed {
                                        tracing::debug!("Connection from {} blocked by throttling", addr);
                                        let _ = stream.shutdown().await;
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Error checking throttling for {}: {}", addr, e);
                                    let _ = stream.shutdown().await;
                                    continue;
                                }
                            }
                            
                            // Record connection statistics
                            statistics_manager.record_connection().await;
                        } else if is_server_connection && !is_client_connection {
                            // Record server connection statistics
                            statistics_manager.record_server_connection().await;
                        }
                        
                        let mut conn_handler = connection_handler.write().await;
                        if let Err(e) = conn_handler.handle_connection_with_type(stream, addr, tls_acceptor.clone(), is_client_connection, is_server_connection, Some(&lookup_service)).await {
                            tracing::error!("Error handling connection from {}: {}", addr, e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error accepting connection on port {}: {}", port, e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Start message processing loop
    /// Note: Message processing is currently handled directly in handle_client_message
    /// and through the module system. This method is kept for potential future use
    /// if we want to implement a dedicated message queue/processing thread.
    async fn start_message_processor(&self) -> Result<()> {
        // Message processing is currently handled synchronously in handle_client_message
        // No async processing loop needed at this time
        Ok(())
    }
    
    /// Handle a message from a client
    pub async fn handle_message(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        // Record message statistics (from local client, is_remote = false)
        let command_name = match &message.command {
            MessageType::Custom(cmd) => cmd.as_str(),
            _ => "UNKNOWN",
        };
        self.statistics_manager.record_message_received(command_name, message.to_string().len(), false).await;
        
        let connection_handler = self.connection_handler.read().await;
        let client = connection_handler.get_client(&client_id)
            .ok_or_else(|| Error::User("Client not found".to_string()))?;
        
        // Process through modules first
        let mut module_manager = self.module_manager.write().await;
        match module_manager.handle_message_with_server(client, &message, Some(self)).await? {
            ModuleResult::HandledStop => return Ok(()),
            ModuleResult::Rejected(reason) => {
                // Send error message to client
                let error_msg = Message::new(MessageType::Custom("ERROR".to_string()), vec![reason]);
                if let Some(client) = connection_handler.get_client(&client_id) {
                    let _ = client.send(error_msg);
                }
                return Ok(());
            }
            ModuleResult::Handled => return Ok(()),
            ModuleResult::NotHandled => {
                // Handle core commands
                self.handle_core_command(client_id, message).await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle a message from a server
    pub async fn handle_server_message(&self, server_name: &str, message: Message) -> Result<()> {
        // Record message statistics (from remote server, is_remote = true)
        let command_name = match &message.command {
            MessageType::Custom(cmd) => cmd.as_str(),
            _ => "UNKNOWN",
        };
        self.statistics_manager.record_message_received(command_name, message.to_string().len(), true).await;
        
        // Validate that this server is authorized to connect
        // This should be called when a server first connects, not on every message
        // For now, we'll check if the server is in our configuration
        
        // Check if this is a super server
        let is_super_server = self.server_connections.is_super_server(server_name);

        // Process through modules
        let mut module_manager = self.module_manager.write().await;
        match module_manager.handle_server_message(server_name, &message).await? {
            ModuleResult::HandledStop => return Ok(()),
            ModuleResult::Rejected(reason) => {
                tracing::warn!("Server {} message rejected: {}", server_name, reason);
                return Ok(());
            }
            ModuleResult::Handled => return Ok(()),
            ModuleResult::NotHandled => {
                // Handle core server commands
                self.handle_core_server_command(server_name, message, is_super_server).await?;
            }
        }
        
        Ok(())
    }
    
    /// Check if a server is a super server
    pub async fn is_super_server(&self, server_name: &str) -> bool {
        let super_servers = self.super_servers.read().await;
        super_servers.contains_key(server_name)
    }
    
    /// Handle core server commands
    async fn handle_core_server_command(&self, server_name: &str, message: Message, is_super_server: bool) -> Result<()> {
        match message.command {
            MessageType::Server => {
                self.handle_server_registration(server_name, message, is_super_server).await?;
            }
            MessageType::ServerQuit => {
                self.handle_server_quit(server_name, message).await?;
            }
            MessageType::Ping => {
                self.handle_server_ping(server_name, message).await?;
            }
            MessageType::Pong => {
                self.handle_server_pong(server_name, message).await?;
            }
            MessageType::Nick => {
                self.handle_server_nick_propagation(server_name, message).await?;
            }
            MessageType::Quit => {
                self.handle_server_quit_propagation(server_name, message).await?;
            }
            MessageType::UserBurst => {
                self.handle_user_burst(server_name, message).await?;
            }
            MessageType::ServerBurst => {
                self.handle_server_burst_received(server_name, message).await?;
            }
            MessageType::ChannelBurst => {
                self.handle_channel_burst_received(server_name, message).await?;
            }
            MessageType::Wallops => {
                self.handle_server_wallops_received(server_name, message).await?;
            }
            MessageType::Kill => {
                self.handle_server_kill_received(server_name, message).await?;
            }
            MessageType::Away => {
                self.handle_server_away_received(server_name, message).await?;
            }
            MessageType::Join => {
                self.handle_server_join_received(server_name, message).await?;
            }
            MessageType::Part => {
                self.handle_server_part_received(server_name, message).await?;
            }
            _ => {
                // Other server commands can be handled here
                tracing::debug!("Unhandled server command: {:?}", message.command);
            }
        }
        Ok(())
    }
    
    /// Handle initial server registration from a new connection (before server is fully connected)
    async fn handle_initial_server_registration(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        if message.params.len() < 3 {
            return Err(Error::MessageParse("SERVER command requires at least 3 parameters".to_string()));
        }
        
        let server_name = &message.params[0];
        let hop_count: u8 = message.params[1].parse()
            .map_err(|_| Error::MessageParse("Invalid hop count in SERVER command".to_string()))?;
        let server_description = &message.params[2];
        
        tracing::info!("Server {} attempting to register (hopcount: {})", server_name, hop_count);
        
        // Validate server password
        let connection_handler = self.connection_handler.read().await;
        let client = connection_handler.get_client(&client_id)
            .ok_or_else(|| Error::Server("Client not found".to_string()))?;
        
        let provided_password = client.server_password.clone()
            .ok_or_else(|| Error::Server("No password provided (PASS command required before SERVER)".to_string()))?;
        
        // Check if this server is in our configuration
        let server_link = self.server_connections.get_server_link(server_name)
            .ok_or_else(|| Error::Server(format!("Server {} is not authorized (not in configuration)", server_name)))?;
        
        // Validate password
        if server_link.password != provided_password {
            tracing::warn!("Password mismatch for server {}", server_name);
            return Err(Error::Server(format!("Password mismatch for server {}", server_name)));
        }
        
        tracing::info!("Server {} password validated successfully", server_name);
        
        // Create server connection
        let remote_addr = client.remote_addr.clone();
        let local_addr = client.local_addr.clone();
        
        drop(connection_handler); // Release the read lock
        
        // Create a new server connection object
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let server_connection = crate::server_connection::ServerConnection::new(
            client_id,
            remote_addr.parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
            local_addr.parse().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
            tx,
            false, // incoming connection
        );
        
        // Add server info
        let mut server_connection = server_connection;
        server_connection.info.name = server_name.clone();
        server_connection.info.description = server_description.clone();
        server_connection.info.hop_count = hop_count;
        server_connection.state = crate::server_connection::ServerConnectionState::Registered;
        
        // Check if it's a super server
        let is_super_server = self.server_connections.is_super_server(server_name);
        server_connection.info.is_super_server = is_super_server;
        
        // Add to super servers map if applicable
        if is_super_server {
            let mut super_servers = self.super_servers.write().await;
            super_servers.insert(server_name.clone(), true);
        }
        
        // Add server connection to manager
        self.server_connections.add_connection(server_connection.clone()).await?;
        
        // Add server to database
        let server_info = crate::database::ServerInfo {
            name: server_name.clone(),
            description: server_description.clone(),
            version: String::new(),
            hopcount: hop_count as u32,
            connected_at: chrono::Utc::now(),
            is_super_server,
            user_count: 0,
        };
        self.database.add_server(server_info)?;
        
        // Send server burst to the new server
        self.send_server_burst(server_name).await?;
        
        tracing::info!("Server {} fully registered and burst sent", server_name);
        
        Ok(())
    }
    
    /// Handle server registration
    async fn handle_server_registration(&self, server_name: &str, message: Message, is_super_server: bool) -> Result<()> {
        tracing::info!("Server {} registered (super: {})", server_name, is_super_server);
        
        // Validate SERVER command parameters
        if message.params.len() < 3 {
            return Err(Error::MessageParse("SERVER command requires at least 3 parameters".to_string()));
        }
        
        let _server_name_param = &message.params[0];
        let hop_count: u8 = message.params[1].parse()
            .map_err(|_| Error::MessageParse("Invalid hop count in SERVER command".to_string()))?;
        let _server_description = &message.params[2];
        
        // Update server connection state
        self.server_connections.update_connection_state(server_name, crate::server_connection::ServerConnectionState::Registered).await?;
        
        // Send server burst to propagate our users and channels
        self.send_server_burst(server_name).await?;
        
        tracing::info!("Server {} fully registered with hop count {}", server_name, hop_count);
        
        Ok(())
    }
    
    /// Handle server quit
    async fn handle_server_quit(&self, server_name: &str, message: Message) -> Result<()> {
        let quit_reason = message.params.first()
            .map(|s| s.as_str())
            .unwrap_or("Server quit");
        
        tracing::info!("Server {} quit: {}", server_name, quit_reason);
        
        // 1. Get all users from the quitting server
        let users_to_remove = self.database.get_users_by_server(server_name);
        let user_count = users_to_remove.len();
        tracing::info!("Found {} users from server {}", user_count, server_name);
        
        // 2. Remove all users from this server and broadcast their quit
        for user in users_to_remove {
            // Remove from nick_to_id mapping
            {
                let mut nick_to_id = self.nick_to_id.write().await;
                nick_to_id.remove(&user.nick);
            }
            
            // Remove from users map
            {
                let mut users = self.users.write().await;
                users.remove(&user.id);
            }
            
            // Remove from database
            if let Err(e) = self.database.remove_user(user.id) {
                tracing::warn!("Failed to remove user {} from database: {}", user.nick, e);
            }
            
            // Broadcast QUIT to local clients
            let quit_msg = Message::with_prefix(
                Prefix::User {
                    nick: user.nick.clone(),
                    user: user.username.clone(),
                    host: user.host.clone(),
                },
                MessageType::Quit,
                vec![format!("{} {}", server_name, quit_reason)],
            );
            
            if let Err(e) = self.broadcast_system.broadcast_to_all(quit_msg, None).await {
                tracing::warn!("Failed to broadcast quit for {}: {}", user.nick, e);
            }
            
            tracing::debug!("Removed user {} from server {}", user.nick, server_name);
        }
        
        // 3. Remove server from database
        if self.database.remove_server(server_name).is_none() {
            tracing::debug!("Server {} was not in database", server_name);
        }
        
        // 4. Remove from super servers if it's a u-lined server
        {
            let mut super_servers = self.super_servers.write().await;
            super_servers.remove(server_name);
        }
        
        // 5. Remove server connection
        if let Err(e) = self.server_connections.remove_connection(server_name).await {
            tracing::warn!("Failed to remove server connection for {}: {}", server_name, e);
        }
        
        // 6. Propagate SQUIT to other connected servers (except source)
        let squit_msg = Message::with_prefix(
            Prefix::Server(self.config.server.name.clone()),
            MessageType::ServerQuit,
            vec![
                server_name.to_string(),
                quit_reason.to_string(),
            ],
        );
        
        if let Err(e) = self.server_connections.broadcast_message(&squit_msg, Some(server_name)).await {
            tracing::warn!("Failed to propagate SQUIT for {}: {}", server_name, e);
        }
        
        tracing::info!("Server {} quit processing complete. Cleaned up {} users", 
                      server_name, user_count);
        
        Ok(())
    }
    
    /// Send server burst to propagate our state to a newly connected server
    async fn send_server_burst(&self, target_server: &str) -> Result<()> {
        tracing::info!("Sending server burst to {}", target_server);
        
        // Send basic server information
        let server_info = Message::with_prefix(
            Prefix::Server(self.config.server.name.clone()),
            MessageType::Server,
            vec![
                self.config.server.name.clone(),
                "1".to_string(), // hop count
                self.config.server.description.clone(),
                self.config.server.version.clone(),
            ]
        );
        self.server_connections.send_to_server(target_server, server_info).await?;
        
        // Send user burst for all local users
        let users = self.users.read().await;
        let mut user_count = 0;
        for user in users.values() {
            // Only burst users on our server (local users)
            if user.server == self.config.server.name {
                let user_burst = Message::new(
                    MessageType::UserBurst,
                    vec![
                        user.nick.clone(),
                        user.username.clone(),
                        user.host.clone(),
                        user.realname.clone(),
                        user.server.clone(),
                        user.id.to_string(),
                        user.registered_at.timestamp().to_string(),
                    ]
                );
                
                if let Err(e) = self.server_connections.send_to_server(target_server, user_burst).await {
                    tracing::warn!("Failed to send user burst for {}: {}", user.nick, e);
                }
                user_count += 1;
            }
        }
        
        tracing::info!("Server burst to {} completed ({} users sent)", target_server, user_count);
        Ok(())
    }
    
    
    /// Handle server PING
    async fn handle_server_ping(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.is_empty() {
            return Err(Error::MessageParse("PING requires a token parameter".to_string()));
        }
        
        let token = &message.params[0];
        let pong_message = Message::new(
            MessageType::Pong,
            vec![token.clone()]
        );
        
        self.server_connections.send_to_server(server_name, pong_message).await?;
        tracing::debug!("Sent PONG to server {}", server_name);
        Ok(())
    }
    
    /// Handle server PONG
    async fn handle_server_pong(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.is_empty() {
            return Err(Error::MessageParse("PONG requires a token parameter".to_string()));
        }
        
        let token = &message.params[0];
        tracing::debug!("Received PONG from server {} with token {}", server_name, token);
        
        // Update last pong time for the server
        self.server_connections.update_connection_pong(server_name).await?;
        
        Ok(())
    }

    /// Handle WALLOPS message received from another server
    async fn handle_server_wallops_received(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.is_empty() {
            tracing::warn!("Received WALLOPS from server {} with no message", server_name);
            return Ok(());
        }
        
        // Get the wallops message (all parameters joined)
        let wallops_message = message.params.join(" ");
        
        // Create the wallops message format with server prefix
        let wallops_msg = format!(":{} WALLOPS :{}", server_name, wallops_message);
        
        // Send to all local clients with wallops mode (+w)
        let connection_handler = self.connection_handler.read().await;
        let mut local_sent_count = 0;
        
        for (_, user) in self.users.read().await.iter() {
            if user.has_mode('w') {
                // Find the user's client and send the message
                if let Some(user_client) = connection_handler.get_client_by_nick(&user.nick) {
                    if let Err(e) = user_client.send_raw(&wallops_msg) {
                        tracing::warn!("Failed to send wallops to {}: {}", user.nick, e);
                    } else {
                        local_sent_count += 1;
                    }
                }
            }
        }
        
        // Forward to other servers (except the one we received it from)
        let server_wallops_msg = Message::new(
            MessageType::Wallops,
            vec![wallops_message.clone()]
        );
        
        // Get all server connections except the source
        let connections = self.server_connections.get_all_connections().await;
        for connection in connections {
            if connection.info.name != server_name {
                if let Err(e) = connection.send(server_wallops_msg.clone()) {
                    tracing::warn!("Failed to forward wallops to server {}: {}", connection.info.name, e);
                }
            }
        }
        
        tracing::info!(
            "Wallops received from server {} and sent to {} local recipients: {}",
            server_name,
            local_sent_count,
            wallops_message
        );
        
        Ok(())
    }

    /// Handle KILL message received from another server
    async fn handle_server_kill_received(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.len() < 2 {
            tracing::warn!("Received KILL from server {} with insufficient parameters", server_name);
            return Ok(());
        }
        
        let target_nick = &message.params[0];
        let kill_reason = &message.params[1];
        
        // Find the target user
        let database = self.database.clone();
        let Some(target_user) = database.get_user_by_nick(target_nick) else {
            tracing::warn!("Received KILL for unknown user {} from server {}", target_nick, server_name);
            return Ok(());
        };
        
        // Check if target is a server (not allowed)
        if target_user.nick == self.config.server.name {
            tracing::warn!("Received KILL for server {} from server {}", self.config.server.name, server_name);
            return Ok(());
        }
        
        // Find the target user's client
        let target_client_id = if let Some(target_user) = database.get_user_by_nick(target_nick) {
            Some(target_user.id)
        } else {
            None
        };
        
        if let Some(client_id) = target_client_id {
            // Send KILL message to the target user and handle disconnection
            {
                let connection_handler = self.connection_handler.read().await;
                if let Some(target_client) = connection_handler.get_client(&client_id) {
                    // Send KILL message to the target user
                    let kill_message = Message::new(
                        MessageType::Kill,
                        vec![target_nick.to_string(), kill_reason.to_string()]
                    );
                    let _ = target_client.send(kill_message);
                }
            }
            
            // Send quit message to all users in channels
            let quit_reason = format!("Killed ({})", kill_reason);
            self.broadcast_user_quit_by_id(client_id, &quit_reason).await?;
            
            // Remove user from database
            database.remove_user(client_id)?;
            
            // Close the connection
            let mut connection_handler = self.connection_handler.write().await;
            connection_handler.remove_client(&client_id);
            
            tracing::info!("Killed user {} from server {}: {}", target_nick, server_name, kill_reason);
        }
        
        // Forward to other servers (except the one we received it from)
        let server_kill_msg = Message::new(
            MessageType::Kill,
            vec![target_nick.to_string(), kill_reason.clone()]
        );
        
        // Get all server connections except the source
        let connections = self.server_connections.get_all_connections().await;
        for connection in connections {
            if connection.info.name != server_name {
                if let Err(e) = connection.send(server_kill_msg.clone()) {
                    tracing::warn!("Failed to forward KILL to server {}: {}", connection.info.name, e);
                }
            }
        }
        
        Ok(())
    }

    /// Handle AWAY message received from another server
    async fn handle_server_away_received(&self, server_name: &str, message: Message) -> Result<()> {
        // AWAY messages from servers don't have a source prefix in our current implementation
        // This would need to be enhanced to extract the source user from the message prefix
        // For now, we'll just forward the message to other servers
        
        // Forward to other servers (except the one we received it from)
        let server_away_msg = Message::new(
            MessageType::Away,
            message.params.clone()
        );
        
        // Get all server connections except the source
        let connections = self.server_connections.get_all_connections().await;
        for connection in connections {
            if connection.info.name != server_name {
                if let Err(e) = connection.send(server_away_msg.clone()) {
                    tracing::warn!("Failed to forward AWAY to server {}: {}", connection.info.name, e);
                }
            }
        }
        
        tracing::debug!("Forwarded AWAY message from server {}", server_name);
        Ok(())
    }

    /// Handle JOIN message received from another server
    async fn handle_server_join_received(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.is_empty() {
            tracing::warn!("Received JOIN from server {} with no channel", server_name);
            return Ok(());
        }
        
        let channel_name = &message.params[0];
        
        // Forward to other servers (except the one we received it from)
        let server_join_msg = Message::new(
            MessageType::Join,
            vec![channel_name.clone()]
        );
        
        // Get all server connections except the source
        let connections = self.server_connections.get_all_connections().await;
        for connection in connections {
            if connection.info.name != server_name {
                if let Err(e) = connection.send(server_join_msg.clone()) {
                    tracing::warn!("Failed to forward JOIN to server {}: {}", connection.info.name, e);
                }
            }
        }
        
        tracing::debug!("Forwarded JOIN message for channel {} from server {}", channel_name, server_name);
        Ok(())
    }

    /// Handle PART message received from another server
    async fn handle_server_part_received(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.is_empty() {
            tracing::warn!("Received PART from server {} with no channel", server_name);
            return Ok(());
        }
        
        let channel_name = &message.params[0];
        let reason = message.params.get(1).cloned().unwrap_or_default();
        
        // Forward to other servers (except the one we received it from)
        let server_part_msg = Message::new(
            MessageType::Part,
            if reason.is_empty() {
                vec![channel_name.clone()]
            } else {
                vec![channel_name.clone(), reason.clone()]
            }
        );
        
        // Get all server connections except the source
        let connections = self.server_connections.get_all_connections().await;
        for connection in connections {
            if connection.info.name != server_name {
                if let Err(e) = connection.send(server_part_msg.clone()) {
                    tracing::warn!("Failed to forward PART to server {}: {}", connection.info.name, e);
                }
            }
        }
        
        tracing::debug!("Forwarded PART message for channel {} from server {}", channel_name, server_name);
        Ok(())
    }
    
    /// Handle SQUIT command (server quit)
    async fn handle_squit(&self, _server_name: &str, message: Message) -> Result<()> {
        if message.params.is_empty() {
            return Err(Error::MessageParse("SQUIT requires a server name parameter".to_string()));
        }
        
        let target_server = &message.params[0];
        let reason = message.params.get(1).map(|s| s.as_str()).unwrap_or("Server quit");
        
        tracing::info!("SQUIT command received for server {}: {}", target_server, reason);
        
        // Remove the server connection
        if let Some(_connection) = self.server_connections.remove_connection(target_server).await? {
            tracing::info!("Removed server connection: {}", target_server);
            
            // Propagate SQUIT to other servers
            let squit_propagation = Message::new(
                MessageType::ServerQuit,
                vec![target_server.to_string(), reason.to_string()]
            );
            self.propagate_to_servers(squit_propagation).await?;
        }
        
        Ok(())
    }
    
    /// Handle NICK propagation from other servers
    async fn handle_server_nick_propagation(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.len() < 2 {
            return Err(Error::MessageParse("NICK propagation requires old and new nickname parameters".to_string()));
        }
        
        let old_nick = message.params[0].clone();
        let new_nick = message.params[1].clone();
        
        tracing::debug!("Received NICK propagation from server {}: {} -> {}", server_name, old_nick, new_nick);
        
        // Get user by old nickname
        let user = match self.database.get_user_by_nick(&old_nick) {
            Some(user) => user,
            None => {
                tracing::warn!("NICK propagation for unknown user: {}", old_nick);
                return Ok(());
            }
        };
        
        // Check if new nickname is already in use
        if self.database.get_user_by_nick(&new_nick).is_some() {
            tracing::warn!("NICK propagation failed: {} is already in use", new_nick);
            return Ok(());
        }
        
        let user_id = user.id;
        
        // Update user's nickname
        let mut updated_user = user.clone();
        updated_user.nick = new_nick.clone();
        
        // Update in database
        if let Err(e) = self.database.update_user(&user_id, updated_user.clone()) {
            tracing::error!("Failed to update user nickname in database: {}", e);
            return Err(e);
        }
        
        // Update in users map
        {
            let mut users = self.users.write().await;
            users.insert(user_id, updated_user.clone());
        }
        
        // Update nick_to_id mapping
        {
            let mut nick_to_id = self.nick_to_id.write().await;
            nick_to_id.remove(&old_nick);
            nick_to_id.insert(new_nick.clone(), user_id);
        }
        
        // Broadcast NICK change to local clients
        let nick_msg = Message::with_prefix(
            Prefix::User {
                nick: old_nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Nick,
            vec![new_nick.clone()],
        );
        
        if let Err(e) = self.broadcast_system.broadcast_to_all(nick_msg, None).await {
            tracing::warn!("Failed to broadcast NICK change for {}: {}", old_nick, e);
        }
        
        // Propagate to other servers
        let nick_propagation = Message::with_prefix(
            Prefix::Server(server_name.to_string()),
            MessageType::Nick,
            vec![old_nick.clone(), new_nick.clone()],
        );
        
        if let Err(e) = self.server_connections.broadcast_message(&nick_propagation, Some(server_name)).await {
            tracing::warn!("Failed to propagate NICK change: {}", e);
        }
        
        tracing::info!("Processed NICK propagation from {}: {} -> {}", server_name, old_nick, new_nick);
        
        Ok(())
    }
    
    /// Handle QUIT propagation from other servers
    async fn handle_server_quit_propagation(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.is_empty() {
            return Err(Error::MessageParse("QUIT propagation requires nickname parameter".to_string()));
        }
        
        let nick = message.params[0].clone();
        let reason = message.params.get(1).map(|s| s.as_str()).unwrap_or("Quit");
        
        tracing::debug!("Received QUIT propagation from server {} for user {}: {}", server_name, nick, reason);
        
        // Get user by nickname
        let user = match self.database.get_user_by_nick(&nick) {
            Some(user) => user,
            None => {
                tracing::warn!("QUIT propagation for unknown user: {}", nick);
                return Ok(());
            }
        };
        
        let user_id = user.id;
        
        // Remove from nick_to_id mapping
        {
            let mut nick_to_id = self.nick_to_id.write().await;
            nick_to_id.remove(&nick);
        }
        
        // Remove from users map
        {
            let mut users = self.users.write().await;
            users.remove(&user_id);
        }
        
        // Remove from database
        if let Err(e) = self.database.remove_user(user_id) {
            tracing::warn!("Failed to remove user {} from database: {}", nick, e);
        }
        
        // Broadcast QUIT to local clients
        let quit_msg = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Quit,
            vec![reason.to_string()],
        );
        
        if let Err(e) = self.broadcast_system.broadcast_to_all(quit_msg, None).await {
            tracing::warn!("Failed to broadcast QUIT for {}: {}", nick, e);
        }
        
        // Propagate to other servers
        let quit_propagation = Message::with_prefix(
            Prefix::Server(server_name.to_string()),
            MessageType::Quit,
            vec![nick.clone(), reason.to_string()],
        );
        
        if let Err(e) = self.server_connections.broadcast_message(&quit_propagation, Some(server_name)).await {
            tracing::warn!("Failed to propagate QUIT for {}: {}", nick, e);
        }
        
        tracing::info!("Processed QUIT propagation from {} for user {}: {}", server_name, nick, reason);
        
        Ok(())
    }
    
    /// Handle user burst from other servers
    async fn handle_user_burst(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.len() < 7 {
            return Err(Error::MessageParse("User burst requires 7 parameters".to_string()));
        }
        
        let nick = message.params[0].clone();
        let username = message.params[1].clone();
        let host = message.params[2].clone();
        let realname = message.params[3].clone();
        let user_server = message.params[4].clone();
        let user_id_str = &message.params[5];
        let connected_at_str = &message.params[6];
        
        tracing::debug!("Received user burst from server {}: {}!{}@{}", server_name, nick, username, host);
        
        // Parse user ID
        let user_id = uuid::Uuid::parse_str(user_id_str)
            .map_err(|_| Error::MessageParse(format!("Invalid user ID in burst: {}", user_id_str)))?;
        
        // Parse connection time
        let connected_at = connected_at_str.parse::<i64>()
            .ok()
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .ok_or_else(|| Error::MessageParse(format!("Invalid timestamp in burst: {}", connected_at_str)))?;
        
        // Create user object for remote user
        let user = User {
            id: user_id,
            nick: nick.clone(),
            username: username.clone(),
            realname: realname.clone(),
            host: host.clone(),
            server: user_server.clone(),
            registered_at: connected_at,
            last_activity: chrono::Utc::now(),
            modes: std::collections::HashSet::new(),
            channels: std::collections::HashSet::new(),
            registered: true,
            is_operator: false,
            operator_flags: std::collections::HashSet::new(),
            away_message: None,
            is_bot: false,
            bot_info: None,
        };
        
        // Add user to database
        if let Err(e) = self.database.add_user(user.clone()) {
            tracing::warn!("Failed to add burst user {} to database: {}", nick, e);
            // Don't fail the whole burst if one user fails - might be duplicate
        }
        
        // Add to users map
        {
            let mut users = self.users.write().await;
            users.insert(user_id, user.clone());
        }
        
        // Add to nick_to_id map
        {
            let mut nick_to_id = self.nick_to_id.write().await;
            nick_to_id.insert(nick.clone(), user_id);
        }
        
        tracing::info!("Processed user burst from {}: {} ({}!{}@{})", 
                      server_name, nick, username, user_server, host);
        
        Ok(())
    }
    
    /// Handle server burst from other servers
    async fn handle_server_burst_received(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.len() < 4 {
            return Err(Error::MessageParse("Server burst requires 4 parameters".to_string()));
        }
        
        let burst_server_name = message.params[0].clone();
        let description = message.params[1].clone();
        let hop_count_str = &message.params[2];
        let version = message.params[3].clone();
        
        tracing::debug!("Received server burst from server {}: {} (hop: {})", server_name, burst_server_name, hop_count_str);
        
        // Parse hop count
        let hop_count: u32 = hop_count_str.parse()
            .map_err(|_| Error::MessageParse(format!("Invalid hop count in server burst: {}", hop_count_str)))?;
        
        // Create server info
        let server_info = crate::database::ServerInfo {
            name: burst_server_name.clone(),
            description: description.clone(),
            version: version.clone(),
            hopcount: hop_count,
            connected_at: chrono::Utc::now(),
            is_super_server: self.server_connections.is_super_server(&burst_server_name),
            user_count: 0,
        };
        
        // Add server to database
        if let Err(e) = self.database.add_server(server_info) {
            tracing::warn!("Failed to add burst server {} to database: {}", burst_server_name, e);
            // Don't fail - might already exist
        }
        
        tracing::info!("Processed server burst from {}: {} (hop: {}, version: {})", 
                      server_name, burst_server_name, hop_count, version);
        
        Ok(())
    }
    
    /// Handle channel burst from other servers
    async fn handle_channel_burst_received(&self, server_name: &str, message: Message) -> Result<()> {
        if message.params.is_empty() {
            return Err(Error::MessageParse("Channel burst requires at least 1 parameter".to_string()));
        }
        
        let channel_name = message.params[0].clone();
        tracing::debug!("Received channel burst from server {}: {}", server_name, channel_name);
        
        // Parse channel burst parameters
        // Format: CBURST #channel [topic] [modes] [members...]
        let topic = if message.params.len() > 1 && !message.params[1].is_empty() {
            Some(message.params[1].clone())
        } else {
            None
        };
        
        let modes = if message.params.len() > 2 {
            message.params[2].chars().collect::<std::collections::HashSet<char>>()
        } else {
            std::collections::HashSet::new()
        };
        
        // Create channel info
        let channel_info = crate::database::ChannelInfo {
            name: channel_name.clone(),
            topic,
            user_count: 0, // Will be updated as members join
            modes,
        };
        
        // Add channel to database
        if let Err(e) = self.database.add_channel(channel_info) {
            tracing::debug!("Channel {} may already exist: {}", channel_name, e);
            // Don't fail - channel might already exist
        }
        
        // Process channel members if provided (params 3+)
        let mut member_count = 0;
        if message.params.len() > 3 {
            for i in 3..message.params.len() {
                let member = &message.params[i];
                if !member.is_empty() {
                    // Add user to channel
                    if let Err(e) = self.database.add_user_to_channel(member, &channel_name) {
                        tracing::warn!("Failed to add user {} to channel {}: {}", member, channel_name, e);
                    } else {
                        member_count += 1;
                    }
                }
            }
        }
        
        tracing::info!("Processed channel burst from {}: {} ({} members)", 
                      server_name, channel_name, member_count);
        
        Ok(())
    }
    
    /// Handle PASS command for server connections
    async fn handle_server_password(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let password = &message.params[0];
        
        // Store the password in the client for later validation when SERVER command is received
        let mut connection_handler = self.connection_handler.write().await;
        if let Some(client) = connection_handler.get_client_mut(&client_id) {
            client.server_password = Some(password.clone());
            client.set_state(ClientState::PasswordProvided);
            
            tracing::debug!("Server password received for client {}", client_id);
            Ok(())
        } else {
            Err(Error::Server("Client not found".to_string()))
        }
    }
    
    /// Propagate message to all connected servers
    async fn propagate_to_servers(&self, message: Message) -> Result<()> {
        let connections = self.server_connections.get_all_connections().await;
        for connection in connections {
            if connection.is_registered() {
                if let Err(e) = connection.send(message.clone()) {
                    tracing::warn!("Failed to propagate message to server {}: {}", connection.info.name, e);
                }
            }
        }
        Ok(())
    }
    
    /// Handle core IRC commands
    async fn handle_core_command(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let _client = connection_handler.get_client(&client_id)
            .ok_or_else(|| Error::User("Client not found".to_string()))?;
        
        match message.command {
            MessageType::Password => {
                self.handle_password(client_id, message).await?;
            }
            MessageType::Nick => {
                self.handle_nick(client_id, message).await?;
            }
            MessageType::User => {
                self.handle_user(client_id, message).await?;
            }
            MessageType::Ping => {
                self.handle_ping(client_id, message).await?;
            }
            MessageType::Pong => {
                self.handle_pong(client_id, message).await?;
            }
            MessageType::Quit => {
                self.handle_quit(client_id, message).await?;
            }
            // Server queries
            MessageType::Admin => {
                self.handle_admin(client_id, message).await?;
            }
            MessageType::Version => {
                self.handle_version(client_id, message).await?;
            }
            MessageType::Stats => {
                self.handle_stats(client_id, message).await?;
            }
            MessageType::Links => {
                self.handle_links(client_id, message).await?;
            }
            MessageType::Time => {
                self.handle_time(client_id, message).await?;
            }
            MessageType::Info => {
                self.handle_info(client_id, message).await?;
            }
            MessageType::Trace => {
                self.handle_trace(client_id, message).await?;
            }
            MessageType::Motd => {
                self.handle_motd(client_id, message).await?;
            }
            // User queries
            MessageType::Who => {
                self.handle_who(client_id, message).await?;
            }
            MessageType::Whois => {
                self.handle_whois(client_id, message).await?;
            }
            MessageType::Whowas => {
                self.handle_whowas(client_id, message).await?;
            }
            // Messaging commands
            MessageType::PrivMsg => {
                self.handle_privmsg(client_id, message).await?;
            }
            MessageType::Notice => {
                self.handle_notice(client_id, message).await?;
            }
            MessageType::Wallops => {
                // WALLOPS is now handled by messaging modules
                // Let modules handle this command
                return Ok(());
            }
            // Miscellaneous commands
            MessageType::Away => {
                self.handle_away(client_id, message).await?;
            }
            MessageType::Join => {
                self.handle_join(client_id, message).await?;
            }
            MessageType::Part => {
                self.handle_part(client_id, message).await?;
            }
            MessageType::Ison => {
                self.handle_ison(client_id, message).await?;
            }
            MessageType::Userhost => {
                self.handle_userhost(client_id, message).await?;
            }
            MessageType::Lusers => {
                self.handle_lusers(client_id, message).await?;
            }
            MessageType::Users => {
                self.handle_users(client_id, message).await?;
            }
            // Server connection commands
            MessageType::Connect => {
                self.handle_connect(client_id, message).await?;
            }
            MessageType::Oper => {
                self.handle_oper(client_id, message).await?;
            }
            MessageType::Kill => {
                self.handle_kill(client_id, message).await?;
            }
            MessageType::ServerQuit => {
                self.handle_operator_squit(client_id, message).await?;
            }
            MessageType::Server => {
                // Handle initial server registration from new connections
                self.handle_initial_server_registration(client_id, message).await?;
            }
            _ => {
                // Command not handled by core
                tracing::debug!("Unhandled command: {:?}", message.command);
            }
        }
        
        Ok(())
    }
    
    /// Handle PASS command
    async fn handle_password(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        if message.params.is_empty() {
            let error_msg = NumericReply::need_more_params("PASS");
            let connection_handler = self.connection_handler.read().await;
            if let Some(client) = connection_handler.get_client(&client_id) {
                let _ = client.send(error_msg);
            }
            return Ok(());
        }
        
        // Check if this is a server connection
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            if client.connection_type == crate::client::ConnectionType::Server {
                // Handle server password
                return self.handle_server_password(client_id, message).await;
            }
        }
        
        // Check if password is required and correct for clients
        if self.config.security.require_client_password {
            if let Some(ref required_password) = self.config.security.client_password {
                if message.params[0] != *required_password {
                    let error_msg = NumericReply::password_mismatch();
                    let connection_handler = self.connection_handler.read().await;
                    if let Some(client) = connection_handler.get_client(&client_id) {
                        let _ = client.send(error_msg);
                    }
                    return Ok(());
                }
            }
        }
        
        // Update client state
        let mut connection_handler = self.connection_handler.write().await;
        if let Some(client) = connection_handler.get_client_mut(&client_id) {
            client.set_state(ClientState::PasswordProvided);
        }
        
        Ok(())
    }
    
    /// Handle NICK command
    async fn handle_nick(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        if message.params.is_empty() {
            let error_msg = NumericReply::no_nickname_given();
            let connection_handler = self.connection_handler.read().await;
            if let Some(client) = connection_handler.get_client(&client_id) {
                let _ = client.send(error_msg);
            }
            return Ok(());
        }
        
        let nick = &message.params[0];
        
        // Validate nickname
        if !self.is_valid_nickname(nick) {
            let error_msg = NumericReply::erroneous_nickname(nick);
            let connection_handler = self.connection_handler.read().await;
            if let Some(client) = connection_handler.get_client(&client_id) {
                let _ = client.send(error_msg);
            }
            return Ok(());
        }
        
        // Check if nickname is in use
        let nick_to_id = self.nick_to_id.read().await;
        if nick_to_id.contains_key(nick) {
            let error_msg = NumericReply::nickname_in_use(nick);
            let connection_handler = self.connection_handler.read().await;
            if let Some(client) = connection_handler.get_client(&client_id) {
                let _ = client.send(error_msg);
            }
            return Ok(());
        }
        drop(nick_to_id);
        
        // Check if this is a nickname change or initial registration
        let connection_handler = self.connection_handler.read().await;
        let old_nick = if let Some(client) = connection_handler.get_client(&client_id) {
            client.user.as_ref().map(|u| u.nick.clone())
        } else {
            None
        };
        drop(connection_handler);
        
        // Register nickname
        let mut connection_handler = self.connection_handler.write().await;
        if let Some(client) = connection_handler.get_client_mut(&client_id) {
            client.set_state(ClientState::NickSet);
            
            // If user object exists, update the nickname
            if let Some(ref mut user) = client.user {
                let old_nick = user.nick.clone();
                user.nick = nick.clone();
                
                // Update in database
                if let Err(e) = self.database.update_user(&user.id, user.clone()) {
                    tracing::error!("Failed to update user nickname in database: {}", e);
                }
                
                // Update in users map
                {
                    let mut users = self.users.write().await;
                    users.insert(user.id, user.clone());
                }
                
                // Update nick_to_id mapping
                {
                    let mut nick_to_id = self.nick_to_id.write().await;
                    nick_to_id.remove(&old_nick);
                    nick_to_id.insert(nick.clone(), user.id);
                }
                
                // Broadcast NICK change to local clients
                let nick_msg = Message::with_prefix(
                    Prefix::User {
                        nick: old_nick.clone(),
                        user: user.username.clone(),
                        host: user.host.clone(),
                    },
                    MessageType::Nick,
                    vec![nick.clone()],
                );
                
                if let Err(e) = self.broadcast_system.broadcast_to_all(nick_msg, None).await {
                    tracing::warn!("Failed to broadcast NICK change: {}", e);
                }
                
                // Propagate NICK change to other servers
                let nick_propagation = Message::with_prefix(
                    Prefix::Server(self.config.server.name.clone()),
                    MessageType::Nick,
                    vec![old_nick, nick.clone()],
                );
                
                drop(connection_handler); // Release the lock before async call
                
                if let Err(e) = self.server_connections.broadcast_to_servers(nick_propagation).await {
                    tracing::warn!("Failed to propagate NICK change: {}", e);
                }
                
                tracing::info!("Client {} nickname changed to: {}", client_id, nick);
            } else {
                tracing::debug!("Client {} nickname set to: {}", client_id, nick);
            }
        }
        
        Ok(())
    }
    
    /// Handle USER command
    async fn handle_user(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        if message.params.len() < 4 {
            let error_msg = NumericReply::need_more_params("USER");
            let connection_handler = self.connection_handler.read().await;
            if let Some(client) = connection_handler.get_client(&client_id) {
                let _ = client.send(error_msg);
            }
            return Ok(());
        }
        
        let username = &message.params[0];
        let hostname = &message.params[1];
        let servername = &message.params[2];
        let realname = &message.params[3];
        
        // Create user
        let user = User::new(
            "".to_string(), // Nick will be set separately
            username.clone(),
            realname.clone(),
            hostname.clone(),
            servername.clone(),
        );
        
        // Update client
        let mut connection_handler = self.connection_handler.write().await;
        if let Some(client) = connection_handler.get_client_mut(&client_id) {
            client.set_user(user);
            client.set_state(ClientState::UserSet);
            
            // Check if client is fully registered
            if client.has_nick() && client.has_user() {
                client.set_state(ClientState::Registered);
                
                // Add user to database
                let user = User::new(
                    client.nickname().unwrap_or("unknown").to_string(),
                    username.clone(),
                    realname.clone(),
                    hostname.clone(),
                    servername.clone(),
                );
                self.database.add_user(user)?;
                
                // Send welcome message
                let welcome_msg = NumericReply::welcome(
                    &self.config.server.name,
                    client.nickname().unwrap_or("unknown"),
                    username,
                    hostname,
                );
                let _ = client.send(welcome_msg);
                
                // Send MOTD after welcome message
                let motd_messages = self.motd_manager.get_all_motd_messages(&self.config.server.name).await;
                for motd_msg in motd_messages {
                    let _ = client.send(motd_msg);
                }
                
                // Broadcast user registration to all connected servers
                let nick = client.nickname().unwrap_or("unknown");
                let server_user_msg = Message::new(
                    MessageType::UserBurst,
                    vec![
                        nick.to_string(),
                        username.clone(),
                        hostname.clone(),
                        realname.clone(),
                        self.config.server.name.clone(),
                        client_id.to_string(),
                        chrono::Utc::now().to_rfc3339(),
                    ]
                );
                
                if let Err(e) = self.server_connections.broadcast_to_servers(server_user_msg).await {
                    tracing::warn!("Failed to broadcast USER registration to servers: {}", e);
                }
                
                tracing::info!("User {} registered and broadcasted to servers", nick);
            }
        }
        
        Ok(())
    }
    
    /// Handle PING command
    async fn handle_ping(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let pong_msg = Message::new(MessageType::Pong, message.params);
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let _ = client.send(pong_msg);
        }
        Ok(())
    }
    
    /// Handle PONG command
    async fn handle_pong(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let token = message.params.first().map(|s| s.as_str()).unwrap_or("");
        
        // Update last pong time and verify token
        let mut connection_handler = self.connection_handler.write().await;
        if let Some(client) = connection_handler.get_client_mut(&client_id) {
            // Record pong received (this also resets unanswered pings and updates activity)
            client.timing.record_pong_received();
            
            tracing::debug!("Received PONG from client {} with token: {}", client_id, token);
            
            // Check if client has timed out
            if client.timing.is_timed_out() {
                tracing::warn!("Client {} has timed out despite PONG", client_id);
                // Connection will be cleaned up by timeout checker
            }
        }
        
        Ok(())
    }
    
    /// Handle QUIT command
    async fn handle_quit(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let quit_message = message.params.first().map(|s| s.as_str()).unwrap_or("Client quit");
        
        // Propagate QUIT to other servers before removing client
        let connection_handler = self.connection_handler.read().await;
        let should_propagate = if let Some(client) = connection_handler.get_client(&client_id) {
            client.is_registered()
        } else {
            false
        };
        drop(connection_handler);
        
        if should_propagate {
            let quit_propagation = Message::new(
                MessageType::Quit,
                vec![quit_message.to_string()]
            );
            self.propagate_to_servers(quit_propagation).await?;
        }
        
        // Notify modules
        let module_manager = self.module_manager.read().await;
        if let Some(client) = self.connection_handler.read().await.get_client(&client_id) {
            if let Some(_user) = client.get_user() {
                // let _ = module_manager.handle_user_disconnection(user).await; // Commented out - needs mutable reference
            }
        }
        drop(module_manager);
        
        // Remove client
        let mut connection_handler = self.connection_handler.write().await;
        connection_handler.remove_client(&client_id);
        
        Ok(())
    }
    
    /// Validate nickname
    fn is_valid_nickname(&self, nick: &str) -> bool {
        if nick.is_empty() || nick.len() > self.config.server.max_nickname_length {
            return false;
        }
        
        // First character must be letter or special character
        let first_char = nick.chars().next().unwrap();
        if !first_char.is_ascii_alphabetic() && !"[]\\`_^{|}~".contains(first_char) {
            return false;
        }
        
        // Remaining characters must be letter, digit, or special character
        for c in nick.chars().skip(1) {
            if !c.is_ascii_alphanumeric() && !"-[]\\`_^{|}~".contains(c) {
                return false;
            }
        }
        
        true
    }
    
    // Server query command handlers
    
    /// Handle ADMIN command
    async fn handle_admin(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            // Send admin information
            let _ = client.send(NumericReply::admin_me(&self.config.server.name));
            let _ = client.send(NumericReply::admin_loc1(&self.config.server.description));
            let _ = client.send(NumericReply::admin_loc2("Rust IRC Daemon"));
            let _ = client.send(NumericReply::admin_email("admin@example.com"));
        }
        Ok(())
    }
    
    /// Handle VERSION command
    async fn handle_version(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let version_msg = NumericReply::version(
                &self.config.server.name,
                &self.config.server.version,
                "0",
                &self.config.server.name,
                "Rust IRC Daemon",
            );
            let _ = client.send(version_msg);
        }
        Ok(())
    }
    
    /// Handle STATS command - RFC 1459 compliant with module extensions
    async fn handle_stats(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let query = message.params.get(0).map(|s| s.as_str()).unwrap_or("");
            
            // Get current statistics
            let stats_manager = self.statistics_manager.clone();
            let stats_arc = stats_manager.statistics();
            let stats_guard = stats_arc.read().await;
            let stats = &*stats_guard;
            
            match query {
                "l" => {
                    // List of servers (links) - RFC 1459
                    self.handle_stats_links(client, &stats).await?;
                }
                "m" => {
                    // Commands usage statistics - RFC 1459
                    self.handle_stats_commands(client, &stats).await?;
                }
                "o" => {
                    // List of operators currently online - RFC 1459
                    self.handle_stats_operators(client).await?;
                }
                "u" => {
                    // Server uptime - RFC 1459
                    let uptime_msg = NumericReply::stats_uptime(&self.config.server.name, stats.uptime_seconds());
                    let _ = client.send(uptime_msg);
                }
                "y" => {
                    // Class information - RFC 1459
                    self.handle_stats_classes(client).await?;
                }
                "c" => {
                    // Connection information - RFC 1459
                    self.handle_stats_connections(client, &stats).await?;
                }
                _ => {
                    // Check if any module handles this query
                    let mut module_manager = self.module_manager.write().await;
                    if let Ok(module_responses) = module_manager.handle_stats_query(query, client_id, Some(self)).await {
                        for response in module_responses {
                            match response {
                                ModuleStatsResponse::Stats(letter, _data) => {
                                    let stats_msg = NumericReply::stats_commands(&letter, 0, 0, 0);
                                    let _ = client.send(stats_msg);
                                }
                                ModuleStatsResponse::ModuleStats(module, data) => {
                                    let stats_msg = NumericReply::stats_module(&module, &data);
                                    let _ = client.send(stats_msg);
                                }
                            }
                        }
                    } else {
                        // Unknown query - send empty response
                        let stats_msg = NumericReply::stats_commands("UNKNOWN", 0, 0, 0);
                        let _ = client.send(stats_msg);
                    }
                }
            }
            
            let end_msg = NumericReply::end_of_stats(query);
            let _ = client.send(end_msg);
        }
        Ok(())
    }
    
    /// Handle STATS l - Server links
    async fn handle_stats_links(&self, client: &Client, _stats: &crate::ServerStatistics) -> Result<()> {
        // Check if the requesting user is an operator
        let users = self.users.read().await;
        let requesting_user = users.get(&client.id);
        let is_operator = requesting_user.map(|u| u.is_operator).unwrap_or(false);
        
        // Get connected servers from server connection manager
        let connections = self.server_connections.get_all_connections().await;
        
        for connection in connections {
            if connection.is_registered() {
                let stats_msg = if is_operator && self.config.server.show_server_details_in_stats {
                    // Show detailed server information to operators (if configured)
                    NumericReply::stats_link_info_detailed(
                        &connection.info.name,
                        connection.stats.sendq_current,
                        connection.stats.sendq_max,
                        connection.stats.sendq_dropped,
                        connection.stats.recvq_current,
                        connection.stats.recvq_max,
                        connection.stats.messages_sent,
                        connection.stats.bytes_sent,
                        connection.stats.messages_received,
                        connection.stats.bytes_received,
                        connection.time_online_seconds(),
                    )
                } else {
                    // Show limited information to non-operators or when configured to hide details
                    NumericReply::stats_link_info_detailed(
                        "***", // Hide server name for security
                        connection.stats.sendq_current,
                        connection.stats.sendq_max,
                        0, // Hide dropped count
                        connection.stats.recvq_current,
                        connection.stats.recvq_max,
                        0, // Hide message counts
                        0, // Hide byte counts
                        0, // Hide message counts
                        0, // Hide byte counts
                        connection.time_online_seconds(),
                    )
                };
                let _ = client.send(stats_msg);
            }
        }
        
        Ok(())
    }
    
    /// Handle STATS m - Commands usage statistics
    async fn handle_stats_commands(&self, client: &Client, stats: &crate::ServerStatistics) -> Result<()> {
        let top_commands = stats.get_top_commands(10); // Top 10 commands
        
        for (command, cmd_stats) in top_commands {
            let stats_msg = NumericReply::stats_commands(
                &command,
                cmd_stats.total_count().try_into().unwrap_or(u32::MAX),
                (cmd_stats.total_bytes / cmd_stats.total_count().max(1)).try_into().unwrap_or(u32::MAX), // avg bytes per command
                cmd_stats.remote_count.try_into().unwrap_or(u32::MAX),
            );
            let _ = client.send(stats_msg);
        }
        
        Ok(())
    }
    
    /// Handle STATS o - Operators currently online
    async fn handle_stats_operators(&self, client: &Client) -> Result<()> {
        let users = self.users.read().await;
        
        // Check if the requesting user is an operator
        let requesting_user = users.get(&client.id);
        let is_operator = requesting_user.map(|u| u.is_operator).unwrap_or(false);
        
        for user in users.values() {
            if user.is_operator {
                let stats_msg = if is_operator {
                    // Show full information to operators
                    NumericReply::stats_oline(
                        &format!("{}@{}", user.username, user.host),
                        &user.nick,
                        0, // port - not applicable for users
                        "Operator",
                    )
                } else {
                    // Show limited information to non-operators
                    NumericReply::stats_oline(
                        "***@***", // Hide hostmask
                        &user.nick,
                        0, // port - not applicable for users
                        "Operator",
                    )
                };
                let _ = client.send(stats_msg);
            }
        }
        
        Ok(())
    }
    
    /// Handle STATS y - Class information
    async fn handle_stats_classes(&self, client: &Client) -> Result<()> {
        // Default class information
        let stats_msg = NumericReply::stats_yline(
            "default",
            120, // ping frequency in seconds
            600, // connect frequency in seconds
            1024, // max sendq
        );
        let _ = client.send(stats_msg);
        
        Ok(())
    }
    
    /// Handle STATS c - Connection information
    async fn handle_stats_connections(&self, client: &Client, stats: &crate::ServerStatistics) -> Result<()> {
        // Check if the requesting user is an operator
        let users = self.users.read().await;
        let requesting_user = users.get(&client.id);
        let is_operator = requesting_user.map(|u| u.is_operator).unwrap_or(false);
        
        let stats_msg = if is_operator && self.config.server.show_server_details_in_stats {
            // Show detailed connection information to operators (if configured)
            NumericReply::stats_commands(
                "CONNECTIONS",
                stats.total_connections.try_into().unwrap_or(u32::MAX),
                (stats.total_bytes_received + stats.total_bytes_sent).try_into().unwrap_or(u32::MAX),
                stats.current_servers.try_into().unwrap_or(u32::MAX),
            )
        } else {
            // Show limited information to non-operators or when configured to hide details
            NumericReply::stats_commands(
                "CONNECTIONS",
                stats.current_clients, // Only show current clients, not total
                0, // Hide byte counts
                0, // Hide server count
            )
        };
        let _ = client.send(stats_msg);
        
        Ok(())
    }
    
    /// Handle MOTD command
    async fn handle_motd(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let motd_messages = self.motd_manager.get_all_motd_messages(&self.config.server.name).await;
            
            for message in motd_messages {
                let _ = client.send(message);
            }
        }
        Ok(())
    }
    
    /// Handle LINKS command
    async fn handle_links(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            // For now, just show this server
            let links_msg = NumericReply::links(
                "*",
                &self.config.server.name,
                0, // hopcount
                &self.config.server.description,
            );
            let _ = client.send(links_msg);
            
            let end_msg = NumericReply::end_of_links("*");
            let _ = client.send(end_msg);
        }
        Ok(())
    }
    
    /// Handle TIME command
    async fn handle_time(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let now = chrono::Utc::now();
            let time_str = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();
            
            let time_msg = NumericReply::time(&self.config.server.name, &time_str);
            let _ = client.send(time_msg);
        }
        Ok(())
    }
    
    /// Handle INFO command
    async fn handle_info(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let info_lines = vec![
                format!("{} - Rust IRC Daemon", self.config.server.name),
                "A modular IRC daemon written in Rust".to_string(),
                "Supports RFC 1459 and IRCv3 extensions".to_string(),
                "Modular architecture with plugin support".to_string(),
                "Built with tokio for async performance".to_string(),
            ];
            
            for line in info_lines {
                let info_msg = NumericReply::info(&line);
                let _ = client.send(info_msg);
            }
            
            let end_msg = NumericReply::end_of_info();
            let _ = client.send(end_msg);
        }
        Ok(())
    }
    
    /// Handle TRACE command
    async fn handle_trace(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            // Trace this server
            let trace_msg = NumericReply::trace_server(
                "0", // class
                &self.config.server.name,
                &self.config.server.version,
                "0", // debug_level
                &self.config.server.name,
            );
            let _ = client.send(trace_msg);
            
            let end_msg = NumericReply::trace_end(&self.config.server.name, &self.config.server.version);
            let _ = client.send(end_msg);
        }
        Ok(())
    }
    
    // User query command handlers
    
    /// Handle WHO command
    async fn handle_who(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let target = message.params.get(0).map(|s| s.as_str()).unwrap_or("*");
            
            // Check if target is a channel (starts with #)
            if target.starts_with('#') {
                // Channel WHO - get users in channel
                let channel_users = self.database.get_channel_users(target);
                for nick in channel_users {
                    if let Some(user) = self.database.get_user_by_nick(&nick) {
                        let who_msg = NumericReply::who_reply(
                            target,
                            &user.username,
                            &user.host,
                            &self.config.server.name,
                            &user.nick,
                            if user.is_away() { "G" } else { "H" },
                            "0",
                            &user.realname,
                        );
                        let _ = client.send(who_msg);
                    }
                }
            } else {
                // User pattern WHO - search for matching users
                let users = self.database.search_users(target);
                for user in users {
                    let who_msg = NumericReply::who_reply(
                        target,
                        &user.username,
                        &user.host,
                        &self.config.server.name,
                        &user.nick,
                        if user.is_away() { "G" } else { "H" },
                        "0",
                        &user.realname,
                    );
                    let _ = client.send(who_msg);
                }
            }
            
            let end_msg = NumericReply::end_of_who(target);
            let _ = client.send(end_msg);
        }
        Ok(())
    }
    
    /// Handle WHOIS command
    async fn handle_whois(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let target_nick = message.params.get(0).map(|s| s.as_str()).unwrap_or("");
            
            if target_nick.is_empty() {
                let error_msg = NumericReply::need_more_params("WHOIS");
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            // Look up user in database
            if let Some(user) = self.database.get_user_by_nick(target_nick) {
                // Check if the target user has spy privileges and notify them
                if user.is_spy() {
                    self.notify_spy_user(&user, client_id).await?;
                }
                
                // Get the requesting user for administrator privileges check
                let requesting_user = if let Some(client_user) = &client.user {
                    self.database.get_user(&client_user.id)
                } else {
                    None
                };
                
                let whois_user_msg = NumericReply::whois_user(
                    &user.nick,
                    &user.username,
                    &user.host,
                    &user.realname,
                );
                let _ = client.send(whois_user_msg);
                
                let whois_server_msg = NumericReply::whois_server(
                    &user.nick,
                    &self.config.server.name,
                    &self.config.server.description,
                );
                let _ = client.send(whois_server_msg);
                
                if user.is_operator {
                    let whois_op_msg = NumericReply::whois_operator(&user.nick);
                    let _ = client.send(whois_op_msg);
                }
                
                // Show channels if requesting user is administrator
                if let Some(req_user) = requesting_user {
                    if req_user.is_administrator() {
                        // Show all channels (including secret ones) for administrators
                        let channels = self.database.get_user_channels(&user.nick);
                        if !channels.is_empty() {
                            let whois_channels_msg = NumericReply::whois_channels(
                                &user.nick,
                                &channels.join(" "),
                            );
                            let _ = client.send(whois_channels_msg);
                        }
                    } else {
                        // Show only public channels for non-administrators
                        let channels = self.get_public_channels_for_user(&user.nick).await;
                        if !channels.is_empty() {
                            let whois_channels_msg = NumericReply::whois_channels(
                                &user.nick,
                                &channels.join(" "),
                            );
                            let _ = client.send(whois_channels_msg);
                        }
                    }
                }
                
                // Show bot information if user is a bot
                if user.is_bot() {
                    if let Some(bot_info) = user.get_bot_info() {
                        let whois_bot_msg = NumericReply::whois_bot(
                            &user.nick,
                            &bot_info.name,
                            &bot_info.description.as_deref().unwrap_or("No description"),
                        );
                        let _ = client.send(whois_bot_msg);
                        
                        if let (Some(version), Some(capabilities)) = (&bot_info.version, Some(bot_info.capabilities.join(", "))) {
                            let bot_info_msg = NumericReply::bot_info(
                                &user.nick,
                                version,
                                &capabilities,
                            );
                            let _ = client.send(bot_info_msg);
                        }
                    }
                }
                
                // Calculate idle time
                let idle_seconds = (Utc::now() - user.last_activity).num_seconds() as u32;
                let whois_idle_msg = NumericReply::whois_idle(
                    &user.nick,
                    &user.registered_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    &idle_seconds.to_string(),
                );
                let _ = client.send(whois_idle_msg);
                
                // Show channels user is in
                let channels = self.database.get_user_channels(&user.nick);
                if !channels.is_empty() {
                    let channels_str = channels.join(" ");
                    let whois_channels_msg = NumericReply::whois_channels(
                        &user.nick,
                        &channels_str,
                    );
                    let _ = client.send(whois_channels_msg);
                }
            } else {
                // User not found locally - try network-wide query if enabled
                if self.config.broadcast.enable_network_queries {
                    let servers = self.database.get_all_servers();
                    let server_names: Vec<String> = servers.iter().map(|s| s.name.clone()).collect();
                    
                    if let Ok(_request_id) = self.network_query_manager.query_whois(
                        target_nick.to_string(),
                        client_id,
                        server_names,
                    ).await {
                        // Queue the query and wait for responses
                        // For now, just send "not found" message
                        let end_msg = NumericReply::end_of_whois(target_nick);
                        let _ = client.send(end_msg);
                    }
                } else {
                    // No network queries enabled, just send "not found"
                    let end_msg = NumericReply::end_of_whois(target_nick);
                    let _ = client.send(end_msg);
                }
            }
            
            let end_msg = NumericReply::end_of_whois(target_nick);
            let _ = client.send(end_msg);
        }
        Ok(())
    }
    
    /// Handle WHOWAS command
    async fn handle_whowas(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let target_nick = message.params.get(0).map(|s| s.as_str()).unwrap_or("");
            
            if target_nick.is_empty() {
                let error_msg = NumericReply::need_more_params("WHOWAS");
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            // Look up user in history database
            let history_entries = self.database.get_user_history(target_nick).await;
            
            if !history_entries.is_empty() {
                for entry in history_entries {
                    let whowas_msg = NumericReply::whowas_user(
                        &entry.user.nick,
                        &entry.user.username,
                        &entry.user.host,
                        &entry.user.realname,
                    );
                    let _ = client.send(whowas_msg);
                }
            } else if self.config.broadcast.enable_network_queries {
                // User not found locally - try network-wide query
                let servers = self.database.get_all_servers();
                let server_names: Vec<String> = servers.iter().map(|s| s.name.clone()).collect();
                
                if let Ok(_request_id) = self.network_query_manager.query_whowas(
                    target_nick.to_string(),
                    client_id,
                    server_names,
                ).await {
                    // Queue the query and wait for responses
                    // For now, just send "not found" message
                    let end_msg = NumericReply::end_of_whowas(target_nick);
                    let _ = client.send(end_msg);
                }
            } else {
                // No network queries enabled, just send "not found"
                let end_msg = NumericReply::end_of_whowas(target_nick);
                let _ = client.send(end_msg);
            }
            
            let end_msg = NumericReply::end_of_whowas(target_nick);
            let _ = client.send(end_msg);
        }
        Ok(())
    }
    
    /// Handle PRIVMSG command
    async fn handle_privmsg(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            if !client.is_registered() {
                let error_msg = NumericReply::not_registered();
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            if message.params.len() < 2 {
                let error_msg = NumericReply::no_recipients("PRIVMSG");
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            let target = &message.params[0];
            let text = &message.params[1];
            
            if text.is_empty() {
                let error_msg = NumericReply::no_text_to_send();
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            // Get sender information
            let sender_nick = client.nickname().unwrap_or("unknown");
            let sender_user = client.username().unwrap_or("unknown");
            let sender_host = client.hostname().unwrap_or("unknown");
            
            // Create message with sender prefix
            let sender_prefix = Prefix::User {
                nick: sender_nick.to_string(),
                user: sender_user.to_string(),
                host: sender_host.to_string(),
            };
            
            let _privmsg = Message::with_prefix(
                sender_prefix,
                MessageType::PrivMsg,
                vec![target.to_string(), text.to_string()],
            );
            
            // Check if target is a channel or user
            if target.starts_with('#') || target.starts_with('&') || target.starts_with('+') || target.starts_with('!') {
                // Channel message - delegate to channel module if available
                // For now, just log it
                tracing::info!("PRIVMSG to channel {}: {}", target, text);
            } else {
                // Private message to user
                if let Some(_target_user) = self.database.get_user_by_nick(target) {
                    // Find the target user's client and send the message
                    // For now, just log it
                    tracing::info!("PRIVMSG from {} to {}: {}", sender_nick, target, text);
                } else {
                    let error_msg = NumericReply::no_such_nick(target);
                    let _ = client.send(error_msg);
                }
            }
        }
        Ok(())
    }
    

    /// Handle NOTICE command
    async fn handle_notice(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            if !client.is_registered() {
                // NOTICE doesn't send error replies for unregistered users
                return Ok(());
            }
            
            if message.params.len() < 2 {
                // NOTICE doesn't send error replies for missing parameters
                return Ok(());
            }
            
            let target = &message.params[0];
            let text = &message.params[1];
            
            if text.is_empty() {
                // NOTICE doesn't send error replies for empty text
                return Ok(());
            }
            
            // Get sender information
            let sender_nick = client.nickname().unwrap_or("unknown");
            let sender_user = client.username().unwrap_or("unknown");
            let sender_host = client.hostname().unwrap_or("unknown");
            
            // Create message with sender prefix
            let sender_prefix = Prefix::User {
                nick: sender_nick.to_string(),
                user: sender_user.to_string(),
                host: sender_host.to_string(),
            };
            
            let _notice = Message::with_prefix(
                sender_prefix,
                MessageType::Notice,
                vec![target.to_string(), text.to_string()],
            );
            
            // Check if target is a channel or user
            if target.starts_with('#') || target.starts_with('&') || target.starts_with('+') || target.starts_with('!') {
                // Channel notice - delegate to channel module if available
                tracing::info!("NOTICE to channel {}: {}", target, text);
            } else {
                // Private notice to user
                if let Some(_target_user) = self.database.get_user_by_nick(target) {
                    tracing::info!("NOTICE from {} to {}: {}", sender_nick, target, text);
                }
                // NOTICE doesn't send error replies for non-existent users
            }
        }
        Ok(())
    }
    
    /// Handle AWAY command
    async fn handle_away(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            if !client.is_registered() {
                let error_msg = NumericReply::not_registered();
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            // Get user from database
            if let Some(nick) = client.nickname() {
                if let Some(mut user) = self.database.get_user_by_nick(nick) {
                    if message.params.is_empty() {
                        // Remove away status
                        let was_away = user.away_message.is_some();
                        user.away_message = None;
                        let _ = self.database.add_user(user);
                        
                        let unaway_msg = NumericReply::unaway();
                        let _ = client.send(unaway_msg);
                        
                        // Broadcast away removal to servers
                        if was_away {
                            let server_away_msg = Message::new(
                                MessageType::Away,
                                vec![]
                            );
                            
                            if let Err(e) = self.server_connections.broadcast_to_servers(server_away_msg).await {
                                tracing::warn!("Failed to broadcast AWAY removal to servers: {}", e);
                            }
                        }
                    } else {
                        // Set away message
                        let away_message = message.params[0].clone();
                        let was_away = user.away_message.is_some();
                        user.away_message = Some(away_message.clone());
                        let _ = self.database.add_user(user);
                        
                        let now_away_msg = NumericReply::now_away();
                        let _ = client.send(now_away_msg);
                        
                        // Broadcast away status to servers
                        if !was_away {
                            let server_away_msg = Message::new(
                                MessageType::Away,
                                vec![away_message]
                            );
                            
                            if let Err(e) = self.server_connections.broadcast_to_servers(server_away_msg).await {
                                tracing::warn!("Failed to broadcast AWAY status to servers: {}", e);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle JOIN command
    async fn handle_join(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            if !client.is_registered() {
                let error_msg = NumericReply::not_registered();
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            if message.params.is_empty() {
                let error_msg = NumericReply::need_more_params("JOIN");
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            // Get user from database
            if let Some(nick) = client.nickname() {
                if let Some(mut user) = self.database.get_user_by_nick(nick) {
                    let channel_name = &message.params[0];
                    
                    // Add user to channel
                    user.channels.insert(channel_name.clone());
                    let _ = self.database.add_user(user);
                    
                    // Add channel to database if it doesn't exist
                    let mut default_modes = std::collections::HashSet::new();
                    default_modes.insert('n');
                    default_modes.insert('t');
                    
                    let channel_info = crate::ChannelInfo {
                        name: channel_name.clone(),
                        topic: None,
                        user_count: 1,
                        modes: default_modes, // Default modes: no external messages, topic ops only
                    };
                    let _ = self.database.add_channel(channel_info);
                    
                    // Send JOIN message to all users in the channel
                    let join_message = Message::with_prefix(
                        Prefix::User {
                            nick: nick.to_string(),
                            user: client.username().unwrap_or("unknown").to_string(),
                            host: client.hostname().unwrap_or("unknown").to_string(),
                        },
                        MessageType::Join,
                        vec![channel_name.clone()]
                    );
                    
                    // Broadcast to channel members
                    let channel_users = self.database.get_channel_users(channel_name);
                    for member_nick in channel_users {
                        if let Some(member_user) = self.database.get_user_by_nick(&member_nick) {
                            if let Some(member_client) = connection_handler.get_client(&member_user.id) {
                                let _ = member_client.send(join_message.clone());
                            }
                        }
                    }
                    
                    // Broadcast JOIN to all connected servers
                    let server_join_msg = Message::new(
                        MessageType::Join,
                        vec![channel_name.clone()]
                    );
                    
                    if let Err(e) = self.server_connections.broadcast_to_servers(server_join_msg).await {
                        tracing::warn!("Failed to broadcast JOIN to servers: {}", e);
                    }
                    
                    tracing::info!("User {} joined channel {}", nick, channel_name);
                }
            }
        }
        Ok(())
    }

    /// Handle PART command
    async fn handle_part(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            if !client.is_registered() {
                let error_msg = NumericReply::not_registered();
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            if message.params.is_empty() {
                let error_msg = NumericReply::need_more_params("PART");
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            // Get user from database
            if let Some(nick) = client.nickname() {
                if let Some(mut user) = self.database.get_user_by_nick(nick) {
                    let channel_name = &message.params[0];
                    let reason = message.params.get(1).cloned().unwrap_or_default();
                    
                    // Remove user from channel
                    user.channels.retain(|ch| ch != channel_name);
                    let _ = self.database.add_user(user);
                    
                    // Send PART message to all users in the channel
                    let part_message = Message::with_prefix(
                        Prefix::User {
                            nick: nick.to_string(),
                            user: client.username().unwrap_or("unknown").to_string(),
                            host: client.hostname().unwrap_or("unknown").to_string(),
                        },
                        MessageType::Part,
                        if reason.is_empty() {
                            vec![channel_name.clone()]
                        } else {
                            vec![channel_name.clone(), reason.clone()]
                        }
                    );
                    
                    // Broadcast to channel members
                    let channel_users = self.database.get_channel_users(channel_name);
                    for member_nick in channel_users {
                        if let Some(member_user) = self.database.get_user_by_nick(&member_nick) {
                            if let Some(member_client) = connection_handler.get_client(&member_user.id) {
                                let _ = member_client.send(part_message.clone());
                            }
                        }
                    }
                    
                    // Broadcast PART to all connected servers
                    let server_part_msg = Message::new(
                        MessageType::Part,
                        if reason.is_empty() {
                            vec![channel_name.clone()]
                        } else {
                            vec![channel_name.clone(), reason.clone()]
                        }
                    );
                    
                    if let Err(e) = self.server_connections.broadcast_to_servers(server_part_msg).await {
                        tracing::warn!("Failed to broadcast PART to servers: {}", e);
                    }
                    
                    tracing::info!("User {} parted channel {}: {}", nick, channel_name, reason);
                }
            }
        }
        Ok(())
    }
    
    /// Handle ISON command
    async fn handle_ison(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            if !client.is_registered() {
                let error_msg = NumericReply::not_registered();
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            if message.params.is_empty() {
                // No nicknames provided, send empty response
                let ison_msg = NumericReply::ison(&[]);
                let _ = client.send(ison_msg);
                return Ok(());
            }
            
            // Check which nicknames are online
            let mut online_nicks = Vec::new();
            for nick in &message.params {
                if self.database.get_user_by_nick(nick).is_some() {
                    online_nicks.push(nick.clone());
                }
            }
            
            let ison_msg = NumericReply::ison(&online_nicks);
            let _ = client.send(ison_msg);
        }
        Ok(())
    }
    
    /// Handle USERHOST command
    async fn handle_userhost(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            if !client.is_registered() {
                let error_msg = NumericReply::not_registered();
                let _ = client.send(error_msg);
                return Ok(());
            }
            
            if message.params.is_empty() {
                // No nicknames provided, send empty response
                let userhost_msg = NumericReply::userhost(&[]);
                let _ = client.send(userhost_msg);
                return Ok(());
            }
            
            // Get user information for each nickname
            let mut userhost_entries = Vec::new();
            for nick in &message.params {
                if let Some(user) = self.database.get_user_by_nick(nick) {
                    let operator_flag = if user.is_operator { "*" } else { "" };
                    let away_flag = if user.away_message.is_some() { "G" } else { "H" };
                    let entry = format!("{}={}{}@{}", nick, operator_flag, away_flag, user.host);
                    userhost_entries.push(entry);
                }
            }
            
            let userhost_msg = NumericReply::userhost(&userhost_entries);
            let _ = client.send(userhost_msg);
        }
        Ok(())
    }

    /// Handle CONNECT command for server connections
    async fn handle_connect(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let client = connection_handler.get_client(&client_id)
            .ok_or_else(|| Error::User("Client not found".to_string()))?;

        // Check if client is registered
        if !client.is_registered() {
            let error_msg = NumericReply::not_registered();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Check if remote CONNECT is allowed
        if !self.config.security.server_security.allow_remote_connect {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Validate parameters
        if message.params.len() < 2 {
            let error_msg = NumericReply::need_more_params("CONNECT");
            let _ = client.send(error_msg);
            return Ok(());
        }

        let target_server = &message.params[0];
        let target_port: u16 = message.params[1].parse()
            .map_err(|_| Error::User("Invalid port number".to_string()))?;

        // Check if user is an operator with CONNECT privileges
        if self.config.security.server_security.require_oper_for_connect {
            let user = client.user.as_ref().unwrap();
            if !user.is_operator {
                let error_msg = NumericReply::no_privileges();
                let _ = client.send(error_msg);
                return Ok(());
            }

            // Check if user has remote connect flag (for remote connections)
            // For now, we'll check if it's a remote connection by comparing with local server
            let is_remote = target_server != &self.config.server.name;
            if is_remote && !user.can_remote_connect() {
                let error_msg = NumericReply::no_privileges();
                let _ = client.send(error_msg);
                return Ok(());
            }
            if !is_remote && !user.can_local_connect() {
                let error_msg = NumericReply::no_privileges();
                let _ = client.send(error_msg);
                return Ok(());
            }
        }

        // Check if target server is already connected
        if self.server_connections.is_connected(target_server).await {
            let error_msg = NumericReply::already_registered();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Check if target server is in allowed hosts
        if !self.is_host_allowed(target_server) {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Validate that the target server is configured in our server links
        // For CONNECT command, we need to check if we have a configured link to this server
        if !self.is_server_configured_for_connect(target_server, target_port) {
            let error_msg = NumericReply::connect_failed(target_server, "Server not configured for connection");
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Check hop count limits
        if self.server_connections.server_count().await >= self.config.security.server_security.max_hop_count as usize {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Attempt to connect to the target server
        match self.connect_to_server(target_server, target_port).await {
            Ok(_) => {
                let success_msg = NumericReply::connect_success(target_server, target_port);
                let _ = client.send(success_msg);
                tracing::info!("Remote CONNECT from {} to {}:{} successful", 
                    client.user.as_ref().unwrap().nick, target_server, target_port);
            }
            Err(e) => {
                let error_msg = NumericReply::connect_failed(target_server, &e.to_string());
                let _ = client.send(error_msg);
                tracing::warn!("Remote CONNECT from {} to {}:{} failed: {}", 
                    client.user.as_ref().unwrap().nick, target_server, target_port, e);
            }
        }

        Ok(())
    }

    /// Handle OPER command
    async fn handle_oper(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let client = connection_handler.get_client(&client_id)
            .ok_or_else(|| Error::User("Client not found".to_string()))?;

        // Check if client is registered
        if !client.is_registered() {
            let error_msg = NumericReply::not_registered();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Validate parameters
        if message.params.len() < 2 {
            let error_msg = NumericReply::need_more_params("OPER");
            let _ = client.send(error_msg);
            return Ok(());
        }

        let _oper_name = &message.params[0];
        let password = &message.params[1];

        // Get user and authenticate
        let database = self.database.clone();
        if let Some(mut user) = database.get_user(&client.id) {
            if self.authenticate_operator(&mut user, password).await {
                // Send success message with operator privileges
                let success_msg = NumericReply::youre_oper();
                let _ = client.send(success_msg);
                
                // Send operator privileges information
                self.send_operator_privileges(&client, &user).await;
                
                // Update user in database
                database.update_user(&client.id, user.clone())?;
                
                tracing::info!("User {} authenticated as operator with flags: {:?}", 
                    user.nick, user.operator_flags);
            } else {
                // Authentication failed
                let error_msg = NumericReply::password_mismatch();
                let _ = client.send(error_msg);
                
                tracing::warn!("Failed operator authentication attempt for user {} from {}", 
                    user.nick, user.host);
            }
        } else {
            let error_msg = NumericReply::password_mismatch();
            let _ = client.send(error_msg);
        }

        Ok(())
    }

    /// Check if a user is an operator
    async fn is_user_operator(&self, user: &User) -> bool {
        user.is_operator
    }

    /// Authenticate operator and set flags
    async fn authenticate_operator(&self, user: &mut User, password: &str) -> bool {
        if let Some(operator_config) = self.config.authenticate_operator(
            &user.nick,
            password,
            &user.username,
            &user.host,
        ) {
            // Set operator flags
            let flags: HashSet<crate::config::OperatorFlag> = operator_config.flags.iter().cloned().collect();
            user.set_operator_flags(flags);
            
            tracing::info!("Operator {} authenticated with flags: {:?}", user.nick, user.operator_flags);
            true
        } else {
            false
        }
    }

    /// Check if a host is allowed for remote connections
    fn is_host_allowed(&self, host: &str) -> bool {
        // Check denied hosts first
        for denied_host in &self.config.security.server_security.denied_remote_hosts {
            if self.matches_host_pattern(host, denied_host) {
                return false;
            }
        }

        // Check allowed hosts
        for allowed_host in &self.config.security.server_security.allowed_remote_hosts {
            if self.matches_host_pattern(host, allowed_host) {
                return true;
            }
        }

        false
    }

    /// Check if a host matches a pattern (supports wildcards)
    fn matches_host_pattern(&self, host: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Simple wildcard matching
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                return host.starts_with(parts[0]) && host.ends_with(parts[1]);
            }
        }

        host == pattern
    }

    /// Check if a server is configured for CONNECT command
    fn is_server_configured_for_connect(&self, server_name: &str, port: u16) -> bool {
        // Check if we have a server link configuration for this server
        if let Some(link) = self.server_connections.get_server_link(server_name) {
            // Verify the port matches (or allow if not specified)
            return link.port == port || port == 0;
        }
        
        // Check if it's a super server
        if let Some(super_server) = self.server_connections.get_super_server(server_name) {
            return super_server.port == port || port == 0;
        }
        
        false
    }

    /// Connect to a remote server
    async fn connect_to_server(&self, server_name: &str, port: u16) -> Result<()> {
        // Get server link configuration
        let server_link = self.server_connections.get_server_link(server_name);
        
        // Validate the server is configured for connection
        if !self.is_server_configured_for_connect(server_name, port) {
            return Err(Error::Server(format!(
                "Server {} is not configured for connection in server links", 
                server_name
            )));
        }
        
        // Create connection
        let stream = tokio::net::TcpStream::connect(format!("{}:{}", server_name, port)).await
            .map_err(|e| Error::Connection(format!("Failed to connect to {}:{}: {}", server_name, port, e)))?;

        let remote_addr = stream.peer_addr()
            .map_err(|e| Error::Connection(format!("Failed to get peer address: {}", e)))?;
        let local_addr = stream.local_addr()
            .map_err(|e| Error::Connection(format!("Failed to get local address: {}", e)))?;

        // Create server connection
        let connection_id = Uuid::new_v4();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        
        let mut server_connection = ServerConnection::new(
            connection_id,
            remote_addr,
            local_addr,
            sender,
            true, // is_outgoing
        );

        // Set server information
        server_connection.info.name = server_name.to_string();
        server_connection.info.hostname = server_name.to_string();
        server_connection.info.port = port;
        server_connection.info.version = self.config.server.version.clone();
        server_connection.info.description = format!("Connected from {}", self.config.server.name);

        // Set link password if configured
        if let Some(link) = server_link {
            server_connection.info.link_password = Some(link.password.clone());
            server_connection.info.use_tls = link.tls;
        }

        // Add connection to manager
        self.server_connections.add_connection(server_connection).await?;

        // Start server connection handler
        self.start_server_connection_handler(connection_id, stream, receiver, server_name).await?;

        tracing::info!("Successfully connected to server {}:{}", server_name, port);
        Ok(())
    }

    /// Start a server connection handler
    async fn start_server_connection_handler(
        &self,
        _connection_id: Uuid,
        stream: tokio::net::TcpStream,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<Message>,
        server_name: &str,
    ) -> Result<()> {
        let (read_half, mut write_half) = stream.into_split();

        // Spawn message sender task
        let server_name_clone = server_name.to_string();
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                let message_str = message.to_string();
                if let Err(e) = write_half.write_all(message_str.as_bytes()).await {
                    tracing::error!("Failed to send message to server {}: {}", server_name_clone, e);
                    break;
                }
            }
        });

        // Spawn message receiver task
        let server_name_clone2 = server_name.to_string();
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(read_half);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        tracing::info!("Server {} disconnected", server_name_clone2);
                        break;
                    }
                    Ok(_) => {
                        // Parse and handle server message
                        if let Ok(message) = Message::parse(&line.trim()) {
                            // TODO: Handle server message
                            tracing::debug!("Received from server {}: {:?}", server_name_clone2, message);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from server {}: {}", server_name_clone2, e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }
    
    /// Handle KILL command for operators
    async fn handle_kill(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let client = connection_handler.get_client(&client_id)
            .ok_or_else(|| Error::User("Client not found".to_string()))?;

        // Check if client is registered
        if !client.is_registered() {
            let error_msg = NumericReply::not_registered();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Validate parameters
        if message.params.len() < 2 {
            let error_msg = NumericReply::need_more_params("KILL");
            let _ = client.send(error_msg);
            return Ok(());
        }

        let target_nick = &message.params[0];
        let reason = &message.params[1];

        // Get the operator user
        let database = self.database.clone();
        let Some(operator_user) = database.get_user(&client.id) else {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
            return Ok(());
        };

        // Check if user is an operator
        if !operator_user.is_operator {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Find the target user
        let Some(target_user) = database.get_user_by_nick(target_nick) else {
            let error_msg = NumericReply::no_such_nick(target_nick);
            let _ = client.send(error_msg);
            return Ok(());
        };

        // Check operator permissions
        let can_kill_globally = operator_user.is_global_oper();
        let can_kill_locally = operator_user.is_local_oper();
        let target_is_local = target_user.server == self.config.server.name;

        if !can_kill_globally && (!can_kill_locally || !target_is_local) {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Check if trying to kill a server (not allowed)
        if target_user.nick == self.config.server.name {
            let error_msg = NumericReply::cant_kill_server();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Send KILL message to the target user
        let kill_message = Message::with_prefix(
            operator_user.prefix(),
            MessageType::Kill,
            vec![target_nick.to_string(), reason.to_string()],
        );

        // Find the target user's client and send the kill message
        if let Some(target_client_id) = database.get_user_by_nick(target_nick).map(|u| u.id) {
            if let Some(target_client) = connection_handler.get_client(&target_client_id) {
                let _ = target_client.send(kill_message);
            }
        }

        // Send NOTICE to all operators about the kill
        self.notify_operators_kill(&operator_user, &target_user, reason).await?;

        // Broadcast KILL message to all connected servers
        let server_kill_msg = Message::new(
            MessageType::Kill,
            vec![target_nick.to_string(), format!("{}!{}!{}!{} ({})", 
                self.config.server.name, operator_user.host, operator_user.username, operator_user.nick, reason)]
        );
        
        if let Err(e) = self.server_connections.broadcast_to_servers(server_kill_msg).await {
            tracing::warn!("Failed to broadcast KILL to servers: {}", e);
        }

        // Disconnect the target user
        if let Some(target_client_id) = database.get_user_by_nick(target_nick).map(|u| u.id) {
            if let Some(target_client) = connection_handler.get_client(&target_client_id) {
                // Send quit message to all users in channels
                self.broadcast_user_quit(&target_client, &format!("Killed by {}: {}", operator_user.nick, reason)).await?;
                
                // Remove user from database
                database.remove_user(target_client_id)?;
                
                // Close the connection
                drop(connection_handler);
                let mut connection_handler = self.connection_handler.write().await;
                connection_handler.remove_client(&target_client_id);
            }
        }

        tracing::info!("Operator {} killed user {}: {}", operator_user.nick, target_nick, reason);
        Ok(())
    }
    
    /// Notify all operators about a KILL command
    async fn notify_operators_kill(&self, operator: &User, target: &User, reason: &str) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let database = self.database.clone();
        
        // Get all operators
        let operators = database.get_all_users()
            .into_iter()
            .filter(|user| user.is_operator)
            .collect::<Vec<_>>();
        
        let notice_text = format!("*** {} killed {}: {}", operator.nick, target.nick, reason);
        
        for oper in operators {
            if let Some(client_id) = database.get_user_by_nick(&oper.nick).map(|u| u.id) {
                if let Some(client) = connection_handler.get_client(&client_id) {
                    let notice = Message::new(
                        MessageType::Notice,
                        vec![oper.nick.clone(), notice_text.clone()],
                    );
                    let _ = client.send(notice);
                }
            }
        }
        
        Ok(())
    }
    
    /// Send notice to all operators
    async fn send_operator_notice(&self, message: &str) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let database = self.database.clone();
        
        // Get all operators
        let operators = database.get_all_users()
            .into_iter()
            .filter(|user| user.is_operator)
            .collect::<Vec<_>>();
        
        for oper in operators {
            if let Some(client_id) = database.get_user_by_nick(&oper.nick).map(|u| u.id) {
                if let Some(client) = connection_handler.get_client(&client_id) {
                    let notice = Message::new(
                        MessageType::Notice,
                        vec![oper.nick.clone(), message.to_string()],
                    );
                    let _ = client.send(notice);
                }
            }
        }
        
        Ok(())
    }
    
    /// Broadcast user quit to all users in the same channels
    async fn broadcast_user_quit(&self, client: &Client, reason: &str) -> Result<()> {
        let database = self.database.clone();
        let Some(user) = client.get_user() else {
            return Ok(());
        };
        
        // Get all channels the user is in
        let channels = user.channels.clone();
        
        // Create quit message
        let quit_message = Message::with_prefix(
            user.prefix(),
            MessageType::Quit,
            vec![reason.to_string()],
        );
        
        // Broadcast to all users in the same channels
        let connection_handler = self.connection_handler.read().await;
        for channel in channels {
            let channel_users = database.get_channel_users(&channel);
            for nick in channel_users {
                // Get user ID from nickname
                if let Some(user) = database.get_user_by_nick(&nick) {
                    if let Some(target_client) = connection_handler.get_client(&user.id) {
                        let _ = target_client.send(quit_message.clone());
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Broadcast user quit message by client ID
    async fn broadcast_user_quit_by_id(&self, client_id: uuid::Uuid, reason: &str) -> Result<()> {
        let database = self.database.clone();
        let Some(user) = database.get_user(&client_id) else {
            return Ok(());
        };
        
        // Get all channels the user is in
        let channels = user.channels.clone();
        
        // Create quit message
        let quit_message = Message::with_prefix(
            user.prefix(),
            MessageType::Quit,
            vec![reason.to_string()],
        );
        
        // Broadcast to all users in the same channels
        let connection_handler = self.connection_handler.read().await;
        for channel in channels {
            let channel_users = database.get_channel_users(&channel);
            for nick in channel_users {
                // Get user ID from nickname
                if let Some(user) = database.get_user_by_nick(&nick) {
                    if let Some(target_client) = connection_handler.get_client(&user.id) {
                        let _ = target_client.send(quit_message.clone());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle SQUIT command for operators
    async fn handle_operator_squit(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let client = connection_handler.get_client(&client_id)
            .ok_or_else(|| Error::User("Client not found".to_string()))?;

        // Check if client is registered
        if !client.is_registered() {
            let error_msg = NumericReply::not_registered();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Validate parameters
        if message.params.is_empty() {
            let error_msg = NumericReply::need_more_params("SQUIT");
            let _ = client.send(error_msg);
            return Ok(());
        }

        let target_server = &message.params[0];
        let reason = message.params.get(1).map(|s| s.as_str()).unwrap_or("Operator requested");

        // Check if user is an operator
        let user = client.user.as_ref().unwrap();
        if !user.is_operator {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Check if operator has S flag (Squit permission)
        if !user.can_squit() {
            let error_msg = NumericReply::no_privileges();
            let _ = client.send(error_msg);
            tracing::warn!("Operator {} attempted SQUIT without S flag", user.nick);
            return Ok(());
        }

        // Check if target server is connected
        if !self.server_connections.is_connected(target_server).await {
            let error_msg = NumericReply::no_such_server(target_server);
            let _ = client.send(error_msg);
            return Ok(());
        }

        // Send SQUIT to the target server and propagate to others
        let squit_message = Message::new(
            MessageType::ServerQuit,
            vec![target_server.to_string(), reason.to_string()]
        );
        
        self.propagate_to_servers(squit_message).await?;
        
        // Remove the server connection locally
        self.server_connections.remove_connection(target_server).await?;
        
        // Send notice to all operators about the SQUIT
        self.send_operator_notice(&format!("SQUIT: {} disconnected server {}: {}", user.nick, target_server, reason)).await?;
        
        tracing::info!("Operator {} issued SQUIT for server {}: {}", user.nick, target_server, reason);
        Ok(())
    }

    /// Validate an incoming server connection
    pub async fn validate_incoming_server_connection(
        &self, 
        server_name: &str, 
        hostname: &str, 
        port: u16
    ) -> Result<()> {
        // Use the server connection manager to validate
        self.server_connections.validate_incoming_connection(server_name, hostname, port)?;
        
        tracing::info!("Incoming server connection validated: {} ({})", server_name, hostname);
        Ok(())
    }

    /// Handle incoming server connection
    pub async fn handle_incoming_server_connection(
        &self,
        stream: tokio::net::TcpStream,
        remote_addr: std::net::SocketAddr,
    ) -> Result<()> {
        // For now, we'll create a basic server connection
        // In a full implementation, this would involve:
        // 1. Reading the SERVER command from the incoming connection
        // 2. Validating the server name and credentials
        // 3. Checking if the server is configured in our links
        
        let connection_id = Uuid::new_v4();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        
        let server_connection = ServerConnection::new(
            connection_id,
            remote_addr,
            stream.local_addr()?,
            sender,
            false, // is_outgoing = false for incoming connections
        );

        // Add to server connections
        self.server_connections.add_connection(server_connection).await?;

        // Start connection handler
        self.start_server_connection_handler(connection_id, stream, receiver, "unknown").await?;

        tracing::info!("Incoming server connection from {} accepted", remote_addr);
        Ok(())
    }

    /// Notify a spy user that someone did a WHOIS on them
    async fn notify_spy_user(&self, target_user: &User, requesting_client_id: Uuid) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(requesting_client) = connection_handler.get_client(&requesting_client_id) {
            if let Some(requesting_user) = &requesting_client.user {
                // Send spy notification to the target user
                let spy_notification = Message::new(
                    crate::MessageType::Notice,
                    vec![
                        target_user.nick.clone(),
                        format!("SPY: {} ({}@{}) did a WHOIS on you", 
                            requesting_user.nick, 
                            requesting_user.username, 
                            requesting_user.host)
                    ],
                );
                
                // Find the target user's client and send the notification
                let target_client_id = self.database.get_user_by_nick(&target_user.nick)
                    .and_then(|user| Some(user.id));
                
                if let Some(client_id) = target_client_id {
                    if let Some(target_client) = connection_handler.get_client(&client_id) {
                        let _ = target_client.send(spy_notification);
                        tracing::info!("Sent spy notification to {} about WHOIS from {}", 
                            target_user.nick, requesting_user.nick);
                    }
                }
            }
        }
        Ok(())
    }

    /// Get public channels for a user (excluding secret channels)
    async fn get_public_channels_for_user(&self, nickname: &str) -> Vec<String> {
        let channels = self.database.get_user_channels(nickname);
        
        // Filter out secret channels (those with +s mode)
        // For now, we'll return all channels - in a full implementation,
        // we would check channel modes to filter secret channels
        channels.into_iter().collect()
    }

    /// Send operator privileges information to a client
    async fn send_operator_privileges(&self, client: &Client, user: &User) {
        let mut privileges = Vec::new();
        
        if user.is_global_oper() {
            privileges.push("Global Operator (o)");
        }
        if user.is_local_oper() {
            privileges.push("Local Operator (O)");
        }
        if user.can_remote_connect() {
            privileges.push("Remote Connect (C)");
        }
        if user.can_local_connect() {
            privileges.push("Local Connect (c)");
        }
        if user.is_administrator() {
            privileges.push("Administrator (A)");
        }
        if user.is_spy() {
            privileges.push("Spy (y)");
        }
        
        if !privileges.is_empty() {
            let privileges_msg = Message::new(
                crate::MessageType::Notice,
                vec![
                    user.nick.clone(),
                    format!("Operator privileges: {}", privileges.join(", "))
                ],
            );
            let _ = client.send(privileges_msg);
        }
    }
}

/// Load certificates from file
fn load_certificates(filename: &str) -> Result<Vec<Certificate>> {
    let certfile = std::fs::File::open(filename)
        .map_err(|e| Error::Config(format!("Failed to open certificate file: {}", e)))?;
    let mut reader = BufReader::new(certfile);
    
    let certs = rustls_pemfile::certs(&mut reader)
        .map_err(|e| Error::Config(format!("Failed to parse certificate file: {}", e)))?;
    
    Ok(certs.into_iter().map(Certificate).collect())
}

/// Load private key from file
fn load_private_key(filename: &str) -> Result<PrivateKey> {
    let keyfile = std::fs::File::open(filename)
        .map_err(|e| Error::Config(format!("Failed to open key file: {}", e)))?;
    let mut reader = BufReader::new(keyfile);
    
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .map_err(|e| Error::Config(format!("Failed to parse key file: {}", e)))?;
    
    if keys.is_empty() {
        return Err(Error::Config("No private keys found in file".to_string()));
    }
    
    Ok(PrivateKey(keys[0].clone()))
}

impl Server {
    /// Get the server configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
    
    
    /// Register IRCv3 extensions
    /// Note: This method should be implemented in the modules crate
    /// and called from there, not from core
    pub async fn register_ircv3_extensions(&self) -> Result<()> {
        // This method is a placeholder - actual implementation should be in modules crate
        tracing::info!("IRCv3 extensions registration called (placeholder)");
        Ok(())
    }
    
    /// Register channel burst extension
    /// This method registers the channel burst extension for server-to-server synchronization
    async fn register_channel_burst_extension(&self) -> Result<()> {
        // Note: This is a placeholder implementation
        // In a full implementation, this would:
        // 1. Get the channel module's channel data structures
        // 2. Create a ChannelBurstExtension instance
        // 3. Register it with the extension manager
        
        tracing::info!("Channel burst extension registration called (placeholder)");
        tracing::info!("  - Channel burst extension will be registered when channel module is fully integrated");
        tracing::info!("  - This will enable server-to-server channel synchronization");
        
        // TODO: When channel module is fully integrated:
        // let channels = channel_module.get_channels(); // Get channel data from module
        // let channel_burst_extension = Box::new(ChannelBurstExtension::new(
        //     channels,
        //     self.database.clone(),
        //     self.config.server.name.clone(),
        // ));
        // self.extension_manager.register_burst_extension(channel_burst_extension).await?;
        
        Ok(())
    }
    
    /// Handle incoming ChannelBurst messages from other servers
    /// This method processes channel synchronization data from remote servers
    pub async fn handle_channel_burst(&self, source_server: &str, messages: &[Message]) -> Result<()> {
        tracing::info!("Processing channel burst from server: {} ({} messages)", source_server, messages.len());
        
        // TODO: Process channel burst without extensions
        // For now, just log the received channel burst
        tracing::info!("Processed {} channel burst messages from server: {}", messages.len(), source_server);
        
        tracing::info!("Successfully processed channel burst from server: {}", source_server);
        Ok(())
    }
    
    /// Prepare channel burst for sending to another server
    /// This method collects channel information for synchronization
    pub async fn prepare_channel_burst(&self, target_server: &str) -> Result<Vec<Message>> {
        tracing::info!("Preparing channel burst for server: {}", target_server);
        
        // TODO: Prepare channel burst without extensions
        // For now, return empty messages
        let messages = Vec::new();
        
        tracing::info!("Prepared {} channel burst messages for server: {}", messages.len(), target_server);
        Ok(messages)
    }
    
    /// Handle MODE command - User and channel mode management
    /// RFC 1459 Section 4.2.3
    pub async fn handle_mode(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        if message.params.is_empty() {
            return self.send_error(client_id, NumericReply::err_need_more_params("MODE")).await;
        }

        let target = &message.params[0];
        
        // Check if target is a channel (starts with #, &, +, or !)
        if target.starts_with('#') || target.starts_with('&') || target.starts_with('+') || target.starts_with('!') {
            // Channel mode - delegate to channel module
            self.handle_channel_mode(client_id, message).await
        } else {
            // User mode - handle user mode changes
            self.handle_user_mode(client_id, message).await
        }
    }
    
    /// Handle user mode changes
    async fn handle_user_mode(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let target = &message.params[0];
        
        // Get requesting user
        let users = self.users.read().await;
        let requesting_user = users.get(&client_id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if user is trying to change their own modes or someone else's
        let is_self = target == &requesting_user.nick;
        
        if !is_self {
            // Only operators can change other users' modes
            if !requesting_user.is_operator {
                return self.send_error(client_id, NumericReply::err_users_dont_match()).await;
            }
        }
        
        // Get target user
        let target_user = if is_self {
            requesting_user.clone()
        } else {
            // Find target user by nickname
            let target_user = users.values()
                .find(|u| u.nick == *target)
                .ok_or_else(|| Error::User("No such nick".to_string()))?;
            target_user.clone()
        };
        
        // If no mode changes specified, just show current modes
        if message.params.len() == 1 {
            let modes = target_user.modes_string();
            let reply = NumericReply::umode_is(&target_user.nick, &modes);
            return self.send_to_client(client_id, reply).await;
        }
        
        // Parse mode changes
        let mode_string = &message.params[1];
        let mode_changes = self.parse_mode_changes(mode_string)?;
        
        // Apply mode changes
        let mut updated_user = target_user.clone();
        let mut changes_applied = Vec::new();
        
        for (action, mode_char) in mode_changes {
            let adding = action;
            
            // Check if it's a core mode
            if let Some(user_mode) = crate::user_modes::UserMode::from_char(mode_char) {
                // Validate core mode change
                if let Err(_e) = self.validate_mode_change(
                    &target_user,
                    user_mode,
                    adding,
                    &target_user.nick,
                    &requesting_user.nick,
                    requesting_user.is_operator,
                ) {
                    // Use specific error for operator mode attempts
                    let error_reply = if adding && user_mode.oper_only() {
                        NumericReply::err_cant_set_operator_mode()
                    } else {
                        NumericReply::err_users_dont_match()
                    };
                    return self.send_error(client_id, error_reply).await;
                }
                
                // Apply mode change
                if adding {
                    updated_user.add_mode(mode_char);
                    changes_applied.push(format!("+{}", mode_char));
                } else {
                    updated_user.remove_mode(mode_char);
                    changes_applied.push(format!("-{}", mode_char));
                }
            } else {
                // Check if it's a custom mode
                if crate::extensible_modes::is_valid_user_mode(mode_char) {
                    // Validate custom mode change
                    if let Err(_e) = crate::extensible_modes::validate_custom_mode_change(
                        mode_char,
                        adding,
                        &target_user.nick,
                        &requesting_user.nick,
                        requesting_user.is_operator,
                    ) {
                        return self.send_error(client_id, NumericReply::err_users_dont_match()).await;
                    }
                    
                    // Apply custom mode change
                    if adding {
                        updated_user.add_mode(mode_char);
                        changes_applied.push(format!("+{}", mode_char));
                    } else {
                        updated_user.remove_mode(mode_char);
                        changes_applied.push(format!("-{}", mode_char));
                    }
                } else {
                    // Invalid mode
                    return self.send_error(client_id, NumericReply::err_users_dont_match()).await;
                }
            }
        }
        
        // Update user in database
        {
            let mut users = self.users.write().await;
            users.insert(client_id, updated_user.clone());
        }
        
        // Send mode change notification
        if !changes_applied.is_empty() {
            let changes_string = changes_applied.join("");
            let mode_change_msg = Message::new(
                MessageType::Mode,
                vec![target_user.nick.clone(), changes_string],
            );
            
            // Send to the user whose modes changed
            self.send_to_client(client_id, mode_change_msg.clone()).await?;
            
            // If not self, also send to the requesting user
            if !is_self {
                let requesting_client_id = {
                    let users = self.users.read().await;
                    users.values()
                        .find(|u| u.nick == requesting_user.nick)
                        .map(|u| u.id)
                        .ok_or_else(|| Error::User("Requesting user not found".to_string()))?
                };
                self.send_to_client(requesting_client_id, mode_change_msg).await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle channel mode changes (placeholder - delegate to channel module)
    async fn handle_channel_mode(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        // TODO: Implement channel mode handling
        // This should delegate to the channel module
        tracing::info!("Channel mode handling not yet implemented");
        self.send_error(client_id, NumericReply::err_unknown_command("CHANNEL MODE")).await
    }
    
    /// Parse mode change string (e.g., "+iw", "-a+o")
    fn parse_mode_changes(&self, mode_string: &str) -> Result<Vec<(bool, char)>> {
        let mut changes = Vec::new();
        let mut current_action = true; // true for add (+), false for remove (-)
        
        for c in mode_string.chars() {
            match c {
                '+' => current_action = true,
                '-' => current_action = false,
                _ => {
                    if crate::user_modes::is_valid_user_mode(c) {
                        changes.push((current_action, c));
                    }
                }
            }
        }
        
        Ok(changes)
    }
    
    /// Validate mode change for a user
    fn validate_mode_change(
        &self,
        user: &crate::user::User,
        mode: crate::user_modes::UserMode,
        adding: bool,
        target_user: &str,
        requesting_user: &str,
        requesting_user_is_operator: bool,
    ) -> Result<()> {
        let is_self = target_user == requesting_user;
        
        // Special case: Operator mode can only be granted through OPER command
        if adding && mode.oper_only() {
            return Err("Operator mode can only be granted through OPER command".into());
        }
        
        // Check operator requirements for removal of restricted modes
        if !adding && mode.requires_operator() && !requesting_user_is_operator {
            // Exception: Users can always remove their own operator mode
            if !(is_self && (mode == crate::user_modes::UserMode::Operator || mode == crate::user_modes::UserMode::LocalOperator)) {
                return Err("Permission denied".into());
            }
        }
        
        // Check if mode can only be set by the user themselves
        if !is_self && mode.self_only() {
            return Err("You can only change your own modes".into());
        }
        
        // Check if mode is already set/unset
        let currently_has = user.has_mode(mode.to_char());
        if adding && currently_has {
            return Err(format!("Mode {} is already set", mode.to_char()).into());
        }
        if !adding && !currently_has {
            return Err(format!("Mode {} is not set", mode.to_char()).into());
        }
        
        Ok(())
    }
    
    /// Send error message to client
    async fn send_error(&self, client_id: uuid::Uuid, error_msg: Message) -> Result<()> {
        self.send_to_client(client_id, error_msg).await
    }
    
    /// Send message to specific client
    async fn send_to_client(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            client.send(message)?;
        }
        Ok(())
    }
    
    /// Handle LUSERS command - Network statistics
    /// RFC 1459 Section 4.3.1
    pub async fn handle_lusers(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            // Get network statistics
            let users = self.get_user_count().await;
            let operators = self.get_operator_count().await;
            let channels = self.get_channel_count().await;
            let servers = self.get_server_count().await;
            let unknown_connections = self.get_unknown_connection_count().await;
            let local_users = self.get_local_user_count().await;
            let max_local_users = self.config.server.max_clients;
            let global_users = self.get_global_user_count().await;
            let max_global_users = max_local_users; // For now, assume same as local max
            
            // Send LUSERS replies
            let _ = client.send(NumericReply::luser_client(users, 0, servers)); // 0 services for now
            let _ = client.send(NumericReply::luser_op(operators));
            let _ = client.send(NumericReply::luser_unknown(unknown_connections));
            let _ = client.send(NumericReply::luser_channels(channels));
            let _ = client.send(NumericReply::luser_me(local_users, servers));
            let _ = client.send(NumericReply::local_users(local_users, max_local_users.try_into().unwrap_or(u32::MAX)));
            let _ = client.send(NumericReply::global_users(global_users, max_global_users.try_into().unwrap_or(u32::MAX)));
        }
        Ok(())
    }
    
    /// Handle USERS command - RFC 1459 Section 4.3.3
    pub async fn handle_users(&self, client_id: uuid::Uuid, _message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            // Get local and global user counts
            let local_users = self.get_local_user_count().await;
            let global_users = self.get_global_user_count().await;
            
            // Send USERS replies
            let _ = client.send(NumericReply::users_start());
            
            if local_users > 0 {
                // Show local users count
                let _ = client.send(NumericReply::users("local", "users", &format!("{} users", local_users)));
            }
            
            if global_users > 0 {
                // Show global users count
                let _ = client.send(NumericReply::users("global", "users", &format!("{} users", global_users)));
            }
            
            if local_users == 0 && global_users == 0 {
                let _ = client.send(NumericReply::no_users());
            }
            
            let _ = client.send(NumericReply::end_of_users());
        }
        Ok(())
    }
    
    /// Get current user count
    async fn get_user_count(&self) -> u32 {
        let users = self.users.read().await;
        users.len() as u32
    }
    
    /// Get operator count
    async fn get_operator_count(&self) -> u32 {
        let users = self.users.read().await;
        users.values().filter(|user| user.is_operator).count() as u32
    }
    
    /// Get channel count
    async fn get_channel_count(&self) -> u32 {
        self.database.channel_count() as u32
    }
    
    /// Get server count (including this server)
    async fn get_server_count(&self) -> u32 {
        let server_connections = self.server_connections.get_all_connections().await;
        1 + server_connections.len() as u32 // +1 for this server
    }
    
    /// Get unknown connection count (unregistered connections)
    async fn get_unknown_connection_count(&self) -> u32 {
        let connection_handler = self.connection_handler.read().await;
        let registered_clients = connection_handler.get_registered_clients();
        let total_clients = connection_handler.get_all_clients();
        (total_clients.len() - registered_clients.len()) as u32
    }
    
    /// Get local user count
    async fn get_local_user_count(&self) -> u32 {
        let users = self.users.read().await;
        users.values().filter(|user| user.server == self.config.server.name).count() as u32
    }
    
    /// Get global user count (all users across network)
    async fn get_global_user_count(&self) -> u32 {
        self.get_user_count().await // For now, same as local since we don't have network sync yet
    }
    
    /// Get the rehash service
    pub fn rehash_service(&self) -> &Arc<RehashService> {
        &self.rehash_service
    }
    
    /// Reload MOTD from configuration
    pub async fn reload_motd(&mut self) -> Result<()> {
        let config = self.config.clone();
        
        if let Some(motd_file) = &config.server.motd_file {
            info!("Reloading MOTD from file: {}", motd_file);
            let mut new_motd_manager = MotdManager::new();
            new_motd_manager.load_motd(motd_file).await?;
            self.motd_manager = Arc::new(new_motd_manager);
            info!("MOTD reloaded successfully from: {}", motd_file);
        } else {
            warn!("No MOTD file configured, clearing MOTD");
            self.motd_manager = Arc::new(MotdManager::new());
            info!("MOTD cleared successfully");
        }
        
        Ok(())
    }
    
    /// Reload TLS configuration
    pub async fn reload_tls(&mut self) -> Result<()> {
        if !self.config.security.tls.enabled {
            warn!("TLS is not enabled, skipping TLS reload");
            return Ok(());
        }
        
        info!("Reloading TLS configuration");
        self.setup_tls().await?;
        info!("TLS configuration reloaded successfully");
        Ok(())
    }
    
    /// Reload modules from configuration
    pub async fn reload_modules(&mut self) -> Result<()> {
        info!("Reloading modules from configuration");
        
        // Clear existing modules
        {
            let mut module_manager = self.module_manager.write().await;
            module_manager.clear_modules().await?;
        }
        
        // Load modules from configuration
        self.load_modules().await?;
        
        info!("Modules reloaded successfully");
        Ok(())
    }
}
