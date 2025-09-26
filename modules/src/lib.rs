//! Rust IRC Daemon Modules
//! 
//! This crate provides modules and extensions for the IRC daemon.

pub mod channel;
pub mod ircv3;
pub mod optional;

pub use channel::{ChannelModule, Channel, ChannelMember, ChannelMode};
pub use ircv3::Ircv3Module;
pub use optional::OptionalModule;
