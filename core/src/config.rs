//! Configuration management

use crate::{Error, Result, RepliesConfig};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server information
    pub server: ServerConfig,
    /// Network information
    pub network: NetworkConfig,
    /// Connection settings
    pub connection: ConnectionConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// Module settings
    pub modules: ModuleConfig,
    /// Database settings
    pub database: DatabaseConfig,
    /// Broadcasting settings
    pub broadcast: BroadcastConfig,
    /// Services settings
    pub services: ServicesConfig,
    /// Numeric replies configuration
    pub replies: Option<RepliesConfig>,
}

/// Server-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server name
    pub name: String,
    /// Server description
    pub description: String,
    /// Server version
    pub version: String,
    /// Server creation date
    pub created: String,
    /// Maximum number of clients
    pub max_clients: usize,
    /// Maximum number of channels per client
    pub max_channels_per_client: usize,
    /// Maximum channel name length
    pub max_channel_name_length: usize,
    /// Maximum nickname length
    pub max_nickname_length: usize,
    /// Maximum topic length
    pub max_topic_length: usize,
    /// Maximum away message length
    pub max_away_length: usize,
    /// Maximum kick message length
    pub max_kick_length: usize,
    /// Maximum quit message length
    pub max_quit_length: usize,
    /// Administrator email
    pub admin_email: String,
    /// Administrator location line 1
    pub admin_location1: String,
    /// Administrator location line 2
    pub admin_location2: String,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network name
    pub name: String,
    /// Network description
    pub description: String,
    /// Server links
    pub links: Vec<ServerLink>,
    /// Operator passwords
    pub operators: Vec<OperatorConfig>,
    /// Super servers (u-lined)
    pub super_servers: Vec<SuperServerConfig>,
}

/// Server link configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerLink {
    /// Server name
    pub name: String,
    /// Server hostname
    pub hostname: String,
    /// Server port
    pub port: u16,
    /// Link password
    pub password: String,
    /// Whether to use TLS
    pub tls: bool,
    /// Whether this is an outgoing connection
    pub outgoing: bool,
}

/// Operator flags for different privileges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperatorFlag {
    /// Global operator (can kill any user)
    GlobalOper,
    /// Local operator (can only kill or take operator actions on local server)
    LocalOper,
    /// Can call remote connect
    RemoteConnect,
    /// Can call local connect
    LocalConnect,
    /// Administrator (can see secret channels in WHOIS)
    Administrator,
    /// Spy (informed when someone does WHOIS on them)
    Spy,
}

/// Operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorConfig {
    /// Operator nickname
    pub nickname: String,
    /// Operator password (SHA256 hashed)
    pub password_hash: String,
    /// Operator hostmask (user@host pattern)
    pub hostmask: String,
    /// Operator flags
    pub flags: Vec<OperatorFlag>,
    /// Whether this operator is enabled
    pub enabled: bool,
}

impl OperatorConfig {
    /// Create a new operator configuration
    pub fn new(nickname: String, password: &str, hostmask: String, flags: Vec<OperatorFlag>) -> Self {
        Self {
            password_hash: PasswordHasher::hash_password(password),
            nickname,
            hostmask,
            flags,
            enabled: true,
        }
    }
    
    /// Check if operator has a specific flag
    pub fn has_flag(&self, flag: OperatorFlag) -> bool {
        self.flags.contains(&flag)
    }
    
    /// Check if operator is a global operator
    pub fn is_global_oper(&self) -> bool {
        self.has_flag(OperatorFlag::GlobalOper)
    }
    
    /// Check if operator is a local operator
    pub fn is_local_oper(&self) -> bool {
        self.has_flag(OperatorFlag::LocalOper)
    }
    
    /// Check if operator can do remote connect
    pub fn can_remote_connect(&self) -> bool {
        self.has_flag(OperatorFlag::RemoteConnect)
    }
    
    /// Check if operator can do local connect
    pub fn can_local_connect(&self) -> bool {
        self.has_flag(OperatorFlag::LocalConnect)
    }
    
    /// Check if operator is administrator
    pub fn is_administrator(&self) -> bool {
        self.has_flag(OperatorFlag::Administrator)
    }
    
    /// Check if operator has spy privileges
    pub fn is_spy(&self) -> bool {
        self.has_flag(OperatorFlag::Spy)
    }
    
    /// Verify password
    pub fn verify_password(&self, password: &str) -> bool {
        PasswordHasher::verify_password(password, &self.password_hash)
    }
    
