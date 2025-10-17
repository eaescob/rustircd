//! Supabase SASL Integration Example
//! 
//! This example shows how to set up RustIRCd with Supabase authentication
//! using SASL for client authentication.

use rustircd_core::{Config, Server, AuthManager};
use rustircd_modules::{SaslModule, SaslConfig};
use rustircd_modules::auth::supabase::{SupabaseAuthProvider, SupabaseAuthConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load configuration
    let config = load_config().await?;
    
    // Create authentication manager
    let auth_manager = Arc::new(AuthManager::new(3600)); // 1 hour cache TTL
    
    // Configure Supabase authentication
    let supabase_config = SupabaseAuthConfig {
        project_url: "https://your-project-id.supabase.co".to_string(),
        api_key: std::env::var("SUPABASE_ANON_KEY")
            .expect("SUPABASE_ANON_KEY environment variable must be set"),
        user_table: Some("irc_users".to_string()),
        username_column: Some("username".to_string()),
        password_column: Some("password_hash".to_string()),
        email_column: Some("email".to_string()),
        use_email_auth: false,
        timeout_seconds: Some(30),
        max_connections: Some(10),
    };
    
    // Create Supabase authentication provider
    let supabase_provider = SupabaseAuthProvider::new(supabase_config)?;

    // Register Supabase as the primary authentication provider
    auth_manager.register_provider(Arc::new(supabase_provider)).await?;
    auth_manager.set_primary_provider("supabase").await?;
    
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
    let _server = Server::new(config).await;

    // Note: In a real implementation, you would need to integrate the SASL module
    // and authentication manager into the server's connection handling.
    // For this example, we're just demonstrating the configuration.
    println!("Server configured with Supabase authentication");
    println!("SASL module: {:?}", sasl_module.get_supported_mechanisms());

    // In a real implementation:
    // server.start().await?;
    
    Ok(())
}

async fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Load configuration from file or create default
    let config = Config::from_file("config.toml")
        .unwrap_or_else(|_| Config::default());

    Ok(config)
}

// Example Supabase table schema for IRC users
//
// ```sql
// CREATE TABLE irc_users (
//   id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
//   username TEXT UNIQUE NOT NULL,
//   email TEXT UNIQUE NOT NULL,
//   password_hash TEXT NOT NULL,
//   real_name TEXT,
//   hostname TEXT,
//   is_active BOOLEAN DEFAULT true,
//   created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
//   updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
// );
//
// -- Enable Row Level Security
// ALTER TABLE irc_users ENABLE ROW LEVEL SECURITY;
//
// -- Create policy for reading user data
// CREATE POLICY "Allow read access for auth" ON irc_users
//   FOR SELECT USING (true);
//
// -- Create policy for updating user data
// CREATE POLICY "Allow update access for auth" ON irc_users
//   FOR UPDATE USING (true);
// ```
//
// ## Authentication Flow
//
// 1. **Client connects** to RustIRCd
// 2. **Client sends** `AUTHENTICATE PLAIN <base64-credentials>`
// 3. **RustIRCd decodes** credentials and creates `AuthRequest`
// 4. **AuthManager calls** Supabase provider with credentials
// 5. **Supabase provider** queries the `irc_users` table
// 6. **Supabase returns** authentication result
// 7. **RustIRCd sends** `900 RPL_SASLSUCCESS` or error to client
//
// ## Configuration Example
//
// ```toml
// [server]
// name = "irc.example.com"
// description = "Example IRC Server with Supabase Auth"
//
// [modules.sasl]
// enabled = true
// mechanisms = ["PLAIN", "EXTERNAL"]
// sasl_service = "SaslServ"
// service_name = "services.example.org"
// require_sasl = false
// timeout_seconds = 300
//
// [auth]
// primary_provider = "supabase"
// cache_ttl_seconds = 3600
// max_cache_size = 10000
//
// [auth.supabase]
// enabled = true
// project_url = "https://your-project-id.supabase.co"
// api_key = "${SUPABASE_ANON_KEY}"
// user_table = "irc_users"
// username_column = "username"
// password_column = "password_hash"
// email_column = "email"
// use_email_auth = false
// timeout_seconds = 30
// max_connections = 10
// ```