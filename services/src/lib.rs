//! Rust IRC Daemon Services Framework
//! 
//! This crate provides a framework for implementing IRC services.

pub mod framework;
pub mod example;

pub use framework::{Service, ServiceManager, ServiceResult};
