//! Supabase SASL Integration Example
//! 
//! This example demonstrates how to integrate Supabase authentication
//! with the rustircd SASL system for IRC user authentication.

use rustircd_core::{AuthManager, AuthRequest, ClientInfo};
use rustircd_modules::{
    SaslModule, SaslConfig, 
    SupabaseAuthProvider, SupabaseAuthConfig, SupabaseAuthProviderBuilder
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Supabase SASL Integration Example");
    println!("=====================================");

    // 1. Create Supabase authentication provider
    println!("\n📋 Setting up Supabase authentication provider...");
    
    let supabase_config = SupabaseAuthConfig {
        project_url: "https://your-project-id.supabase.co".to_string(),
        api_key: "your-supabase-anon-key".to_string(),
        user_table: Some("irc_users".to_string()),
        username_column: Some("username".to_string()),
        password_column: Some("password_hash".to_string()),
        email_column: Some("email".to_string()),
        use_email_auth: false, // Use username instead of email
        timeout_seconds: Some(30),
        max_connections: Some(10),
    };

    let supabase_provider = SupabaseAuthProvider::new(supabase_config)?;
    println!("   ✓ Supabase provider created");

    // 2. Create authentication manager
    println!("\n🔐 Setting up authentication manager...");
    
    let auth_manager = Arc::new(AuthManager::new(3600)); // 1 hour cache TTL
    
    // Register the Supabase provider
    auth_manager.register_provider(Arc::new(supabase_provider)).await?;
    
    // Set Supabase as the primary authentication provider
    auth_manager.set_primary_provider("supabase").await?;
    
    println!("   ✓ Authentication manager configured");
    println!("   ✓ Supabase set as primary provider");

    // 3. Create SASL module
    println!("\n🔑 Setting up SASL module...");
    
    let sasl_config = SaslConfig {
        mechanisms: vec!["PLAIN".to_string()],
        allow_insecure_mechanisms: false,
        max_failed_attempts: 3,
        session_timeout_seconds: 300,
    };

    let sasl_module = SaslModule::new(sasl_config, auth_manager);
    println!("   ✓ SASL module created");

    // 4. Demonstrate authentication flow
    println!("\n🧪 Testing authentication flow...");
    
    // Simulate a client authentication request
    let client_info = ClientInfo {
        id: Uuid::new_v4(),
        ip: "192.168.1.100".to_string(),
        hostname: Some("client.example.com".to_string()),
        secure: false,
    };

    let auth_request = AuthRequest {
        username: "testuser".to_string(),
        credential: "password123".to_string(),
        authzid: None,
        client_info: client_info.clone(),
        mechanism: "PLAIN".to_string(),
    };

    println!("   📤 Sending authentication request:");
    println!("      Username: {}", auth_request.username);
    println!("      Mechanism: {}", auth_request.mechanism);
    println!("      Client IP: {}", auth_request.client_info.ip);

    // Test the authentication
    let auth_result = sasl_module.authenticate(&client_info, &auth_request).await;
    
    match auth_result {
        Ok(result) => {
            match result {
                rustircd_modules::SaslResponse { 
                    response_type: rustircd_modules::SaslResponseType::Success, 
                    data, 
                    error 
                } => {
                    println!("   ✅ Authentication successful!");
                    if let Some(data) = data {
                        println!("      Response data: {}", data);
                    }
                }
                rustircd_modules::SaslResponse { 
                    response_type: rustircd_modules::SaslResponseType::Failure, 
                    data, 
                    error 
                } => {
                    println!("   ❌ Authentication failed!");
                    if let Some(error) = error {
                        println!("      Error: {}", error);
                    }
                }
                _ => {
                    println!("   ⏳ Authentication in progress...");
                }
            }
        }
        Err(e) => {
            println!("   💥 Authentication error: {}", e);
        }
    }

    // 5. Show provider statistics
    println!("\n📊 Authentication provider statistics:");
    
    let stats = auth_manager.get_provider_stats("supabase").await;
    if let Some(provider) = stats {
        println!("   Provider: {}", provider.name);
        println!("   Description: {}", provider.description);
        println!("   Available: {}", provider.available);
        println!("   Capabilities:");
        println!("      - Username auth: {}", provider.capabilities.username_auth);
        println!("      - Email auth: {}", provider.capabilities.email_auth);
        println!("      - Password auth: {}", provider.capabilities.password_auth);
        println!("      - Account validation: {}", provider.capabilities.account_validation);
    }

    // 6. Demonstrate builder pattern
    println!("\n🔧 Demonstrating builder pattern...");
    
    let supabase_provider_built = SupabaseAuthProviderBuilder::new()
        .project_url("https://another-project.supabase.co")
        .api_key("another-api-key")
        .user_table("custom_users")
        .username_column("login")
        .password_column("passwd")
        .email_column("email_address")
        .use_email_auth(true)
        .timeout_seconds(60)
        .max_connections(20)
        .build()?;
    
    println!("   ✓ Supabase provider created with builder pattern");

    // 7. Show configuration
    println!("\n⚙️  Configuration example:");
    println!("```toml");
    println!("[auth.supabase]");
    println!("project_url = \"https://your-project-id.supabase.co\"");
    println!("api_key = \"your-supabase-anon-key\"");
    println!("user_table = \"irc_users\"");
    println!("username_column = \"username\"");
    println!("password_column = \"password_hash\"");
    println!("email_column = \"email\"");
    println!("use_email_auth = false");
    println!("timeout_seconds = 30");
    println!("max_connections = 10");
    println!("```");

    // 8. Show SQL schema example
    println!("\n🗄️  Example Supabase table schema:");
    println!("```sql");
    println!("-- Create table for IRC users");
    println!("CREATE TABLE irc_users (");
    println!("  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,");
    println!("  username TEXT UNIQUE NOT NULL,");
    println!("  email TEXT UNIQUE NOT NULL,");
    println!("  password_hash TEXT NOT NULL,");
    println!("  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),");
    println!("  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),");
    println!("  is_active BOOLEAN DEFAULT true");
    println!(");");
    println!("");
    println!("-- Enable Row Level Security");
    println!("ALTER TABLE irc_users ENABLE ROW LEVEL SECURITY;");
    println!("");
    println!("-- Create policy for reading user data");
    println!("CREATE POLICY \"Allow read access for auth\" ON irc_users");
    println!("  FOR SELECT USING (true);");
    println!("```");

    println!("\n🎉 Supabase SASL integration example completed!");
    println!("\n📚 Next steps:");
    println!("   1. Set up your Supabase project");
    println!("   2. Create the irc_users table with the schema above");
    println!("   3. Configure your rustircd with the Supabase provider");
    println!("   4. Test SASL authentication with IRC clients");

    Ok(())
}

