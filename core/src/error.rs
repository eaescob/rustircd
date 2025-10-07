//! Error types for the IRC daemon

use thiserror::Error;

/// Main error type for the IRC daemon
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Message parsing error: {0}")]
    MessageParse(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Module error: {0}")]
    Module(String),
    
    #[error("Channel error: {0}")]
    Channel(String),
    
    #[error("User error: {0}")]
    User(String),
    
    #[error("Server error: {0}")]
    Server(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Generic(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Generic(s)
    }
}
