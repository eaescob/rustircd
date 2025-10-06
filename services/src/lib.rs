//! Rust IRC Daemon Services Framework
//! 
//! This crate provides a framework for implementing IRC services.

pub mod framework;
pub mod example;
pub mod atheme;

pub use framework::{Service, ServiceManager, ServiceResult};
pub use atheme::{AthemeIntegration, AthemeConfig, AthemeConnection, AthemeConnectionState, AthemeStats, AthemeServicesModule, AthemeConfigBuilder};
