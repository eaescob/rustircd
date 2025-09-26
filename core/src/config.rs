//! Configuration management

use crate::{Error, Result};
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

/// Operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorConfig {
    /// Operator name
    pub name: String,
    /// Operator password
    pub password: String,
    /// Operator hostmask
    pub hostmask: String,
    /// Operator privileges
    pub privileges: Vec<String>,
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
    /// Client port
    pub client_port: u16,
    /// Server port
    pub server_port: u16,
    /// Client TLS port
    pub client_tls_port: Option<u16>,
    /// Server TLS port
    pub server_tls_port: Option<u16>,
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
            client_port: 6667,
            server_port: 6668,
            client_tls_port: Some(6697),
            server_tls_port: Some(6698),
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
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Failed to parse config file: {}", e)))
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
        if self.connection.client_port == 0 {
            return Err(Error::Config("Client port cannot be 0".to_string()));
        }
        
        if self.connection.server_port == 0 {
            return Err(Error::Config("Server port cannot be 0".to_string()));
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
        
        Ok(())
    }
}
