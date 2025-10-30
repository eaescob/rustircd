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
    /// Connection classes - define parameters for groups of connections
    pub classes: Vec<ConnectionClass>,
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
    /// Authentication settings
    pub authentication: Option<AuthenticationConfig>,
    /// Netsplit recovery settings
    pub netsplit: NetsplitConfig,
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
    /// Show server IPs/hostnames in STATS commands (even for operators)
    pub show_server_details_in_stats: bool,
    /// MOTD (Message of the Day) file path
    pub motd_file: Option<String>,
    /// WHOIS string for IRC operators (default: "is an IRC Operator")
    #[serde(default = "default_oper_whois_string")]
    pub oper_whois_string: String,
    /// WHOIS string for server administrators (default: "is a Server Administrator")
    #[serde(default = "default_admin_whois_string")]
    pub admin_whois_string: String,
}

fn default_oper_whois_string() -> String {
    "is an IRC Operator".to_string()
}

fn default_admin_whois_string() -> String {
    "is a Server Administrator".to_string()
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
    /// Connection class for this server link
    pub class: Option<String>,
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
    /// Can use SQUIT command to disconnect servers
    Squit,
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
    
    /// Check if operator can use SQUIT command
    pub fn can_squit(&self) -> bool {
        self.has_flag(OperatorFlag::Squit)
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
    /// Hash a password using Argon2id (recommended)
    ///
    /// This method uses the Argon2id algorithm with secure defaults:
    /// - Memory cost: 19 MiB
    /// - Time cost: 2 iterations
    /// - Parallelism: 1 thread
    /// - Random salt per password
    ///
    /// Returns a PHC-formatted string that includes algorithm, parameters, salt, and hash.
    pub fn hash_password(password: &str) -> String {
        use argon2::{
            password_hash::{PasswordHasher as Argon2Hasher, SaltString},
            Argon2,
        };
        use rand::rngs::OsRng;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .expect("Failed to hash password")
            .to_string()
    }

    /// Hash a password using legacy SHA-256 (deprecated, for migration only)
    ///
    /// This method is provided for backward compatibility during migration.
    /// New passwords should use hash_password() which uses Argon2id.
    #[deprecated(since = "0.1.0", note = "Use hash_password() with Argon2 instead")]
    pub fn hash_password_sha256(password: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify a password against its hash
    ///
    /// Automatically detects the hash format:
    /// - Argon2 hashes start with "$argon2"
    /// - SHA-256 hashes are 64 hex characters
    ///
    /// This allows backward compatibility with existing SHA-256 hashes
    /// during the migration period.
    pub fn verify_password(password: &str, hash: &str) -> bool {
        // Detect hash format
        if hash.starts_with("$argon2") {
            // Argon2 hash - use password_hash crate
            use argon2::{
                password_hash::{PasswordHash, PasswordVerifier},
                Argon2,
            };

            match PasswordHash::new(hash) {
                Ok(parsed_hash) => {
                    Argon2::default()
                        .verify_password(password.as_bytes(), &parsed_hash)
                        .is_ok()
                }
                Err(e) => {
                    tracing::error!("Failed to parse Argon2 hash: {}", e);
                    false
                }
            }
        } else if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
            // Legacy SHA-256 hash (64 hex characters)
            tracing::warn!("Using legacy SHA-256 password verification. Please migrate to Argon2.");
            #[allow(deprecated)]
            let computed_hash = Self::hash_password_sha256(password);
            computed_hash == hash
        } else {
            tracing::error!("Unknown password hash format: {}", &hash[..hash.len().min(20)]);
            false
        }
    }

    /// Check if a hash is using the modern Argon2 format
    pub fn is_argon2_hash(hash: &str) -> bool {
        hash.starts_with("$argon2")
    }

    /// Check if a hash is using the legacy SHA-256 format
    pub fn is_sha256_hash(hash: &str) -> bool {
        hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
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
    /// Whether to verify TLS certificate (default: true)
    pub tls_verify: Option<bool>,
    /// Custom CA file path for TLS verification
    pub tls_ca_file: Option<String>,
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
    /// Optional bind address for this specific port (overrides global bind_address)
    pub bind_address: Option<String>,
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

/// Connection class configuration - defines parameters for groups of connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionClass {
    /// Class name (unique identifier)
    pub name: String,
    /// Maximum number of clients in this class
    pub max_clients: Option<usize>,
    /// Ping frequency in seconds (how often to send PING to check if connection is alive)
    pub ping_frequency: Option<u64>,
    /// Connection timeout in seconds (how long to wait before dropping unresponsive connection)
    pub connection_timeout: Option<u64>,
    /// Maximum sendq size in bytes (send queue - outgoing data buffer)
    pub max_sendq: Option<usize>,
    /// Maximum receive queue size in bytes (incoming data buffer)
    pub max_recvq: Option<usize>,
    /// Whether to disable throttling for this class
    pub disable_throttling: bool,
    /// Maximum connections per IP for this class (overrides global setting)
    pub max_connections_per_ip: Option<usize>,
    /// Maximum connections per host for this class (overrides global setting)
    pub max_connections_per_host: Option<usize>,
    /// Class description
    pub description: Option<String>,
}

impl Default for ConnectionClass {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            max_clients: None,
            ping_frequency: Some(120),
            connection_timeout: Some(300),
            max_sendq: Some(1048576), // 1MB
            max_recvq: Some(8192),    // 8KB
            disable_throttling: false,
            max_connections_per_ip: None,
            max_connections_per_host: None,
            description: Some("Default connection class".to_string()),
        }
    }
}

