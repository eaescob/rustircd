//! Server-to-server connection management

use crate::{Error, Result, Message, Config};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

/// Server connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ServerConnectionState {
    /// Just connected, not registered
    Connected,
    /// Password provided
    PasswordProvided,
    /// Server registered
    Registered,
    /// Connection lost
    Disconnected,
}

/// Server information
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server hostname
    pub hostname: String,
    /// Server port
    pub port: u16,
    /// Server version
    pub version: String,
    /// Server description
    pub description: String,
    /// Connection time
    pub connected_at: DateTime<Utc>,
    /// Whether this is a super server (u-lined)
    pub is_super_server: bool,
    /// Link password (for outgoing connections)
    pub link_password: Option<String>,
    /// Whether to use TLS
    pub use_tls: bool,
    /// Whether this is an outgoing connection
    pub is_outgoing: bool,
    /// Server hop count
    pub hop_count: u8,
    /// Parent server name (for tree structure)
    pub parent_server: Option<String>,
    /// Child servers
    pub child_servers: Vec<String>,
}

/// Server connection
#[derive(Debug, Clone)]
pub struct ServerConnection {
    /// Connection ID
    pub id: Uuid,
    /// Server information
    pub info: ServerInfo,
    /// Connection state
    pub state: ServerConnectionState,
    /// Remote address
    pub remote_addr: SocketAddr,
    /// Local address
    pub local_addr: SocketAddr,
    /// Message sender for sending messages to server
    pub sender: mpsc::UnboundedSender<Message>,
    /// Whether connection is encrypted
    pub encrypted: bool,
    /// Last ping time
    pub last_ping: Option<DateTime<Utc>>,
    /// Last pong time
    pub last_pong: Option<DateTime<Utc>>,
    /// Connection statistics
    pub stats: ServerConnectionStats,
}

/// Server connection statistics
#[derive(Debug, Clone)]
pub struct ServerConnectionStats {
    /// Bytes received
    pub bytes_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Connection start time
    pub connected_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
}

impl Default for ServerConnectionStats {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            bytes_received: 0,
            bytes_sent: 0,
            messages_received: 0,
            messages_sent: 0,
            connected_at: now,
            last_activity: now,
        }
    }
}

impl ServerConnection {
    /// Create a new server connection
    pub fn new(
        id: Uuid,
        remote_addr: SocketAddr,
        local_addr: SocketAddr,
        sender: mpsc::UnboundedSender<Message>,
        is_outgoing: bool,
    ) -> Self {
        Self {
            id,
            info: ServerInfo {
                name: String::new(),
                hostname: remote_addr.ip().to_string(),
                port: remote_addr.port(),
                version: String::new(),
                description: String::new(),
                connected_at: Utc::now(),
                is_super_server: false,
                link_password: None,
                use_tls: false,
                is_outgoing,
                hop_count: 1,
                parent_server: None,
                child_servers: Vec::new(),
            },
            state: ServerConnectionState::Connected,
            remote_addr,
            local_addr,
            sender,
            encrypted: false,
            last_ping: None,
            last_pong: None,
            stats: ServerConnectionStats::default(),
        }
    }

    /// Send a message to the server
    pub fn send(&self, message: Message) -> Result<()> {
        self.sender.send(message)
            .map_err(|_| Error::Connection("Failed to send message to server".to_string()))?;
        Ok(())
    }

    /// Send a raw string message to the server
    pub fn send_raw(&self, message: &str) -> Result<()> {
        let msg = Message::parse(message)
            .map_err(|e| Error::MessageParse(format!("Failed to parse message: {}", e)))?;
        self.send(msg)
    }

    /// Check if server is registered
    pub fn is_registered(&self) -> bool {
        self.state == ServerConnectionState::Registered
    }

    /// Check if server is a super server
    pub fn is_super_server(&self) -> bool {
        self.info.is_super_server
    }

    /// Update ping time
    pub fn update_ping(&mut self) {
        self.last_ping = Some(Utc::now());
    }

    /// Update pong time
    pub fn update_pong(&mut self) {
        self.last_pong = Some(Utc::now());
    }

    /// Update statistics
    pub fn update_stats(&mut self, bytes_received: u64, bytes_sent: u64) {
        self.stats.bytes_received += bytes_received;
        self.stats.bytes_sent += bytes_sent;
        self.stats.last_activity = Utc::now();
    }
}

/// Server connection manager
#[derive(Debug)]
pub struct ServerConnectionManager {
    /// Active server connections
    connections: Arc<RwLock<HashMap<String, ServerConnection>>>,
    /// Connection ID to server name mapping
    id_to_name: Arc<RwLock<HashMap<Uuid, String>>>,
    /// Server configuration
    config: Arc<Config>,
}

