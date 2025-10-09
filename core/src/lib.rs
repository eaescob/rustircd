//! Rust IRC Daemon Core
//! 
//! This crate provides the core functionality for a modular IRC daemon implementation
//! based on RFC 1459 and IRCv3 specifications.

pub mod client;
pub mod config;
pub mod connection;
pub mod server_connection;
pub mod error;
pub mod message;
pub mod module;
pub mod server;
pub mod user;
pub mod user_modes;
pub mod extensible_modes;
pub mod numeric;
pub mod replies_config;
pub mod utils;
pub mod database;
pub mod broadcast;
pub mod network;
pub mod throttling_manager;
pub mod statistics;
pub mod motd;
pub mod lookup;
pub mod module_numerics;
pub mod rehash;
pub mod buffer;
pub mod class_tracker;
pub mod validation;
pub mod cache;
pub mod batch_optimizer;

#[cfg(test)]
mod tests;

pub use client::Client;
pub use config::Config;
// pub use connection::Connection; // Commented out - Connection is not exported from connection module
pub use server_connection::{ServerConnection, ServerConnectionManager, ServerInfo, ServerConnectionState};
pub use error::{Error, Result};
pub use message::{Message, MessageType, Prefix};
pub use module::{Module, ModuleManager};
pub use server::Server;
pub use user::{User, UserState};
pub use user_modes::{UserMode, UserModeManager};
pub use extensible_modes::{
    CustomUserMode, ExtensibleModeRegistry,
    register_custom_mode, unregister_custom_mode,
    is_valid_user_mode, get_custom_mode,
    validate_custom_mode_change, get_all_custom_modes,
    get_custom_modes_by_module
};
pub use numeric::NumericReply;
pub use replies_config::{RepliesConfig, ReplyConfig, ServerInfo as RepliesServerInfo};
pub use database::{Database, DatabaseConfig, UserHistoryEntry, ServerInfo as DatabaseServerInfo, ChannelInfo};
pub use broadcast::{BroadcastSystem, BroadcastTarget, BroadcastMessage, BroadcastPriority, MessageBuilder};
pub use network::{NetworkQueryManager, NetworkMessageHandler, NetworkQuery, NetworkResponse, NetworkMessage};
pub use throttling_manager::ThrottlingManager;
pub use statistics::{StatisticsManager, ServerStatistics, CommandStats};
pub use motd::MotdManager;
pub use lookup::{LookupService, DnsResolver, IdentClient, LookupResult, IdentResult};
pub use module_numerics::{ModuleNumericManager, ModuleNumeric, ModuleNumericClient};
pub use rehash::RehashService;
pub use buffer::{SendQueue, RecvQueue, ConnectionTiming};
pub use class_tracker::{ClassTracker, ClassStats};
pub use validation::{ConfigValidator, ValidationResult, ValidationError, ValidationWarning, ErrorCategory, print_validation_result};
pub use cache::{LruCache, MessageCache, DnsCache, ChannelMemberCache, UserLookupCache, CacheStats};
pub use batch_optimizer::{BatchOptimizer, BatchConfig, MessageBatch, BatchStats, ConnectionPool, ConnectionPoolStats};

/// Re-exports for convenience
pub use async_trait::async_trait;
pub use serde::{Deserialize, Serialize};
pub use tracing::{debug, error, info, warn};
