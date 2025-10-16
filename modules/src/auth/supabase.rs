//! Supabase authentication provider
//! 
//! This module provides authentication capabilities that integrate with
//! Supabase backend-as-a-service platform for user authentication.

use rustircd_core::{Result, Error, AuthProvider, AuthResult, AuthInfo, AuthRequest, AuthProviderCapabilities};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Supabase authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseAuthConfig {
    /// Supabase project URL
    pub project_url: String,
    /// Supabase API key (anon key or service role key)
    pub api_key: String,
    /// Database table name for users (default: "users")
    pub user_table: Option<String>,
    /// Username column name (default: "username")
    pub username_column: Option<String>,
    /// Password column name (default: "password_hash")
    pub password_column: Option<String>,
    /// Email column name (default: "email")
    pub email_column: Option<String>,
    /// Whether to use email for authentication instead of username
    pub use_email_auth: bool,
    /// Connection timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Maximum number of concurrent connections
    pub max_connections: Option<usize>,
}

impl Default for SupabaseAuthConfig {
    fn default() -> Self {
        Self {
            project_url: "https://your-project.supabase.co".to_string(),
            api_key: "your-anon-key".to_string(),
            user_table: Some("users".to_string()),
            username_column: Some("username".to_string()),
            password_column: Some("password_hash".to_string()),
            email_column: Some("email".to_string()),
            use_email_auth: false,
            timeout_seconds: Some(30),
            max_connections: Some(10),
        }
    }
}

/// Supabase authentication statistics
#[derive(Debug, Default, Clone)]
struct SupabaseAuthStats {
    /// Total authentication attempts
    total_attempts: u64,
    /// Successful authentications
    successful: u64,
    /// Failed authentications
    failed: u64,
    /// Network errors
    network_errors: u64,
    /// Database errors
    database_errors: u64,
    /// Last authentication timestamp
    last_auth: Option<chrono::DateTime<chrono::Utc>>,
}

/// Supabase authentication provider
pub struct SupabaseAuthProvider {
    /// Supabase configuration
    config: SupabaseAuthConfig,
    /// HTTP client for API requests
    client: reqwest::Client,
    /// Authentication statistics
    stats: Arc<RwLock<SupabaseAuthStats>>,
    /// Connection pool for database queries
    connections: Arc<RwLock<Vec<SupabaseConnection>>>,
}

/// Supabase database connection
#[derive(Debug, Clone)]
struct SupabaseConnection {
    /// Connection ID
    id: uuid::Uuid,
    /// Connection state
    state: SupabaseConnectionState,
    /// Last used timestamp
    last_used: chrono::DateTime<chrono::Utc>,
}

/// Supabase connection state
#[derive(Debug, Clone)]
enum SupabaseConnectionState {
    /// Connected and ready
    Ready,
    /// Connected and authenticated
    Authenticated,
    /// Disconnected
    Disconnected,
    /// Error state
    Error(String),
}