    /// Check if hostmask matches
    pub fn matches_hostmask(&self, user: &str, host: &str) -> bool {
        self.matches_hostmask_pattern(&format!("{}@{}", user, host))
    }
    
    /// Check if a user@host pattern matches this operator's hostmask
    pub fn matches_hostmask_pattern(&self, pattern: &str) -> bool {
        // Simple wildcard matching - can be enhanced with proper regex
        if self.hostmask == "*@*" {
            return true;
        }
        
        if self.hostmask.contains('*') {
            let parts: Vec<&str> = self.hostmask.split('@').collect();
            if parts.len() != 2 {
                return false;
            }
            
            let user_pattern = parts[0];
            let host_pattern = parts[1];
            
            let user_host_parts: Vec<&str> = pattern.split('@').collect();
            if user_host_parts.len() != 2 {
                return false;
            }
            
            let user = user_host_parts[0];
            let host = user_host_parts[1];
            
            return self.matches_pattern(user, user_pattern) && self.matches_pattern(host, host_pattern);
        }
        
        self.hostmask == pattern
    }
    
    /// Simple pattern matching with wildcards
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                return text.starts_with(parts[0]) && text.ends_with(parts[1]);
            }
        }
        
        text == pattern
    }
}

/// Password hashing utilities
pub struct PasswordHasher;

impl PasswordHasher {
    /// Hash a password using SHA256
    pub fn hash_password(password: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Verify a password against its hash
    pub fn verify_password(password: &str, hash: &str) -> bool {
        Self::hash_password(password) == hash
    }
}

/// Super server configuration (u-lined)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperServerConfig {
    /// Super server name
    pub name: String,
    /// Super server hostname
    pub hostname: String,
    /// Super server port
    pub port: u16,
    /// Link password
    pub password: String,
    /// Whether to use TLS
    pub tls: bool,
    /// Super server privileges
    pub privileges: Vec<String>,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// List of ports to listen on
    pub ports: Vec<PortConfig>,
    /// Bind address
    pub bind_address: String,
    /// Connection timeout (seconds)
    pub connection_timeout: u64,
    /// Ping timeout (seconds)
    pub ping_timeout: u64,
    /// Maximum connection rate per IP
    pub max_connections_per_ip: usize,
    /// Maximum connection rate per host
    pub max_connections_per_host: usize,
}

/// Port configuration for listening
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    /// Port number
    pub port: u16,
    /// Connection type (client, server, or both)
    pub connection_type: PortConnectionType,
    /// Whether to use TLS for this port
    pub tls: bool,
    /// Optional description for this port
    pub description: Option<String>,
}

/// Types of connections allowed on a port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortConnectionType {
    /// Client connections only
    Client,
    /// Server connections only
    Server,
    /// Both client and server connections
    Both,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Allowed client hosts
    pub allowed_hosts: Vec<String>,
    /// Denied client hosts
    pub denied_hosts: Vec<String>,
    /// Require password for clients
    pub require_client_password: bool,
    /// Client password
    pub client_password: Option<String>,
    /// Enable ident lookups
    pub enable_ident: bool,
    /// Enable DNS lookups
    pub enable_dns: bool,
    /// Enable reverse DNS
    pub enable_reverse_dns: bool,
    /// TLS configuration
    pub tls: TlsConfig,
    /// Server security settings
    pub server_security: ServerSecurityConfig,
}

/// Server security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSecurityConfig {
    /// Allow remote CONNECT commands from operators
    pub allow_remote_connect: bool,
    /// Require operator privileges for CONNECT
    pub require_oper_for_connect: bool,
    /// Allowed hosts for remote connections
    pub allowed_remote_hosts: Vec<String>,
    /// Denied hosts for remote connections
    pub denied_remote_hosts: Vec<String>,
    /// Maximum hop count for server connections
    pub max_hop_count: u8,
    /// Require authentication for server connections
    pub require_server_auth: bool,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,
    /// Certificate file
    pub cert_file: Option<String>,
    /// Private key file
    pub key_file: Option<String>,
    /// CA certificate file
    pub ca_file: Option<String>,
    /// TLS version
    pub version: String,
    /// Cipher suites
    pub cipher_suites: Vec<String>,
}

/// Module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Module directory
    pub module_directory: String,
    /// Enabled modules
    pub enabled_modules: Vec<String>,
    /// Module-specific settings
    pub module_settings: HashMap<String, serde_json::Value>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Maximum number of users to keep in history
    pub max_history_size: usize,
    /// Number of days to retain user history
    pub history_retention_days: i64,
    /// Enable channel tracking
    pub enable_channel_tracking: bool,
    /// Enable user activity tracking
    pub enable_activity_tracking: bool,
}