/// Helper function to demonstrate different configuration approaches
async fn demonstrate_configuration_variations() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 Configuration Variations:");
    
    // 1. Email-based authentication
    println!("\n1. Email-based authentication:");
    let email_config = SupabaseAuthConfig {
        project_url: "https://project.supabase.co".to_string(),
        api_key: "api-key".to_string(),
        user_table: Some("users".to_string()),
        username_column: Some("username".to_string()),
        password_column: Some("password_hash".to_string()),
        email_column: Some("email".to_string()),
        use_email_auth: true, // Use email for authentication
        timeout_seconds: Some(30),
        max_connections: Some(10),
    };
    let _email_provider = SupabaseAuthProvider::new(email_config)?;
    println!("   ✓ Email-based auth configured");

    // 2. Custom table and column names
    println!("\n2. Custom table and columns:");
    let custom_config = SupabaseAuthConfig {
        project_url: "https://project.supabase.co".to_string(),
        api_key: "api-key".to_string(),
        user_table: Some("custom_irc_accounts".to_string()),
        username_column: Some("login_name".to_string()),
        password_column: Some("encrypted_password".to_string()),
        email_column: Some("email_address".to_string()),
        use_email_auth: false,
        timeout_seconds: Some(60),
        max_connections: Some(5),
    };
    let _custom_provider = SupabaseAuthProvider::new(custom_config)?;
    println!("   ✓ Custom table/columns configured");

    // 3. High-performance configuration
    println!("\n3. High-performance configuration:");
    let perf_config = SupabaseAuthConfig {
        project_url: "https://project.supabase.co".to_string(),
        api_key: "api-key".to_string(),
        user_table: Some("users".to_string()),
        username_column: Some("username".to_string()),
        password_column: Some("password_hash".to_string()),
        email_column: Some("email".to_string()),
        use_email_auth: false,
        timeout_seconds: Some(10), // Faster timeout
        max_connections: Some(50), // More connections
    };
    let _perf_provider = SupabaseAuthProvider::new(perf_config)?;
    println!("   ✓ High-performance config applied");

    Ok(())
}