impl SupabaseAuthProvider {
    /// Create a new Supabase authentication provider
    pub fn new(config: SupabaseAuthConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds.unwrap_or(30)))
            .build()
            .map_err(|e| Error::Auth(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            stats: Arc::new(RwLock::new(SupabaseAuthStats::default())),
            connections: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Get authentication statistics
    pub async fn get_stats(&self) -> SupabaseAuthStats {
        self.stats.read().await.clone()
    }

    /// Authenticate user against Supabase
    async fn authenticate_user(&self, username: &str, password: &str) -> Result<AuthInfo> {
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_attempts += 1;
            stats.last_auth = Some(chrono::Utc::now());
        }

        // Determine the authentication field
        let email_default = "email".to_string();
        let username_default = "username".to_string();
        let users_default = "users".to_string();
        let password_default = "password_hash".to_string();
        
        let auth_field = if self.config.use_email_auth {
            self.config.email_column.as_ref().unwrap_or(&email_default)
        } else {
            self.config.username_column.as_ref().unwrap_or(&username_default)
        };

        let user_table = self.config.user_table.as_ref().unwrap_or(&users_default);
        let password_column = self.config.password_column.as_ref().unwrap_or(&password_default);

        // Query Supabase for user
        let query_url = format!(
            "{}/rest/v1/{}?{}=eq.{}&select={},{}",
            self.config.project_url,
            user_table,
            auth_field,
            username,
            auth_field,
            password_column
        );

        let response = self.client
            .get(&query_url)
            .header("apikey", &self.config.api_key)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| {
                Error::Auth(format!("Network error: {}", e))
            })?;

        if !response.status().is_success() {
            let mut stats = self.stats.write().await;
            stats.network_errors += 1;
            return Err(Error::Auth(format!("Supabase API error: {}", response.status())));
        }

        let users: Vec<HashMap<String, serde_json::Value>> = response
            .json()
            .await
            .map_err(|e| Error::Auth(format!("Failed to parse response: {}", e)))?;

        if users.is_empty() {
            let mut stats = self.stats.write().await;
            stats.failed += 1;
            return Err(Error::Auth("User not found".to_string()));
        }

        let user = &users[0];
        let stored_hash = user.get(password_column)
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Auth("Password hash not found".to_string()))?;

        // Verify password (in a real implementation, you'd use proper password hashing)
        if self.verify_password(password, stored_hash)? {
            let mut stats = self.stats.write().await;
            stats.successful += 1;

            // Extract user information
            let realname = user.get(self.config.email_column.as_ref().unwrap_or(&"email".to_string()))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut metadata = HashMap::new();
            metadata.insert("provider".to_string(), "supabase".to_string());
            metadata.insert("auth_method".to_string(), "database".to_string());

            Ok(AuthInfo {
                username: username.to_string(),
                realname: Some(realname),
                hostname: None,
                metadata,
                provider: "supabase".to_string(),
                authenticated_at: chrono::Utc::now(),
            })
        } else {
            let mut stats = self.stats.write().await;
            stats.failed += 1;
            Err(Error::Auth("Invalid password".to_string()))
        }
    }

    /// Verify password against stored hash
    fn verify_password(&self, password: &str, stored_hash: &str) -> Result<bool> {
        // In a real implementation, you would use a proper password hashing library
        // like argon2, bcrypt, or scrypt. For demonstration, we'll do a simple comparison
        // WARNING: This is NOT secure for production use!
        
        // For Supabase, you might want to use their built-in auth system instead
        // This is just an example for custom user tables
        
        // Simple hash verification (replace with proper implementation)
        Ok(password == stored_hash)
    }

    /// Get a connection from the pool
    async fn get_connection(&self) -> Result<SupabaseConnection> {
        let mut connections = self.connections.write().await;
        
        // Find an available connection
        for conn in connections.iter_mut() {
            if matches!(conn.state, SupabaseConnectionState::Ready) {
                conn.last_used = chrono::Utc::now();
                return Ok(conn.clone());
            }
        }

        // Create a new connection if none available
        let new_conn = SupabaseConnection {
            id: Uuid::new_v4(),
            state: SupabaseConnectionState::Ready,
            last_used: chrono::Utc::now(),
        };

        connections.push(new_conn.clone());
        Ok(new_conn)
    }

    /// Test connection to Supabase
    async fn test_connection(&self) -> Result<bool> {
        let test_url = format!("{}/rest/v1/", self.config.project_url);
        
        let response = self.client
            .get(&test_url)
            .header("apikey", &self.config.api_key)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[async_trait]
impl AuthProvider for SupabaseAuthProvider {
    fn name(&self) -> &str {
        "supabase"
    }

    fn description(&self) -> &str {
        "Supabase backend-as-a-service authentication provider"
    }

    fn capabilities(&self) -> AuthProviderCapabilities {
        AuthProviderCapabilities {
            password_auth: true,
            certificate_auth: false,
            token_auth: false,
            challenge_response: false,
            account_validation: true,
        }
    }

    async fn is_available(&self) -> bool {
        self.test_connection().await.unwrap_or(false)
    }

    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult> {
        tracing::info!("Authenticating user '{}' against Supabase", request.username);

        match self.authenticate_user(&request.username, &request.credential).await {
            Ok(auth_info) => {
                tracing::info!("Supabase authentication successful for user: {}", auth_info.username);
                Ok(AuthResult::Success(auth_info))
            }
            Err(e) => {
                tracing::warn!("Supabase authentication failed for user '{}': {}", request.username, e);
                Ok(AuthResult::Failure(e.to_string()))
            }
        }
    }

    async fn validate(&self, auth_info: &AuthInfo) -> Result<bool> {
        // Check if the auth info is from this provider
        if auth_info.provider != "supabase" {
            return Ok(false);
        }

        // In a real implementation, you might want to verify the user still exists
        // and is still active in Supabase
        Ok(true)
    }
}

/// Builder for Supabase authentication provider
pub struct SupabaseAuthProviderBuilder {
    config: SupabaseAuthConfig,
}

impl SupabaseAuthProviderBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: SupabaseAuthConfig::default(),
        }
    }

    /// Set the Supabase project URL
    pub fn project_url(mut self, url: impl Into<String>) -> Self {
        self.config.project_url = url.into();
        self
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = key.into();
        self
    }

    /// Set the user table name
    pub fn user_table(mut self, table: impl Into<String>) -> Self {
        self.config.user_table = Some(table.into());
        self
    }

    /// Set the username column name
    pub fn username_column(mut self, column: impl Into<String>) -> Self {
        self.config.username_column = Some(column.into());
        self
    }

    /// Set the password column name
    pub fn password_column(mut self, column: impl Into<String>) -> Self {
        self.config.password_column = Some(column.into());
        self
    }

    /// Set the email column name
    pub fn email_column(mut self, column: impl Into<String>) -> Self {
        self.config.email_column = Some(column.into());
        self
    }

    /// Enable email-based authentication
    pub fn use_email_auth(mut self, use_email: bool) -> Self {
        self.config.use_email_auth = use_email;
        self
    }

    /// Set connection timeout
    pub fn timeout_seconds(mut self, timeout: u64) -> Self {
        self.config.timeout_seconds = Some(timeout);
        self
    }

    /// Set maximum connections
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = Some(max);
        self
    }

    /// Build the authentication provider
    pub fn build(self) -> Result<SupabaseAuthProvider> {
        SupabaseAuthProvider::new(self.config)
    }
}

impl Default for SupabaseAuthProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}
