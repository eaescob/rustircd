# Adding New Authentication Modules

This guide explains how to add new authentication providers to the rustircd authentication system. The flexible authentication framework makes it easy to integrate with various external authentication services.

## Overview

The authentication system consists of:

- **AuthManager**: Central coordinator for multiple authentication providers
- **AuthProvider trait**: Interface that all authentication providers must implement
- **AuthRequest/AuthResult**: Data structures for authentication flow
- **SASL integration**: IRC SASL authentication support

## Step-by-Step Guide

### 1. Create the Authentication Provider Module

Create a new file in `modules/src/auth/` for your authentication provider:

```rust
// modules/src/auth/myprovider.rs
use rustircd_core::{Result, Error, AuthProvider, AuthResult, AuthInfo, AuthRequest, AuthProviderCapabilities};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for your authentication provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyProviderConfig {
    /// API endpoint URL
    pub endpoint: String,
    /// API key or token
    pub api_key: String,
    /// Timeout in seconds
    pub timeout: Option<u64>,
    // Add other configuration options as needed
}

impl Default for MyProviderConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://api.example.com".to_string(),
            api_key: "your-api-key".to_string(),
            timeout: Some(30),
        }
    }
}

/// Your authentication provider
pub struct MyProvider {
    config: MyProviderConfig,
    client: reqwest::Client,
    // Add any internal state you need
}

impl MyProvider {
    /// Create a new provider instance
    pub fn new(config: MyProviderConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout.unwrap_or(30)))
            .build()
            .map_err(|e| Error::Auth(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Authenticate user against your service
    async fn authenticate_user(&self, username: &str, password: &str) -> Result<AuthInfo> {
        // Implement your authentication logic here
        // This is where you make API calls to your service
        
        // Example API call
        let response = self.client
            .post(&format!("{}/auth", self.config.endpoint))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&serde_json::json!({
                "username": username,
                "password": password
            }))
            .send()
            .await
            .map_err(|e| Error::Auth(format!("API request failed: {}", e)))?;

        if response.status().is_success() {
            // Parse response and create AuthInfo
            let mut metadata = HashMap::new();
            metadata.insert("provider".to_string(), "myprovider".to_string());
            
            Ok(AuthInfo {
                username: username.to_string(),
                realname: None,
                hostname: None,
                metadata,
                provider: "myprovider".to_string(),
                authenticated_at: chrono::Utc::now(),
            })
        } else {
            Err(Error::Auth("Authentication failed".to_string()))
        }
    }
}

#[async_trait]
impl AuthProvider for MyProvider {
    fn name(&self) -> &str {
        "myprovider"
    }

    fn description(&self) -> &str {
        "My custom authentication provider"
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
        // Check if your service is available
        // You could ping an endpoint or check connectivity
        true
    }

    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult> {
        match self.authenticate_user(&request.username, &request.credential).await {
            Ok(auth_info) => Ok(AuthResult::Success(auth_info)),
            Err(e) => Ok(AuthResult::Failure(e.to_string())),
        }
    }

    async fn validate(&self, auth_info: &AuthInfo) -> Result<bool> {
        // Validate that the auth info is still valid
        // This is useful for checking if a user's account is still active
        Ok(auth_info.provider == "myprovider")
    }
}
```

### 2. Add to Module Exports

Update `modules/src/auth/mod.rs`:

```rust
pub mod myprovider;

pub use myprovider::MyProvider;
```

Update `modules/src/lib.rs`:

```rust
pub use auth::{MyProvider, /* other providers */};
```

### 3. Add Dependencies (if needed)

If your provider needs additional dependencies, add them to `modules/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
# Add any other dependencies your provider needs
```

### 4. Create Configuration

Add configuration options to your server configuration file:

```toml
[auth.myprovider]
endpoint = "https://api.example.com"
api_key = "your-api-key"
timeout = 30
```

### 5. Integration Example

Here's how to use your new provider in your IRC server:

