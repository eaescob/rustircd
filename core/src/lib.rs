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
pub mod numeric;
pub mod replies_config;
pub mod utils;
pub mod database;
pub mod broadcast;
pub mod network;
pub mod extensions;
pub mod burst_extensions;
pub mod throttling_manager;
pub mod statistics;
pub mod motd;

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
pub use user::User;
pub use user_modes::{UserMode, UserModeManager};
pub use numeric::NumericReply;
pub use replies_config::{RepliesConfig, ReplyConfig, ServerInfo as RepliesServerInfo};
pub use database::{Database, DatabaseConfig, UserHistoryEntry, ServerInfo as DatabaseServerInfo, ChannelInfo};
pub use broadcast::{BroadcastSystem, BroadcastTarget, BroadcastMessage, BroadcastPriority, MessageBuilder};
pub use network::{NetworkQueryManager, NetworkMessageHandler, NetworkQuery, NetworkResponse, NetworkMessage};
pub use extensions::{ExtensionManager, UserExtension, MessageExtension, CapabilityExtension, MessageTagExtension, CapabilityAction, CapabilityResult, BurstExtension, BurstType};
pub use burst_extensions::{CoreUserBurstExtension, CoreServerBurstExtension};
pub use throttling_manager::ThrottlingManager;
pub use statistics::{StatisticsManager, ServerStatistics};
pub use motd::MotdManager;

/// Re-exports for convenience
pub use async_trait::async_trait;
pub use serde::{Deserialize, Serialize};
pub use tracing::{debug, error, info, warn};
