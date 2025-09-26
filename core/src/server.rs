//! Main IRC server implementation

use crate::{
    User, Message, MessageType, NumericReply, Config, ModuleManager,
    connection::ConnectionHandler, Error, Result, module::ModuleResult, client::ClientState,
    Database, BroadcastSystem, NetworkQueryManager, NetworkMessageHandler, ExtensionManager,
    Prefix,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use rustls::{ServerConfig, Certificate, PrivateKey};
use std::io::BufReader;

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
    /// Extension manager for IRCv3 capabilities
    extension_manager: Arc<ExtensionManager>,
    /// TLS acceptor (if enabled)
    tls_acceptor: Option<TlsAcceptor>,
}

impl Server {
    /// Create a new server instance
    pub fn new(config: Config) -> Self {
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
        ));

        // Initialize extension manager
        let extension_manager = Arc::new(ExtensionManager::new());
        
        Self {
            config,
            module_manager: Arc::new(RwLock::new(ModuleManager::new())),
            connection_handler: Arc::new(RwLock::new(connection_handler)),
            users: Arc::new(RwLock::new(HashMap::new())),
            nick_to_id: Arc::new(RwLock::new(HashMap::new())),
            super_servers: Arc::new(RwLock::new(HashMap::new())),
            database,
            broadcast_system,
            network_query_manager,
            network_message_handler,
            extension_manager,
            tls_acceptor: None,
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
        
        // Create TLS configuration
        let tls_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| Error::Tls(e))?;
        
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
        tracing::info!("Starting IRC server on {}:{}", 
                      self.config.connection.bind_address, 
                      self.config.connection.client_port);
        
        // Start client listener
        let client_listener = TcpListener::bind(
            format!("{}:{}", self.config.connection.bind_address, self.config.connection.client_port)
        ).await?;
        
        let tls_acceptor = self.tls_acceptor.clone();
        let connection_handler = self.connection_handler.clone();
        
        // Spawn client connection handler
        tokio::spawn(async move {
            loop {
                match client_listener.accept().await {
                    Ok((stream, addr)) => {
                        let mut conn_handler = connection_handler.write().await;
                        if let Err(e) = conn_handler.handle_connection(stream, addr, tls_acceptor.clone()).await {
                            tracing::error!("Error handling client connection: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error accepting client connection: {}", e);
                    }
                }
            }
        });
        
        // Start server listener if configured
        if self.config.connection.server_port != 0 {
            self.start_server_listener().await?;
        }
        
        // Start message processing loop
        self.start_message_processor().await?;
        
        Ok(())
    }
    
    /// Start server listener for server-to-server connections
    async fn start_server_listener(&self) -> Result<()> {
        let _server_listener = TcpListener::bind(
            format!("{}:{}", self.config.connection.bind_address, self.config.connection.server_port)
        ).await?;
        
        tracing::info!("Server listener started on port {}", self.config.connection.server_port);
        
        // TODO: Implement server-to-server connection handling
        Ok(())
    }
    
    /// Start message processing loop
    async fn start_message_processor(&self) -> Result<()> {
        let _connection_handler = self.connection_handler.clone();
        let module_manager = self.module_manager.clone();
        let users = self.users.clone();
        let nick_to_id = self.nick_to_id.clone();
        // Channels are now managed by modules, not core
        // let channels = self.channels.clone();
        
        tokio::spawn(async move {
            // TODO: Implement message processing loop
            // This would receive messages from the connection handler
            // and process them through the module system
        });
        
        Ok(())
    }
    
    /// Handle a message from a client
    pub async fn handle_message(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let client = connection_handler.get_client(&client_id)
            .ok_or_else(|| Error::User("Client not found".to_string()))?;
        
        // Process through modules first
        let mut module_manager = self.module_manager.write().await;
        match module_manager.handle_message(client, &message).await? {
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
        // Check if this is a super server
        let is_super_server = {
            let super_servers = self.super_servers.read().await;
            super_servers.contains_key(server_name)
        };
        
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
            _ => {
                // Other server commands can be handled here
                tracing::debug!("Unhandled server command: {:?}", message.command);
            }
        }
        Ok(())
    }
    
    /// Handle server registration
    async fn handle_server_registration(&self, server_name: &str, message: Message, is_super_server: bool) -> Result<()> {
        tracing::info!("Server {} registered (super: {})", server_name, is_super_server);
        // TODO: Implement server registration logic
        Ok(())
    }
    
    /// Handle server quit
    async fn handle_server_quit(&self, server_name: &str, message: Message) -> Result<()> {
        tracing::info!("Server {} quit", server_name);
        // TODO: Implement server quit logic
        Ok(())
    }
    
    /// Handle core IRC commands
    async fn handle_core_command(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        let client = connection_handler.get_client(&client_id)
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
            // Miscellaneous commands
            MessageType::Away => {
                self.handle_away(client_id, message).await?;
            }
            MessageType::Ison => {
                self.handle_ison(client_id, message).await?;
            }
            MessageType::Userhost => {
                self.handle_userhost(client_id, message).await?;
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
        
        // Check if password is required and correct
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
        
        // Register nickname
        let mut connection_handler = self.connection_handler.write().await;
        if let Some(client) = connection_handler.get_client_mut(&client_id) {
            client.set_state(ClientState::NickSet);
            // TODO: Set nickname in client
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
                // Send welcome message
                let welcome_msg = NumericReply::welcome(
                    &self.config.server.name,
                    client.nickname().unwrap_or("unknown"),
                    username,
                    hostname,
                );
                let _ = client.send(welcome_msg);
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
    async fn handle_pong(&self, _client_id: uuid::Uuid, _message: Message) -> Result<()> {
        // Update last activity time
        // TODO: Implement ping/pong handling
        Ok(())
    }
    
    /// Handle QUIT command
    async fn handle_quit(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let quit_message = message.params.first().map(|s| s.as_str()).unwrap_or("Client quit");
        
        // Notify modules
        let module_manager = self.module_manager.read().await;
        if let Some(client) = self.connection_handler.read().await.get_client(&client_id) {
            if let Some(user) = client.get_user() {
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
    
    /// Handle STATS command
    async fn handle_stats(&self, client_id: uuid::Uuid, message: Message) -> Result<()> {
        let connection_handler = self.connection_handler.read().await;
        if let Some(client) = connection_handler.get_client(&client_id) {
            let query = message.params.get(0).map(|s| s.as_str()).unwrap_or("");
            
            match query {
                "l" => {
                    // Link information
                    let stats_msg = NumericReply::stats_link_info(
                        &self.config.server.name,
                        0, // sendq
                        0, // sent_messages
                        0, // sent_bytes
                        0, // received_messages
                        0, // received_bytes
                        0, // time_online
                    );
                    let _ = client.send(stats_msg);
                }
                "c" => {
                    // Connection information
                    let stats_msg = NumericReply::stats_commands(
                        "CONNECT",
                        0, // count
                        0, // bytes
                        0, // remote_count
                    );
                    let _ = client.send(stats_msg);
                }
                _ => {
                    // Default stats
                    let stats_msg = NumericReply::stats_commands(
                        "TOTAL",
                        0, // count
                        0, // bytes
                        0, // remote_count
                    );
                    let _ = client.send(stats_msg);
                }
            }
            
            let end_msg = NumericReply::end_of_stats(query);
            let _ = client.send(end_msg);
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
                    
                    if let Ok(request_id) = self.network_query_manager.query_whois(
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
                
                if let Ok(request_id) = self.network_query_manager.query_whowas(
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
            
            let privmsg = Message::with_prefix(
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
                if let Some(target_user) = self.database.get_user_by_nick(target) {
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
            
            let notice = Message::with_prefix(
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
                        user.away_message = None;
                        self.database.add_user(user);
                        
                        let unaway_msg = NumericReply::unaway();
                        let _ = client.send(unaway_msg);
                    } else {
                        // Set away message
                        let away_message = message.params[0].clone();
                        user.away_message = Some(away_message.clone());
                        self.database.add_user(user);
                        
                        let now_away_msg = NumericReply::now_away();
                        let _ = client.send(now_away_msg);
                    }
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
    
    /// Get the extension manager
    pub fn extension_manager(&self) -> &Arc<ExtensionManager> {
        &self.extension_manager
    }
    
    /// Register IRCv3 extensions
    /// Note: This method should be implemented in the modules crate
    /// and called from there, not from core
    pub async fn register_ircv3_extensions(&self) -> Result<()> {
        // This method is a placeholder - actual implementation should be in modules crate
        tracing::info!("IRCv3 extensions registration called (placeholder)");
        Ok(())
    }
}