/// Allow block - defines which hosts can connect and assigns them to a class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowBlock {
    /// Host patterns that are allowed (supports wildcards)
    pub hosts: Vec<String>,
    /// IP patterns that are allowed (supports CIDR notation)
    pub ips: Vec<String>,
    /// Connection class for this allow block
    pub class: String,
    /// Optional password required for connections in this allow block
    pub password: Option<String>,
    /// Maximum number of connections for this allow block
    pub max_connections: Option<usize>,
    /// Description of this allow block
    pub description: Option<String>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Allowed client hosts (deprecated - use allow_blocks instead)
    pub allowed_hosts: Vec<String>,
    /// Denied client hosts
    pub denied_hosts: Vec<String>,
    /// Allow blocks - define which hosts can connect and assign them to classes
    pub allow_blocks: Vec<AllowBlock>,
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
    /// Throttling configuration
    pub throttling: ThrottlingConfig,
    /// Command rate limiting configuration
    pub command_rate_limiting: CommandRateLimitConfig,
    /// Messaging modules configuration
    pub messaging: MessagingConfig,
}

/// Messaging modules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingConfig {
    /// Enable messaging modules system
    pub enabled: bool,
    /// Wallops module configuration
    pub wallops: MessagingModuleConfig,
    /// Globops module configuration
    pub globops: MessagingModuleConfig,
}

/// Individual messaging module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingModuleConfig {
    /// Whether this module is enabled
    pub enabled: bool,
    /// Whether operator privileges are required to send messages
    pub require_operator: bool,
    /// The user mode character for receiving messages
    pub receiver_mode: Option<char>,
    /// Whether the receiver mode can only be set by the user themselves
    pub self_only_mode: bool,
    /// Whether the receiver mode requires operator privileges to set
    pub mode_requires_operator: bool,
}

/// Throttling configuration for connection rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottlingConfig {
    /// Enable throttling module
    pub enabled: bool,
    /// Maximum connections allowed per IP within the time window
    pub max_connections_per_ip: usize,
    /// Time window in seconds for connection counting
    pub time_window_seconds: u64,
    /// Initial throttling duration in seconds (stage 1)
    pub initial_throttle_seconds: u64,
    /// Maximum number of throttling stages
    pub max_stages: u8,
    /// Factor by which throttling increases between stages
    pub stage_factor: u64,
    /// Cleanup interval in seconds for expired throttle entries
    pub cleanup_interval_seconds: u64,
}

