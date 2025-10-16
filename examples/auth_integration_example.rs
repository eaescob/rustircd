//! Authentication Integration Example
//! 
//! This example demonstrates how to set up and use the new authentication system
//! with SASL, services integration, and external authentication providers.

use rustircd_core::{AuthManager, Config};
use rustircd_modules::{SaslModule, SaslConfig, LdapAuthProvider, DatabaseAuthProvider, FileAuthProvider, HttpAuthProvider};
use rustircd_services::{ServicesAuthManager, ServiceContext, AthemeServicesModule};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCd Authentication Integration Example");
    println!("===========================================");
    
    // Create authentication manager
    let auth_manager = Arc::new(AuthManager::new(3600)); // 1 hour cache TTL
    
    // Example 1: Set up SASL with authentication providers
    setup_sasl_with_auth_providers(auth_manager.clone()).await?;
    
    // Example 2: Set up services integration
    setup_services_integration().await?;
    
    // Example 3: Set up external authentication providers
    setup_external_auth_providers(auth_manager.clone()).await?;
    
    // Example 4: Demonstrate authentication flow
    demonstrate_auth_flow(auth_manager.clone()).await?;
    
    println!("Authentication integration example completed successfully!");
    Ok(())
}

/// Example 1: Set up SASL with authentication providers
async fn setup_sasl_with_auth_providers(auth_manager: Arc<AuthManager>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n1. Setting up SASL with authentication providers...");
    
    // Create SASL configuration
    let sasl_config = SaslConfig {
        enabled: true,
        mechanisms: vec!["PLAIN".to_string(), "EXTERNAL".to_string()],
        service_name: "services.example.org".to_string(),
        require_sasl: false,
        timeout_seconds: 300,
    };
    
    // Create SASL module with authentication manager
    let sasl_module = SaslModule::new(sasl_config, auth_manager.clone());
    
    println!("   ✓ SASL module created with authentication manager");
    println!("   ✓ Supported mechanisms: {}", sasl_module.get_supported_mechanisms().join(", "));
    
    Ok(())
}

/// Example 2: Set up services integration
async fn setup_services_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n2. Setting up services integration...");
    
    // Create service context (simplified for example)
    let service_context = Arc::new(ServiceContext::new(
        Arc::new(rustircd_core::Database::new(rustircd_core::DatabaseConfig::default())),
        Arc::new(rustircd_core::ServerConnectionManager::new()),
    ));
    
    // Create services authentication manager
    let services_auth_manager = ServicesAuthManager::new(
        Arc::new(AuthManager::new(3600)),
        service_context,
    );
    
    // Register Atheme provider
    services_auth_manager.register_atheme_provider(None).await?;
    println!("   ✓ Atheme authentication provider registered");
    
    // Register generic services provider
    services_auth_manager.register_services_provider("charybdis".to_string()).await?;
    println!("   ✓ Generic services authentication provider registered");
    
    Ok(())
}