/// Broadcasting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastConfig {
    /// Maximum concurrent queries
    pub max_concurrent_queries: usize,
    /// Query timeout in seconds
    pub query_timeout_seconds: u64,
    /// Enable network-wide queries
    pub enable_network_queries: bool,
    /// Enable efficient broadcasting
    pub enable_efficient_broadcasting: bool,
}

/// Services configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    /// Services directory
    pub services_directory: String,
    /// Enabled services
    pub enabled_services: Vec<String>,
    /// Service-specific settings
    pub service_settings: HashMap<String, serde_json::Value>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_history_size: 10000,
            history_retention_days: 30,
            enable_channel_tracking: true,
            enable_activity_tracking: true,
        }
    }
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            max_concurrent_queries: 100,
            query_timeout_seconds: 30,
            enable_network_queries: true,
            enable_efficient_broadcasting: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            network: NetworkConfig::default(),
            connection: ConnectionConfig::default(),
            security: SecurityConfig::default(),
            modules: ModuleConfig::default(),
            database: DatabaseConfig::default(),
            broadcast: BroadcastConfig::default(),
            services: ServicesConfig::default(),
            replies: None, // Will be loaded from replies.toml if available
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "localhost".to_string(),
            description: "Rust IRC Daemon".to_string(),
            version: "0.1.0".to_string(),
            created: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            max_clients: 1000,
            max_channels_per_client: 10,
            max_channel_name_length: 200,
            max_nickname_length: 9,
            max_topic_length: 390,
            max_away_length: 160,
            max_kick_length: 160,
            max_quit_length: 160,
            admin_email: "admin@example.com".to_string(),
            admin_location1: "Rust IRC Network".to_string(),
            admin_location2: "https://github.com/rustircd/rustircd".to_string(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            name: "RustNet".to_string(),
            description: "Rust IRC Network".to_string(),
            links: Vec::new(),
            operators: Vec::new(),
            super_servers: Vec::new(),
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            ports: vec![
                PortConfig {
                    port: 6667,
                    connection_type: PortConnectionType::Client,
                    tls: false,
                    description: Some("Standard IRC port".to_string()),
                },
                PortConfig {
                    port: 6668,
                    connection_type: PortConnectionType::Server,
                    tls: false,
                    description: Some("Server-to-server connections".to_string()),
                },
                PortConfig {
                    port: 6697,
                    connection_type: PortConnectionType::Client,
                    tls: true,
                    description: Some("Secure IRC port".to_string()),
                },
                PortConfig {
                    port: 6698,
                    connection_type: PortConnectionType::Server,
                    tls: true,
                    description: Some("Secure server-to-server connections".to_string()),
                },
            ],
            bind_address: "0.0.0.0".to_string(),
            connection_timeout: 60,
            ping_timeout: 300,
            max_connections_per_ip: 5,
            max_connections_per_host: 10,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_hosts: vec!["*".to_string()],
            denied_hosts: Vec::new(),
            require_client_password: false,
            client_password: None,
            enable_ident: true,
            enable_dns: true,
            enable_reverse_dns: true,
            tls: TlsConfig::default(),
            server_security: ServerSecurityConfig::default(),
        }
    }
}