/// Action to take when a rate limit is exceeded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitAction {
    /// Drop the command silently (no error message sent)
    Drop,
    /// Send an error message to the user and drop the command
    SendError,
    /// Temporarily mute the user (future implementation)
    Mute,
}

/// Command rate limiting configuration
///
/// Limits the number of commands a user can send within a time window
/// to prevent flooding and DoS attacks through message spam, nick changes, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRateLimitConfig {
    /// Enable command rate limiting
    pub enabled: bool,
    /// Maximum commands allowed within the time window
    pub max_commands: usize,
    /// Time window in seconds for command counting
    pub time_window_seconds: u64,
    /// Commands to rate limit (e.g., ["PRIVMSG", "NOTICE", "NICK"])
    /// If empty, rate limiting applies to all commands
    pub limited_commands: Vec<String>,
    /// Whether to exempt operators from rate limiting
    pub exempt_operators: bool,
    /// Action to take when rate limit is exceeded
    pub limit_action: RateLimitAction,
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
    /// User lookup cache size (nickname → UUID mappings)
    pub user_cache_size: Option<usize>,
    /// User lookup cache TTL in seconds
    pub user_cache_ttl_seconds: Option<u64>,
    /// Channel member cache TTL in seconds
    pub channel_cache_ttl_seconds: Option<u64>,
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
    /// Enabled services (deprecated - use individual service.enabled instead)
    pub enabled_services: Vec<String>,
    /// Service definitions
    pub services: Vec<ServiceDefinition>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Whether authentication is enabled
    pub enabled: bool,
    /// Authentication method
    pub method: AuthenticationMethod,
    /// Whether to require authentication for all users
    pub require_auth: bool,
    /// Authentication cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum number of cached authentications
    pub max_cache_size: usize,
    /// Primary authentication provider (for direct method)
    pub primary_provider: Option<String>,
    /// Direct authentication providers
    pub direct: Option<DirectAuthConfig>,
}

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    /// Direct authentication (Supabase, LDAP, Database, etc.)
    Direct,
    /// IRC services authentication (Atheme, Anope, etc.)
    Services,
}

/// Direct authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectAuthConfig {
    /// Enabled providers
    pub providers: Vec<String>,
    /// Provider-specific configurations
    pub provider_configs: HashMap<String, serde_json::Value>,
}

/// Service definition for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    /// Service name (server name)
    pub name: String,
    /// Service type (matches implementation in services folder)
    pub service_type: String,
    /// Service hostname
    pub hostname: String,
    /// Service port
    pub port: u16,
    /// Service password
    pub password: String,
    /// Whether to use TLS
    pub tls: bool,
    /// Whether to verify TLS certificate (default: true)
    pub tls_verify: Option<bool>,
    /// Custom CA file path for TLS verification
    pub tls_ca_file: Option<String>,
    /// Service-specific configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Whether this service is enabled
    pub enabled: bool,
}

/// Netsplit recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetsplitConfig {
    /// Enable automatic reconnection to disconnected servers
    pub auto_reconnect: bool,
    /// Base delay in seconds for reconnection attempts (exponential backoff starts here)
    pub reconnect_delay_base: u64,
    /// Maximum delay in seconds between reconnection attempts
    pub reconnect_delay_max: u64,
    /// Grace period in seconds before permanently removing split users
    pub split_user_grace_period: u64,
    /// Enable burst protocol optimization for quick reconnects
    pub burst_optimization_enabled: bool,
    /// Time window in seconds for burst optimization (if rejoin within this window, send delta)
    pub burst_optimization_window: u64,
    /// Notify operators about netsplits and reconnections
    pub notify_opers_on_split: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_history_size: 10000,
            history_retention_days: 30,
            enable_channel_tracking: true,
            enable_activity_tracking: true,
            user_cache_size: Some(10000),
            user_cache_ttl_seconds: Some(300),  // 5 minutes
            channel_cache_ttl_seconds: Some(30),  // 30 seconds
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

