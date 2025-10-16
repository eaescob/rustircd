//! Atheme SASL Integration Example
//! 
//! This example shows how to set up RustIRCd with Atheme services
//! for SASL authentication. Atheme provides services like NickServ, ChanServ,
//! and SaslServ that handle user authentication.

use rustircd_core::{Config, Server, AuthManager};
use rustircd_modules::{SaslModule, SaslConfig};
use rustircd_services::{AthemeIntegration, AthemeConfig, AthemeSaslAuthProvider};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load configuration
    let config = load_config().await?;
    
    // Create authentication manager
    let auth_manager = Arc::new(AuthManager::new(3600)); // 1 hour cache TTL
    
    // Configure Atheme services integration
    let atheme_config = AthemeConfig {
        enabled: true,
        service_name: "services.example.org".to_string(),
        hostname: "localhost".to_string(),
        port: 6666,
        password: std::env::var("ATHEME_PASSWORD")
            .unwrap_or_else(|_| "your_atheme_password".to_string()),
        tls: false,
        timeout_seconds: 30,
        reconnect_interval: 60,
        max_reconnect_attempts: 10,
        our_server_name: "irc.example.com".to_string(),
        sasl_service: "SaslServ".to_string(),
    };
    
    // Create Atheme integration
    let atheme_integration = Arc::new(AthemeIntegration::new(atheme_config));
    
    // Create Atheme SASL authentication provider
    let atheme_auth_provider = AthemeSaslAuthProvider::new(atheme_integration.clone());
    
    // Register Atheme as the primary authentication provider
    auth_manager.set_primary_provider("atheme_sasl").await?;
    auth_manager.add_provider("atheme_sasl", Box::new(atheme_auth_provider)).await?;
    
    // Configure SASL module
    let sasl_config = SaslConfig {
        enabled: true,
        mechanisms: vec!["PLAIN".to_string(), "EXTERNAL".to_string()],
        sasl_service: "SaslServ".to_string(),
        service_name: "services.example.org".to_string(),
        require_sasl: false,
        timeout_seconds: 300,
    };
    
    // Create SASL module with authentication manager
    let sasl_module = SaslModule::new(sasl_config, auth_manager);
    
    // Create and start server
    let mut server = Server::new(config).await?;
    
    // Load Atheme services module
    server.load_services_module(Box::new(atheme_integration)).await?;
    
    // Load SASL module
    server.load_module(Box::new(sasl_module)).await?;
    
    // Start the server
    server.start().await?;
    
    Ok(())
}

async fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Load configuration from file or create default
    let config = Config::load("atheme_sasl_config.toml").await
        .unwrap_or_else(|_| Config::default());
    
    Ok(config)
}

/// Example Atheme Services Setup
/// 
/// ## 1. Atheme Configuration (atheme.conf)
/// 
/// ```ini
/// [atheme]
/// uplink = "irc.example.com"
/// uplink_password = "your_atheme_password"
/// 
/// [services]
/// name = "services.example.org"
/// 
/// [sasl]
/// enabled = true
/// mechanisms = ["PLAIN", "EXTERNAL"]
/// 
/// [database]
/// type = "postgresql"
/// host = "localhost"
/// port = 5432
/// name = "atheme"
/// user = "atheme"
/// password = "atheme_password"
/// ```
/// 
/// ## 2. Database Schema
/// 
/// ```sql
/// -- Users table for authentication
/// CREATE TABLE users (
///     id SERIAL PRIMARY KEY,
///     username VARCHAR(50) UNIQUE NOT NULL,
///     password_hash VARCHAR(255) NOT NULL,
///     email VARCHAR(255),
///     real_name VARCHAR(255),
///     created_at TIMESTAMP DEFAULT NOW(),
///     last_login TIMESTAMP,
///     is_active BOOLEAN DEFAULT TRUE
/// );
/// 
/// -- Accounts table for IRC accounts
/// CREATE TABLE accounts (
///     id SERIAL PRIMARY KEY,
///     user_id INTEGER REFERENCES users(id),
///     account_name VARCHAR(50) UNIQUE NOT NULL,
///     created_at TIMESTAMP DEFAULT NOW(),
///     last_seen TIMESTAMP,
///     is_online BOOLEAN DEFAULT FALSE
/// );
/// 
/// -- Insert test user
/// INSERT INTO users (username, password_hash, email, real_name) 
/// VALUES ('testuser', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj4J/5Kz8K2', 'test@example.com', 'Test User');
/// ```
/// 
/// ## 3. Authentication Flow
/// 
/// ```
/// Client → AUTHENTICATE PLAIN <data> → RustIRCd → Atheme SaslServ → Database → Response
/// ```
/// 
/// 1. **Client connects** to RustIRCd
/// 2. **Client sends** `AUTHENTICATE PLAIN <base64-credentials>`
/// 3. **RustIRCd** forwards request to Atheme services
/// 4. **Atheme SaslServ** validates credentials against database
/// 5. **Atheme** sends response back to RustIRCd
/// 6. **RustIRCd** sends `900 RPL_SASLSUCCESS` or error to client
/// 
/// ## 4. Testing the Setup
/// 
/// ```bash
/// # Start Atheme services
/// ./atheme -f atheme.conf
/// 
/// # Start RustIRCd
/// ./rustircd --config atheme_sasl_config.toml
/// 
/// # Connect with IRC client
/// /connect irc.example.com 6667
/// 
/// # Start SASL authentication
/// /quote CAP REQ :sasl
/// /quote AUTHENTICATE PLAIN
/// /quote AUTHENTICATE <base64-encoded-credentials>
/// 
/// # Expected response: 900 RPL_SASLSUCCESS
/// ```
/// 
/// ## 5. Environment Variables
/// 
/// ```bash
/// export ATHEME_PASSWORD="your_atheme_password"
/// export RUST_LOG="info"
/// ```
/// 
/// ## 6. Required Atheme Modules
/// 
/// Make sure these modules are loaded in Atheme:
/// - `m_sasl.so` - SASL authentication module
/// - `m_sasl_plain.so` - PLAIN mechanism
/// - `m_sasl_external.so` - EXTERNAL mechanism
/// - `m_postgresql.so` - PostgreSQL database module