impl Default for ServerSecurityConfig {
    fn default() -> Self {
        Self {
            allow_remote_connect: true,
            require_oper_for_connect: true,
            allowed_remote_hosts: vec!["*".to_string()],
            denied_remote_hosts: Vec::new(),
            max_hop_count: 10,
            require_server_auth: true,
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_file: None,
            key_file: None,
            ca_file: None,
            version: "1.3".to_string(),
            cipher_suites: vec!["TLS_AES_256_GCM_SHA384".to_string(), "TLS_CHACHA20_POLY1305_SHA256".to_string()],
        }
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            module_directory: "modules".to_string(),
            enabled_modules: Vec::new(),
            module_settings: HashMap::new(),
        }
    }
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            services_directory: "services".to_string(),
            enabled_services: Vec::new(),
            service_settings: HashMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_path = path.as_ref();
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;
        
        let mut config: Config = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config file: {}", e)))?;
        
        // Try to load replies configuration if not already set
        if config.replies.is_none() {
            let replies_path = config_path.parent()
                .map(|p| p.join("replies.toml"))
                .unwrap_or_else(|| std::path::PathBuf::from("replies.toml"));
            
            if replies_path.exists() {
                match RepliesConfig::from_file(&replies_path) {
                    Ok(replies_config) => {
                        config.replies = Some(replies_config);
                        tracing::info!("Loaded replies configuration from {:?}", replies_path);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load replies configuration from {:?}: {}", replies_path, e);
                    }
                }
            } else {
                tracing::debug!("No replies.toml found, using default replies");
            }
        }
        
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(path, content)
            .map_err(|e| Error::Config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server name
        if self.server.name.is_empty() {
            return Err(Error::Config("Server name cannot be empty".to_string()));
        }
        
        // Validate ports
        if self.connection.ports.is_empty() {
            return Err(Error::Config("At least one port must be configured".to_string()));
        }
        
        let mut seen_ports = std::collections::HashSet::new();
        for port_config in &self.connection.ports {
            if port_config.port == 0 {
                return Err(Error::Config("Port cannot be 0".to_string()));
            }
            
            if seen_ports.contains(&port_config.port) {
                return Err(Error::Config(format!("Duplicate port {} in configuration", port_config.port)));
            }
            seen_ports.insert(port_config.port);
            
            // Validate TLS configuration for TLS ports
            if port_config.tls && !self.security.tls.enabled {
                return Err(Error::Config(format!("Port {} configured for TLS but TLS is not enabled globally", port_config.port)));
            }
        }
        
        // Validate TLS configuration
        if self.security.tls.enabled {
            if self.security.tls.cert_file.is_none() {
                return Err(Error::Config("TLS enabled but no certificate file specified".to_string()));
            }
            
            if self.security.tls.key_file.is_none() {
                return Err(Error::Config("TLS enabled but no key file specified".to_string()));
            }
        }
        
        // Validate limits
        if self.server.max_clients == 0 {
            return Err(Error::Config("Max clients must be greater than 0".to_string()));
        }
        
        if self.server.max_channels_per_client == 0 {
            return Err(Error::Config("Max channels per client must be greater than 0".to_string()));
        }
        
        // Validate server links
        self.validate_server_links()?;
        
        // Validate super servers
        self.validate_super_servers()?;
        
        // Validate server security configuration
        self.validate_server_security()?;
        
        Ok(())
    }

    /// Validate server links configuration
    fn validate_server_links(&self) -> Result<()> {
        let mut seen_names = std::collections::HashSet::new();
        let mut seen_hostname_port = std::collections::HashSet::new();
        
        for link in &self.network.links {
            // Check for duplicate names
            if seen_names.contains(&link.name) {
                return Err(Error::Config(format!("Duplicate server link name: {}", link.name)));
            }
            seen_names.insert(link.name.clone());
            
            // Check for duplicate hostname:port combinations
            let hostname_port = format!("{}:{}", link.hostname, link.port);
            if seen_hostname_port.contains(&hostname_port) {
                return Err(Error::Config(format!("Duplicate hostname:port combination: {}", hostname_port)));
            }
            seen_hostname_port.insert(hostname_port);
            
            // Validate link configuration
            if link.name.is_empty() {
                return Err(Error::Config("Server link name cannot be empty".to_string()));
            }
            
            if link.hostname.is_empty() {
                return Err(Error::Config(format!("Server link {} hostname cannot be empty", link.name)));
            }
            
            if link.port == 0 {
                return Err(Error::Config(format!("Server link {} port cannot be 0", link.name)));
            }
            
            if link.password.is_empty() {
                return Err(Error::Config(format!("Server link {} password cannot be empty", link.name)));
            }
            
            // Validate hostname format (basic check)
            if !self.is_valid_hostname(&link.hostname) {
                return Err(Error::Config(format!("Invalid hostname format for server link {}: {}", link.name, link.hostname)));
            }
        }
        
        Ok(())
    }

    /// Validate super servers configuration
    fn validate_super_servers(&self) -> Result<()> {
        let mut seen_names = std::collections::HashSet::new();
        
        for super_server in &self.network.super_servers {
            // Check for duplicate names
            if seen_names.contains(&super_server.name) {
                return Err(Error::Config(format!("Duplicate super server name: {}", super_server.name)));
            }
            seen_names.insert(super_server.name.clone());
            
            // Validate super server configuration
            if super_server.name.is_empty() {
                return Err(Error::Config("Super server name cannot be empty".to_string()));
            }
            
            if super_server.hostname.is_empty() {
                return Err(Error::Config(format!("Super server {} hostname cannot be empty", super_server.name)));
            }
            
            if super_server.port == 0 {
                return Err(Error::Config(format!("Super server {} port cannot be 0", super_server.name)));
            }
            
            if super_server.password.is_empty() {
                return Err(Error::Config(format!("Super server {} password cannot be empty", super_server.name)));
            }
            
            // Validate hostname format
            if !self.is_valid_hostname(&super_server.hostname) {
                return Err(Error::Config(format!("Invalid hostname format for super server {}: {}", super_server.name, super_server.hostname)));
            }
            
            // Check if super server conflicts with regular server links
            for link in &self.network.links {
                if link.name == super_server.name {
                    return Err(Error::Config(format!("Super server {} conflicts with server link of same name", super_server.name)));
                }
                if link.hostname == super_server.hostname && link.port == super_server.port {
                    return Err(Error::Config(format!("Super server {}:{} conflicts with server link {}:{}", 
                        super_server.name, super_server.hostname, link.name, link.hostname)));
                }
            }
        }
        
        Ok(())
    }

    /// Validate server security configuration
    fn validate_server_security(&self) -> Result<()> {
        let server_security = &self.security.server_security;
        
        // Validate host patterns
        for allowed_host in &server_security.allowed_remote_hosts {
            if !self.is_valid_host_pattern(allowed_host) {
                return Err(Error::Config(format!("Invalid allowed host pattern: {}", allowed_host)));
            }
        }
        
        for denied_host in &server_security.denied_remote_hosts {
            if !self.is_valid_host_pattern(denied_host) {
                return Err(Error::Config(format!("Invalid denied host pattern: {}", denied_host)));
            }
        }
        
        // Validate hop count
        if server_security.max_hop_count == 0 {
            return Err(Error::Config("Maximum hop count must be greater than 0".to_string()));
        }
        
        // If remote connect is allowed, ensure we have proper configuration
        if server_security.allow_remote_connect {
            if server_security.allowed_remote_hosts.is_empty() {
                return Err(Error::Config("Allowed remote hosts cannot be empty when remote connect is enabled".to_string()));
            }
        }
        
        Ok(())
    }

    /// Check if a hostname is valid (basic validation)
    fn is_valid_hostname(&self, hostname: &str) -> bool {
        if hostname.is_empty() || hostname.len() > 253 {
            return false;
        }
        
        // Allow IP addresses and domain names
        // This is a basic check - in production you might want more sophisticated validation
        hostname.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':')
    }

    /// Check if a host pattern is valid
    fn is_valid_host_pattern(&self, pattern: &str) -> bool {
        if pattern.is_empty() {
            return false;
        }
        
        // Allow wildcards and basic hostname patterns
        pattern.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '*' || c == ':')
    }

    /// Check if a server is allowed to connect (either as link or super server)
    pub fn is_server_allowed(&self, server_name: &str, hostname: &str, port: u16) -> bool {
        // Check server links
        for link in &self.network.links {
            if link.name == server_name && link.hostname == hostname && link.port == port {
                return true;
            }
        }
        
        // Check super servers
        for super_server in &self.network.super_servers {
            if super_server.name == server_name && super_server.hostname == hostname && super_server.port == port {
                return true;
            }
        }
        
        false
    }

    /// Get server link configuration by name
    pub fn get_server_link(&self, server_name: &str) -> Option<&ServerLink> {
        self.network.links.iter().find(|link| link.name == server_name)
    }

    /// Get super server configuration by name
    pub fn get_super_server(&self, server_name: &str) -> Option<&SuperServerConfig> {
        self.network.super_servers.iter().find(|server| server.name == server_name)
    }

    /// Check if a server is a super server
    pub fn is_super_server(&self, server_name: &str) -> bool {
        self.network.super_servers.iter().any(|server| server.name == server_name)
    }

    /// Find operator by nickname
    pub fn find_operator_by_nickname(&self, nickname: &str) -> Option<&OperatorConfig> {
        self.network.operators.iter()
            .find(|op| op.nickname == nickname && op.enabled)
    }

    /// Authenticate operator with password
    pub fn authenticate_operator(&self, nickname: &str, password: &str, user: &str, host: &str) -> Option<&OperatorConfig> {
        if let Some(operator) = self.find_operator_by_nickname(nickname) {
            if operator.verify_password(password) && operator.matches_hostmask(user, host) {
                return Some(operator);
            }
        }
        None
    }

    /// Check if a user is an operator with specific flag
    pub fn is_user_operator_with_flag(&self, nickname: &str, user: &str, host: &str, flag: OperatorFlag) -> bool {
        if let Some(operator) = self.find_operator_by_nickname(nickname) {
            operator.matches_hostmask(user, host) && operator.has_flag(flag)
        } else {
            false
        }
    }
}