/// Example 3: Set up external authentication providers
async fn setup_external_auth_providers(auth_manager: Arc<AuthManager>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. Setting up external authentication providers...");
    
    // LDAP Provider
    let ldap_config = rustircd_modules::auth::ldap::LdapConfig {
        hostname: "ldap.example.com".to_string(),
        port: 389,
        base_dn: "dc=example,dc=com".to_string(),
        bind_dn: Some("cn=admin,dc=example,dc=com".to_string()),
        bind_password: Some("admin_password".to_string()),
        user_filter: "(uid={username})".to_string(),
        use_tls: false,
        timeout_seconds: 30,
        max_connections: 10,
    };
    let ldap_provider = Arc::new(LdapAuthProvider::new(ldap_config));
    auth_manager.register_provider(ldap_provider).await?;
    println!("   ✓ LDAP authentication provider registered");
    
    // Database Provider
    let db_config = rustircd_modules::auth::database::DatabaseAuthConfig {
        connection_string: "sqlite://users.db".to_string(),
        users_table: "users".to_string(),
        username_column: "username".to_string(),
        password_column: "password".to_string(),
        realname_column: Some("realname".to_string()),
        hostname_column: Some("hostname".to_string()),
        metadata_columns: vec!["email".to_string()],
        password_hash: rustircd_modules::auth::database::PasswordHashType::Sha256,
        timeout_seconds: 30,
    };
    let db_provider = Arc::new(DatabaseAuthProvider::new(db_config));
    auth_manager.register_provider(db_provider).await?;
    println!("   ✓ Database authentication provider registered");
    
    // File Provider
    let file_config = rustircd_modules::auth::file::FileAuthConfig {
        user_file: std::path::PathBuf::from("users.txt"),
        format: rustircd_modules::auth::file::FileFormat::Plain,
        password_hash: rustircd_modules::auth::file::PasswordHashType::Sha256,
        cache_ttl: 300,
        auto_reload: true,
    };
    let file_provider = Arc::new(FileAuthProvider::new(file_config));
    auth_manager.register_provider(file_provider).await?;
    println!("   ✓ File authentication provider registered");
    
    // HTTP Provider
    let http_config = rustircd_modules::auth::http::HttpAuthConfig {
        base_url: "https://auth.example.com".to_string(),
        auth_endpoint: "/authenticate".to_string(),
        validation_endpoint: Some("/validate".to_string()),
        method: rustircd_modules::auth::http::HttpMethod::Post,
        headers: {
            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            headers.insert("User-Agent".to_string(), "rustircd/1.0".to_string());
            headers
        },
        timeout_seconds: 30,
        verify_tls: true,
        username_field: "username".to_string(),
        password_field: "password".to_string(),
        response_format: rustircd_modules::auth::http::ResponseFormat::Json,
    };
    let http_provider = Arc::new(HttpAuthProvider::new(http_config));
    auth_manager.register_provider(http_provider).await?;
    println!("   ✓ HTTP authentication provider registered");
    
    // Set primary provider
    auth_manager.set_primary_provider("ldap").await?;
    println!("   ✓ LDAP set as primary authentication provider");
    
    // Add fallback providers
    auth_manager.add_fallback_provider("database").await?;
    auth_manager.add_fallback_provider("file").await?;
    auth_manager.add_fallback_provider("http").await?;
    println!("   ✓ Fallback providers configured");
    
    Ok(())
}

/// Example 4: Demonstrate authentication flow
async fn demonstrate_auth_flow(auth_manager: Arc<AuthManager>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n4. Demonstrating authentication flow...");
    
    // Show registered providers
    let providers = auth_manager.get_providers().await;
    println!("   Registered providers: {}", providers.join(", "));
    
    // Simulate authentication request
    let auth_request = rustircd_core::AuthRequest {
        username: "testuser".to_string(),
        credential: "testpassword".to_string(),
        authzid: None,
        client_info: rustircd_core::ClientInfo {
            id: uuid::Uuid::new_v4(),
            ip: "192.168.1.100".to_string(),
            hostname: Some("client.example.com".to_string()),
            secure: true,
        },
        context: HashMap::new(),
    };
    
    println!("   Simulating authentication for user: {}", auth_request.username);
    
    // Attempt authentication
    match auth_manager.authenticate(&auth_request).await? {
        rustircd_core::AuthResult::Success(auth_info) => {
            println!("   ✓ Authentication successful!");
            println!("     Username: {}", auth_info.username);
            println!("     Provider: {}", auth_info.provider);
            println!("     Real name: {:?}", auth_info.realname);
            println!("     Authenticated at: {}", auth_info.authenticated_at);
        }
        rustircd_core::AuthResult::Failure(reason) => {
            println!("   ✗ Authentication failed: {}", reason);
        }
        rustircd_core::AuthResult::Challenge(challenge) => {
            println!("   ? Authentication challenge: {}", challenge);
        }
        rustircd_core::AuthResult::InProgress => {
            println!("   ⏳ Authentication in progress...");
        }
    }
    
    Ok(())
}

/// Helper function to create a sample users file
fn create_sample_users_file() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create("users.txt")?;
    writeln!(file, "# Sample users file")?;
    writeln!(file, "# Format: username:password_hash:realname:hostname")?;
    writeln!(file, "alice:sha256_hash_here:Alice Smith:alice.example.com")?;
    writeln!(file, "bob:sha256_hash_here:Bob Jones:bob.example.com")?;
    writeln!(file, "charlie:sha256_hash_here:Charlie Brown:charlie.example.com")?;
    
    println!("   ✓ Sample users file created: users.txt");
    Ok(())
}