impl Default for NetsplitConfig {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            reconnect_delay_base: 30,      // 30 seconds initial delay
            reconnect_delay_max: 1800,     // 30 minutes maximum delay
            split_user_grace_period: 60,   // 60 seconds grace period
            burst_optimization_enabled: true,
            burst_optimization_window: 300, // 5 minutes window
            notify_opers_on_split: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            network: NetworkConfig::default(),
            connection: ConnectionConfig::default(),
            classes: vec![ConnectionClass::default()], // Include a default class
            security: SecurityConfig::default(),
            modules: ModuleConfig::default(),
            database: DatabaseConfig::default(),
            broadcast: BroadcastConfig::default(),
            services: ServicesConfig::default(),
            authentication: None, // No authentication by default
            netsplit: NetsplitConfig::default(),
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
            show_server_details_in_stats: true, // Default to showing details for operators
            motd_file: Some("motd.txt".to_string()), // Default MOTD file
            oper_whois_string: default_oper_whois_string(),
            admin_whois_string: default_admin_whois_string(),
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
                    bind_address: None, // Use global bind_address
                },
                PortConfig {
                    port: 6668,
                    connection_type: PortConnectionType::Server,
                    tls: false,
                    description: Some("Server-to-server connections".to_string()),
                    bind_address: None, // Use global bind_address
                },
                PortConfig {
                    port: 6697,
                    connection_type: PortConnectionType::Client,
                    tls: true,
                    description: Some("Secure IRC port".to_string()),
                    bind_address: None, // Use global bind_address
                },
                PortConfig {
                    port: 6698,
                    connection_type: PortConnectionType::Server,
                    tls: true,
                    description: Some("Secure server-to-server connections".to_string()),
                    bind_address: None, // Use global bind_address
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
            allow_blocks: Vec::new(), // Empty by default, will fall back to allowed_hosts
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
            throttling: ThrottlingConfig::default(),
            command_rate_limiting: CommandRateLimitConfig::default(),
            messaging: MessagingConfig::default(),
        }
    }
}

impl Default for MessagingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            wallops: MessagingModuleConfig {
                enabled: true,
                require_operator: true,
                receiver_mode: Some('w'),
                self_only_mode: true,
                mode_requires_operator: false,  // Users can set +w themselves
            },
            globops: MessagingModuleConfig {
                enabled: true,
                require_operator: true,
                receiver_mode: Some('g'),
                self_only_mode: false,          // Operators can set +g on others
                mode_requires_operator: true,   // Only operators can set +g
            },
        }
    }
}

impl Default for MessagingModuleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            require_operator: true,
            receiver_mode: None,
            self_only_mode: true,
            mode_requires_operator: false,
        }
    }
}

impl Default for ThrottlingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_connections_per_ip: 5,
            time_window_seconds: 60,
            initial_throttle_seconds: 10,
            max_stages: 10,
            stage_factor: 10,
            cleanup_interval_seconds: 300, // 5 minutes
        }
    }
}