```rust
use rustircd_core::AuthManager;
use rustircd_modules::MyProvider;

async fn setup_authentication() -> Result<(), Box<dyn std::error::Error>> {
    // Create authentication manager
    let auth_manager = Arc::new(AuthManager::new(3600)); // 1 hour cache
    
    // Create and register your provider
    let config = MyProviderConfig {
        endpoint: "https://api.example.com".to_string(),
        api_key: "your-api-key".to_string(),
        timeout: Some(30),
    };
    
    let provider = MyProvider::new(config)?;
    auth_manager.register_provider(Arc::new(provider)).await?;
    
    // Set as primary provider
    auth_manager.set_primary_provider("myprovider").await?;
    
    // Use with SASL module
    let sasl_config = SaslConfig {
        mechanisms: vec!["PLAIN".to_string()],
        allow_insecure_mechanisms: false,
        max_failed_attempts: 3,
        session_timeout_seconds: 300,
    };
    
    let sasl_module = SaslModule::new(sasl_config, auth_manager);
    
    Ok(())
}
```

## Real-World Examples

### Supabase Integration

The project includes a complete Supabase authentication provider (`modules/src/auth/supabase.rs`) that demonstrates:

- Database queries via Supabase REST API
- Configurable table and column names
- Email vs username authentication
- Connection pooling and error handling
- Builder pattern for configuration

### Other Existing Providers

The system includes several other authentication providers as examples:

- **LDAP** (`ldap.rs`): Active Directory/LDAP integration
- **Database** (`database.rs`): Direct database authentication
- **File** (`file.rs`): File-based user storage
- **HTTP** (`http.rs`): REST API authentication
- **Atheme** (in services): IRC services integration

## Best Practices

### 1. Error Handling

Always provide meaningful error messages:

```rust
.map_err(|e| Error::Auth(format!("Specific error context: {}", e)))?;
```

### 2. Configuration Validation

Validate configuration in your provider constructor:

```rust
pub fn new(config: MyProviderConfig) -> Result<Self> {
    if config.endpoint.is_empty() {
        return Err(Error::Auth("Endpoint URL cannot be empty".to_string()));
    }
    // ... rest of constructor
}
```

### 3. Security Considerations

- Never log passwords or sensitive credentials
- Use secure connections (HTTPS) for API calls
- Implement proper timeout handling
- Consider rate limiting for authentication attempts

### 4. Performance

- Use connection pooling where possible
- Implement caching for frequently accessed data
- Handle network timeouts gracefully
- Consider async/await patterns for I/O operations

### 5. Testing

Create comprehensive tests for your provider:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authentication_success() {
        let config = MyProviderConfig::default();
        let provider = MyProvider::new(config).unwrap();
        
        let request = AuthRequest {
            username: "testuser".to_string(),
            credential: "password123".to_string(),
            // ... other fields
        };
        
        let result = provider.authenticate(&request).await.unwrap();
        assert!(matches!(result, AuthResult::Success(_)));
    }
}
```

## Troubleshooting

### Common Issues

1. **Compilation Errors**: Make sure all trait methods are implemented correctly
2. **Runtime Errors**: Check network connectivity and API credentials
3. **Configuration Issues**: Validate all configuration parameters
4. **Performance Issues**: Monitor connection pools and timeouts

### Debugging

Enable debug logging to troubleshoot authentication issues:

```rust
tracing::debug!("Authenticating user: {}", username);
tracing::warn!("Authentication failed: {}", error);
```

## Advanced Features

### Custom Authentication Mechanisms

You can implement custom SASL mechanisms by extending the `SaslMechanism` trait:

```rust
pub struct CustomMechanism {
    auth_manager: Arc<AuthManager>,
}

#[async_trait]
impl SaslMechanism for CustomMechanism {
    fn name(&self) -> &str {
        "CUSTOM"
    }
    
    async fn step(&self, client: &Client, data: &str) -> Result<SaslResponse> {
        // Implement custom authentication logic
    }
}
```

### Multi-Provider Authentication

You can register multiple providers and use fallback authentication:

```rust
// Register multiple providers
auth_manager.register_provider(Arc::new(primary_provider)).await?;
auth_manager.register_provider(Arc::new(fallback_provider)).await?;

// Set primary and fallback
auth_manager.set_primary_provider("primary").await?;
auth_manager.add_fallback_provider("fallback").await?;
```

## Conclusion

The rustircd authentication system is designed to be flexible and extensible. By following this guide, you can easily integrate any authentication service with your IRC server. The modular design allows you to:

- Add new authentication providers without modifying core code
- Mix and match different authentication methods
- Implement custom authentication flows
- Scale authentication across multiple services

For more examples and detailed API documentation, see the existing authentication providers in the codebase.