impl ServerConnectionManager {
    /// Create a new server connection manager
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            id_to_name: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Add a server connection
    pub async fn add_connection(&self, connection: ServerConnection) -> Result<()> {
        let server_name = connection.info.name.clone();
        let connection_id = connection.id;
        
        let mut connections = self.connections.write().await;
        let mut id_to_name = self.id_to_name.write().await;
        
        connections.insert(server_name.clone(), connection);
        id_to_name.insert(connection_id, server_name);
        
        Ok(())
    }

    /// Remove a server connection
    pub async fn remove_connection(&self, server_name: &str) -> Result<Option<ServerConnection>> {
        let mut connections = self.connections.write().await;
        let mut id_to_name = self.id_to_name.write().await;
        
        if let Some(connection) = connections.remove(server_name) {
            id_to_name.remove(&connection.id);
            Ok(Some(connection))
        } else {
            Ok(None)
        }
    }

    /// Get a server connection by name
    pub async fn get_connection(&self, server_name: &str) -> Option<ServerConnection> {
        let connections = self.connections.read().await;
        connections.get(server_name).cloned()
    }

    /// Get a server connection by ID
    pub async fn get_connection_by_id(&self, connection_id: &Uuid) -> Option<ServerConnection> {
        let connections = self.connections.read().await;
        let id_to_name = self.id_to_name.read().await;
        
        if let Some(server_name) = id_to_name.get(connection_id) {
            connections.get(server_name).cloned()
        } else {
            None
        }
    }
    
    /// Get a mutable server connection by name
    pub async fn get_connection_mut(&self, _server_name: &str) -> Option<tokio::sync::RwLockWriteGuard<'_, ServerConnection>> {
        // This is a bit tricky - we need to return a write guard
        // For now, let's add a method to update connection state
        None // TODO: Implement proper mutable access
    }
    
    /// Update server connection state
    pub async fn update_connection_state(&self, server_name: &str, state: ServerConnectionState) -> Result<()> {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(server_name) {
            connection.state = state;
            Ok(())
        } else {
            Err(Error::Server(format!("Server connection {} not found", server_name)))
        }
    }
    
    /// Update server connection ping time
    pub async fn update_connection_ping(&self, server_name: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(server_name) {
            connection.update_ping();
            Ok(())
        } else {
            Err(Error::Server(format!("Server connection {} not found", server_name)))
        }
    }
    
    /// Update server connection pong time
    pub async fn update_connection_pong(&self, server_name: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(server_name) {
            connection.update_pong();
            Ok(())
        } else {
            Err(Error::Server(format!("Server connection {} not found", server_name)))
        }
    }

    /// Get all server connections
    pub async fn get_all_connections(&self) -> Vec<ServerConnection> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// Check if server is connected
    pub async fn is_connected(&self, server_name: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(server_name)
    }

    /// Get server count
    pub async fn server_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Send message to all servers
    pub async fn broadcast_to_servers(&self, message: Message) -> Result<()> {
        let connections = self.connections.read().await;
        for connection in connections.values() {
            if let Err(e) = connection.send(message.clone()) {
                tracing::warn!("Failed to send message to server {}: {}", connection.info.name, e);
            }
        }
        Ok(())
    }

    /// Send message to specific server
    pub async fn send_to_server(&self, server_name: &str, message: Message) -> Result<()> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(server_name) {
            connection.send(message)?;
            Ok(())
        } else {
            Err(Error::Connection(format!("Server {} not found", server_name)))
        }
    }

    /// Get server link configuration
    pub fn get_server_link(&self, server_name: &str) -> Option<&crate::config::ServerLink> {
        self.config.get_server_link(server_name)
    }

    /// Get super server configuration
    pub fn get_super_server(&self, server_name: &str) -> Option<&crate::config::SuperServerConfig> {
        self.config.get_super_server(server_name)
    }

    /// Validate if a server connection is allowed
    pub fn is_server_allowed(&self, server_name: &str, hostname: &str, port: u16) -> bool {
        self.config.is_server_allowed(server_name, hostname, port)
    }

    /// Check if a server is a super server
    pub fn is_super_server(&self, server_name: &str) -> bool {
        self.config.is_super_server(server_name)
    }

    /// Validate incoming server connection
    pub fn validate_incoming_connection(&self, server_name: &str, hostname: &str, port: u16) -> Result<()> {
        if !self.is_server_allowed(server_name, hostname, port) {
            return Err(Error::Server(format!(
                "Server {} ({}) is not authorized to connect - not found in configuration", 
                server_name, hostname
            )));
        }

        // Additional validation can be added here (e.g., password verification, etc.)
        Ok(())
    }
}