impl Default for CommandRateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_commands: 10,
            time_window_seconds: 60,
            // Empty list means all commands are rate limited (most secure)
            limited_commands: vec![],
            exempt_operators: true,
            limit_action: RateLimitAction::SendError,
        }
    }
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            services_directory: "services".to_string(),
            enabled_services: Vec::new(), // Deprecated - use individual service.enabled
            services: Vec::new(),
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
        
        // Migrate legacy configuration if needed
        config.migrate_legacy_config();
        
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
    
    /// Migrate legacy configuration to new format
    fn migrate_legacy_config(&mut self) {
        // Check for legacy authentication configuration
        // This would detect old [auth.*] sections and migrate them
        // For now, just log that migration is available
        tracing::debug!("Configuration migration completed");
    }
    
    /// Validate configuration (comprehensive validation with warnings)
    pub fn validate(&self) -> Result<()> {
        // Run comprehensive validation
        let validator = crate::validation::ConfigValidator::new(self.clone());
        let validation_result = validator.validate();

        // Log warnings
        for warning in &validation_result.warnings {
            tracing::warn!("[{}] {}", warning.section, warning.message);
            if let Some(suggestion) = &warning.suggestion {
                tracing::info!("  Suggestion: {}", suggestion);
            }
        }

        // Return error if validation failed
        if !validation_result.is_valid {
            let error_messages: Vec<String> = validation_result.errors
                .iter()
                .map(|e| format!("{}: {}", e.section, e.message))
                .collect();
            return Err(Error::Config(format!("Configuration validation failed:\n  {}", error_messages.join("\n  "))));
        }

        // Continue with existing basic validation for backwards compatibility
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
        
        // Validate classes
        self.validate_classes()?;
        
        // Validate allow blocks
        self.validate_allow_blocks()?;
        
        // Validate server link classes
        self.validate_server_link_classes()?;
        
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

    /// Get service definition by name
    pub fn get_service(&self, name: &str) -> Option<&ServiceDefinition> {
        self.services.services.iter().find(|service| service.name == name && service.enabled)
    }

    /// Get service definition by type
    pub fn get_services_by_type(&self, service_type: &str) -> Vec<&ServiceDefinition> {
        self.services.services.iter()
            .filter(|service| service.service_type == service_type && service.enabled)
            .collect()
    }

    /// Check if a service is enabled
    pub fn is_service_enabled(&self, name: &str) -> bool {
        self.services.services.iter().any(|service| service.name == name && service.enabled)
    }

    /// Get all enabled services
    pub fn get_enabled_services(&self) -> Vec<&ServiceDefinition> {
        self.services.services.iter().filter(|service| service.enabled).collect()
    }

    /// Get connection class by name
    pub fn get_class(&self, name: &str) -> Option<&ConnectionClass> {
        self.classes.iter().find(|class| class.name == name)
    }

    /// Get the bind address for a specific port
    pub fn get_bind_address_for_port(&self, port_config: &PortConfig) -> String {
        port_config.bind_address.as_ref()
            .unwrap_or(&self.connection.bind_address)
            .clone()
    }

    /// Find allow block that matches a host or IP
    pub fn find_allow_block(&self, host: &str, ip: &str) -> Option<&AllowBlock> {
        for allow_block in &self.security.allow_blocks {
            // Check if host matches any pattern in the allow block
            for pattern in &allow_block.hosts {
                if self.matches_host_pattern(host, pattern) {
                    return Some(allow_block);
                }
            }
            
            // Check if IP matches any pattern in the allow block
            for pattern in &allow_block.ips {
                if self.matches_ip_pattern(ip, pattern) {
                    return Some(allow_block);
                }
            }
        }
        
        None
    }

    /// Check if a host matches a pattern (simple wildcard matching)
    pub fn matches_host_pattern(&self, host: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                if parts[0].is_empty() {
                    return host.ends_with(parts[1]);
                } else if parts[1].is_empty() {
                    return host.starts_with(parts[0]);
                } else {
                    return host.starts_with(parts[0]) && host.ends_with(parts[1]);
                }
            }
        }
        
        host == pattern
    }

    /// Check if an IP matches a pattern (supports CIDR notation)
    fn matches_ip_pattern(&self, ip: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        // For now, simple exact match - could be enhanced with CIDR support
        if pattern.contains('/') {
            // CIDR notation - simplified check for now
            // In production, you'd want to use a proper CIDR library
            let parts: Vec<&str> = pattern.split('/').collect();
            if parts.len() == 2 {
                let network = parts[0];
                // Simple prefix matching for now
                return ip.starts_with(network);
            }
        }
        
        ip == pattern || self.matches_host_pattern(ip, pattern)
    }

    /// Validate classes configuration
    fn validate_classes(&self) -> Result<()> {
        let mut seen_names = std::collections::HashSet::new();
        
        for class in &self.classes {
            // Check for duplicate class names
            if seen_names.contains(&class.name) {
                return Err(Error::Config(format!("Duplicate class name: {}", class.name)));
            }
            seen_names.insert(class.name.clone());
            
            // Validate class configuration
            if class.name.is_empty() {
                return Err(Error::Config("Class name cannot be empty".to_string()));
            }
            
            // Validate sendq and recvq sizes
            if let Some(sendq) = class.max_sendq {
                if sendq == 0 {
                    return Err(Error::Config(format!("Class {} max_sendq cannot be 0", class.name)));
                }
            }
            
            if let Some(recvq) = class.max_recvq {
                if recvq == 0 {
                    return Err(Error::Config(format!("Class {} max_recvq cannot be 0", class.name)));
                }
            }
            
            // Validate timeouts
            if let Some(ping_freq) = class.ping_frequency {
                if ping_freq == 0 {
                    return Err(Error::Config(format!("Class {} ping_frequency cannot be 0", class.name)));
                }
            }
            
            if let Some(timeout) = class.connection_timeout {
                if timeout == 0 {
                    return Err(Error::Config(format!("Class {} connection_timeout cannot be 0", class.name)));
                }
            }
        }
        
        Ok(())
    }

    /// Validate allow blocks configuration
    fn validate_allow_blocks(&self) -> Result<()> {
        for (idx, allow_block) in self.security.allow_blocks.iter().enumerate() {
            // Validate that the referenced class exists
            if !self.classes.iter().any(|c| c.name == allow_block.class) {
                return Err(Error::Config(format!(
                    "Allow block {} references non-existent class: {}",
                    idx, allow_block.class
                )));
            }
            
            // Validate that at least one host or IP pattern is specified
            if allow_block.hosts.is_empty() && allow_block.ips.is_empty() {
                return Err(Error::Config(format!(
                    "Allow block {} must have at least one host or IP pattern",
                    idx
                )));
            }
            
            // Validate host patterns
            for host in &allow_block.hosts {
                if !self.is_valid_host_pattern(host) {
                    return Err(Error::Config(format!(
                        "Invalid host pattern in allow block {}: {}",
                        idx, host
                    )));
                }
            }
            
            // Validate IP patterns
            for ip in &allow_block.ips {
                if !self.is_valid_ip_pattern(ip) {
                    return Err(Error::Config(format!(
                        "Invalid IP pattern in allow block {}: {}",
                        idx, ip
                    )));
                }
            }
        }
        
        Ok(())
    }

    /// Validate IP pattern (basic check)
    fn is_valid_ip_pattern(&self, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        // Basic validation - allow wildcards, dots, numbers, and CIDR notation
        pattern.chars().all(|c| c.is_numeric() || c == '.' || c == '*' || c == '/' || c == ':')
    }

    /// Validate server links reference valid classes
    fn validate_server_link_classes(&self) -> Result<()> {
        for link in &self.network.links {
            if let Some(class_name) = &link.class {
                if !self.classes.iter().any(|c| &c.name == class_name) {
                    return Err(Error::Config(format!(
                        "Server link {} references non-existent class: {}",
                        link.name, class_name
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_rate_limit_config_defaults() {
        let config = CommandRateLimitConfig::default();

        assert_eq!(config.enabled, true, "Rate limiting should be enabled by default");
        assert_eq!(config.max_commands, 10, "Default max commands should be 10");
        assert_eq!(config.time_window_seconds, 60, "Default time window should be 60 seconds");
        assert_eq!(config.limited_commands.len(), 0, "Default should rate limit ALL commands (empty list)");
        assert_eq!(config.exempt_operators, true, "Operators should be exempt by default");
        assert!(matches!(config.limit_action, RateLimitAction::SendError), "Default action should be SendError");
    }

    #[test]
    fn test_rate_limit_action_serialization() {
        // Test Drop
        let drop_json = serde_json::to_string(&RateLimitAction::Drop).unwrap();
        assert_eq!(drop_json, "\"Drop\"");
        let drop_back: RateLimitAction = serde_json::from_str(&drop_json).unwrap();
        assert!(matches!(drop_back, RateLimitAction::Drop));

        // Test SendError
        let error_json = serde_json::to_string(&RateLimitAction::SendError).unwrap();
        assert_eq!(error_json, "\"SendError\"");
        let error_back: RateLimitAction = serde_json::from_str(&error_json).unwrap();
        assert!(matches!(error_back, RateLimitAction::SendError));

        // Test Mute
        let mute_json = serde_json::to_string(&RateLimitAction::Mute).unwrap();
        assert_eq!(mute_json, "\"Mute\"");
        let mute_back: RateLimitAction = serde_json::from_str(&mute_json).unwrap();
        assert!(matches!(mute_back, RateLimitAction::Mute));
    }

    #[test]
    fn test_command_rate_limit_config_toml_parsing() {
        let toml_str = r#"
            enabled = true
            max_commands = 20
            time_window_seconds = 120
            limited_commands = []
            exempt_operators = false
            limit_action = "Drop"
        "#;

        let config: CommandRateLimitConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.enabled, true);
        assert_eq!(config.max_commands, 20);
        assert_eq!(config.time_window_seconds, 120);
        assert_eq!(config.limited_commands.len(), 0);
        assert_eq!(config.exempt_operators, false);
        assert!(matches!(config.limit_action, RateLimitAction::Drop));
    }

    #[test]
    fn test_command_rate_limit_config_selective_commands() {
        let toml_str = r#"
            enabled = true
            max_commands = 5
            time_window_seconds = 30
            limited_commands = ["PRIVMSG", "NOTICE", "NICK"]
            exempt_operators = true
            limit_action = "Mute"
        "#;

        let config: CommandRateLimitConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.enabled, true);
        assert_eq!(config.max_commands, 5);
        assert_eq!(config.time_window_seconds, 30);
        assert_eq!(config.limited_commands.len(), 3);
        assert_eq!(config.limited_commands[0], "PRIVMSG");
        assert_eq!(config.limited_commands[1], "NOTICE");
        assert_eq!(config.limited_commands[2], "NICK");
        assert_eq!(config.exempt_operators, true);
        assert!(matches!(config.limit_action, RateLimitAction::Mute));
    }

    #[test]
    fn test_module_config_includes_rate_limiting() {
        let toml_str = r#"
            module_directory = "modules"
            enabled_modules = []
            module_settings = {}

            [throttling]
            enabled = false
            max_connections_per_ip = 5
            time_window_seconds = 60
            initial_throttle_seconds = 10
            max_stages = 10
            stage_factor = 10
            cleanup_interval_seconds = 300

            [command_rate_limiting]
            enabled = true
            max_commands = 15
            time_window_seconds = 90
            limited_commands = []
            exempt_operators = true
            limit_action = "SendError"

            [messaging]
            enabled = true

            [messaging.wallops]
            enabled = true
            require_operator = true
            receiver_mode = "w"
            self_only_mode = true
            mode_requires_operator = false

            [messaging.globops]
            enabled = true
            require_operator = true
            receiver_mode = "g"
            self_only_mode = false
            mode_requires_operator = true
        "#;

        let config: ModuleConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.module_directory, "modules");
        assert_eq!(config.command_rate_limiting.enabled, true);
        assert_eq!(config.command_rate_limiting.max_commands, 15);
        assert_eq!(config.command_rate_limiting.time_window_seconds, 90);
        assert_eq!(config.command_rate_limiting.limited_commands.len(), 0);
        assert_eq!(config.command_rate_limiting.exempt_operators, true);
        assert!(matches!(config.command_rate_limiting.limit_action, RateLimitAction::SendError));
    }

    #[test]
    fn test_command_rate_limit_config_disabled() {
        let toml_str = r#"
            enabled = false
            max_commands = 10
            time_window_seconds = 60
            limited_commands = []
            exempt_operators = true
            limit_action = "SendError"
        "#;

        let config: CommandRateLimitConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.enabled, false);
    }

    #[test]
    fn test_empty_limited_commands_means_all() {
        let config = CommandRateLimitConfig::default();

        // Empty limited_commands should mean ALL commands are rate limited
        assert!(
            config.limited_commands.is_empty(),
            "Empty limited_commands list is the most secure option - all commands are rate limited"
        );
    }

    #[test]
    fn test_rate_limit_action_all_variants() {
        let actions = vec![
            RateLimitAction::Drop,
            RateLimitAction::SendError,
            RateLimitAction::Mute,
        ];

        for action in actions {
            // Ensure all actions can be serialized and deserialized
            let json = serde_json::to_string(&action).unwrap();
            let _deserialized: RateLimitAction = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_argon2_password_hashing() {
        let password = "test_password_123";
        let hash = PasswordHasher::hash_password(password);

        // Verify Argon2 hash format
        assert!(hash.starts_with("$argon2"), "Hash should start with $argon2");
        assert!(PasswordHasher::is_argon2_hash(&hash), "Should be identified as Argon2 hash");

        // Verify password verification works
        assert!(PasswordHasher::verify_password(password, &hash), "Password verification should succeed");
        assert!(!PasswordHasher::verify_password("wrong_password", &hash), "Wrong password should fail");
    }

    #[test]
    fn test_sha256_password_backward_compatibility() {
        // Legacy SHA-256 hash for "password" (for testing backward compatibility)
        #[allow(deprecated)]
        let legacy_hash = PasswordHasher::hash_password_sha256("password");

        assert!(PasswordHasher::is_sha256_hash(&legacy_hash), "Should be identified as SHA-256 hash");
        assert_eq!(legacy_hash.len(), 64, "SHA-256 hash should be 64 characters");
        assert!(legacy_hash.chars().all(|c| c.is_ascii_hexdigit()), "Should be hex string");

        // Verify password verification works with legacy hashes
        assert!(PasswordHasher::verify_password("password", &legacy_hash), "Legacy password verification should succeed");
        assert!(!PasswordHasher::verify_password("wrong", &legacy_hash), "Wrong password should fail");
    }

    #[test]
    fn test_password_hash_format_detection() {
        let argon2_hash = PasswordHasher::hash_password("test");
        assert!(PasswordHasher::is_argon2_hash(&argon2_hash));
        assert!(!PasswordHasher::is_sha256_hash(&argon2_hash));

        #[allow(deprecated)]
        let sha256_hash = PasswordHasher::hash_password_sha256("test");
        assert!(!PasswordHasher::is_argon2_hash(&sha256_hash));
        assert!(PasswordHasher::is_sha256_hash(&sha256_hash));

        let invalid_hash = "not_a_real_hash";
        assert!(!PasswordHasher::is_argon2_hash(invalid_hash));
        assert!(!PasswordHasher::is_sha256_hash(invalid_hash));
    }

    #[test]
    fn test_argon2_unique_salts() {
        let password = "same_password";
        let hash1 = PasswordHasher::hash_password(password);
        let hash2 = PasswordHasher::hash_password(password);

        // Same password should produce different hashes due to unique salts
        assert_ne!(hash1, hash2, "Each hash should have a unique salt");

        // But both should verify correctly
        assert!(PasswordHasher::verify_password(password, &hash1));
        assert!(PasswordHasher::verify_password(password, &hash2));
    }
}
