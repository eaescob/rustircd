//! Client connection management

use crate::{Message, User, Error, Result};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Client connection state
#[derive(Debug, Clone)]
pub enum ClientState {
    /// Just connected, not registered
    Connected,
    /// Password provided
    PasswordProvided,
    /// Nickname set
    NickSet,
    /// User info provided
    UserSet,
    /// Fully registered
    Registered,
    /// Disconnected
    Disconnected,
}

/// Client connection information
#[derive(Debug)]
pub struct Client {
    /// Unique client ID
    pub id: Uuid,
    /// Client state
    pub state: ClientState,
    /// User information (if registered)
    pub user: Option<User>,
    /// Remote address
    pub remote_addr: String,
    /// Local address
    pub local_addr: String,
    /// Message sender for sending messages to client
    pub sender: mpsc::UnboundedSender<Message>,
    /// Whether connection is encrypted
    pub encrypted: bool,
    /// Capabilities being negotiated
    pub capabilities: std::collections::HashSet<String>,
    /// Whether client supports IRCv3
    pub supports_ircv3: bool,
}

impl Client {
    /// Create a new client
    pub fn new(
        id: Uuid,
        remote_addr: String,
        local_addr: String,
        sender: mpsc::UnboundedSender<Message>,
    ) -> Self {
        Self {
            id,
            state: ClientState::Connected,
            user: None,
            remote_addr,
            local_addr,
            sender,
            encrypted: false,
            capabilities: std::collections::HashSet::new(),
            supports_ircv3: false,
        }
    }
    
    /// Send a message to the client
    pub fn send(&self, message: Message) -> Result<()> {
        self.sender.send(message)
            .map_err(|_| Error::Connection("Failed to send message to client".to_string()))?;
        Ok(())
    }
    
    /// Send a raw string message to the client
    pub fn send_raw(&self, message: &str) -> Result<()> {
        let msg = Message::parse(message)?;
        self.send(msg)
    }
    
    /// Check if client is registered
    pub fn is_registered(&self) -> bool {
        matches!(self.state, ClientState::Registered)
    }
    
    /// Check if client has provided password
    pub fn has_password(&self) -> bool {
        matches!(self.state, ClientState::PasswordProvided | ClientState::NickSet | ClientState::UserSet | ClientState::Registered)
    }
    
    /// Check if client has set nickname
    pub fn has_nick(&self) -> bool {
        matches!(self.state, ClientState::NickSet | ClientState::UserSet | ClientState::Registered)
    }
    
    /// Check if client has provided user info
    pub fn has_user(&self) -> bool {
        matches!(self.state, ClientState::UserSet | ClientState::Registered)
    }
    
    /// Get client nickname
    pub fn nickname(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.nick.as_str())
    }
    
    /// Get client username
    pub fn username(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.username.as_str())
    }
    
    /// Get client hostname
    pub fn hostname(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.host.as_str())
    }
    
    /// Get client real name
    pub fn realname(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.realname.as_str())
    }
    
    /// Set client state
    pub fn set_state(&mut self, state: ClientState) {
        self.state = state;
    }
    
    /// Set user information
    pub fn set_user(&mut self, user: User) {
        self.user = Some(user);
    }
    
    /// Get user reference
    pub fn get_user(&self) -> Option<&User> {
        self.user.as_ref()
    }
    
    /// Get mutable user reference
    pub fn get_user_mut(&mut self) -> Option<&mut User> {
        self.user.as_mut()
    }
    
    /// Add capability
    pub fn add_capability(&mut self, cap: String) {
        self.capabilities.insert(cap);
    }
    
    /// Remove capability
    pub fn remove_capability(&mut self, cap: &str) {
        self.capabilities.remove(cap);
    }
    
    /// Check if client has capability
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.contains(cap)
    }
    
    /// Set IRCv3 support
    pub fn set_ircv3_support(&mut self, supported: bool) {
        self.supports_ircv3 = supported;
    }
    
    /// Check if client supports IRCv3
    pub fn supports_ircv3(&self) -> bool {
        self.supports_ircv3
    }
    
    /// Get client info string
    pub fn info_string(&self) -> String {
        if let Some(ref user) = self.user {
            format!("{}!{}@{}", user.nick, user.username, user.host)
        } else {
            format!("unknown@{}", self.remote_addr)
        }
    }
}
