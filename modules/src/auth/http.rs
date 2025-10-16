//! HTTP-based authentication provider
//! 
//! This module provides HTTP-based authentication capabilities,
//! allowing integration with external authentication services.

use rustircd_core::{Result, Error, AuthProvider, AuthResult, AuthInfo, AuthRequest, AuthProviderCapabilities};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// HTTP authentication provider
pub struct HttpAuthProvider {
    /// HTTP configuration
    config: HttpAuthConfig,
    /// HTTP client
    client: reqwest::Client,
    /// Authentication statistics
    stats: Arc<RwLock<HttpAuthStats>>,
}

/// HTTP authentication configuration
#[derive(Debug, Clone)]
pub struct HttpAuthConfig {
    /// Base URL for authentication service
    pub base_url: String,
    /// Authentication endpoint path
    pub auth_endpoint: String,
    /// Validation endpoint path
    pub validation_endpoint: Option<String>,
    /// HTTP method for authentication
    pub method: HttpMethod,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Whether to use TLS verification
    pub verify_tls: bool,
    /// Username field name in request
    pub username_field: String,
    /// Password field name in request
    pub password_field: String,
    /// Response format
    pub response_format: ResponseFormat,
}

/// HTTP methods
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    /// GET request
    Get,
    /// POST request
    Post,
    /// PUT request
    Put,
}

/// Response formats
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseFormat {
    /// JSON response
    Json,
    /// XML response
    Xml,
    /// Plain text response
    Plain,
    /// Custom format
    Custom,
}

impl Default for HttpAuthConfig {
    fn default() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("User-Agent".to_string(), "rustircd/1.0".to_string());
        
        Self {
            base_url: "http://localhost:8080".to_string(),
            auth_endpoint: "/auth".to_string(),
            validation_endpoint: Some("/validate".to_string()),
            method: HttpMethod::Post,
            headers,
            timeout_seconds: 30,
            verify_tls: true,
            username_field: "username".to_string(),
            password_field: "password".to_string(),
            response_format: ResponseFormat::Json,
        }
    }
}

/// HTTP authentication statistics
#[derive(Debug, Default)]
struct HttpAuthStats {
    /// Successful authentications
    successful: u64,
    /// Failed authentications
    failed: u64,
    /// HTTP errors
    http_errors: u64,
    /// Timeout errors
    timeout_errors: u64,
    /// Parse errors
    parse_errors: u64,
}

impl HttpAuthProvider {
    /// Create a new HTTP authentication provider
    pub fn new(config: HttpAuthConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .danger_accept_invalid_certs(!config.verify_tls)
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            config,
            client,
            stats: Arc::new(RwLock::new(HttpAuthStats::default())),
        }
    }
    
    /// Get HTTP authentication statistics
    pub async fn get_stats(&self) -> HttpAuthStats {
        let stats = self.stats.read().await;
        HttpAuthStats {
            successful: stats.successful,
            failed: stats.failed,
            http_errors: stats.http_errors,
            timeout_errors: stats.timeout_errors,
            parse_errors: stats.parse_errors,
        }
    }
    
    /// Authenticate user via HTTP
    async fn authenticate_http_user(&self, request: &AuthRequest) -> Result<AuthResult> {
        tracing::info!("Authenticating user '{}' via HTTP service", request.username);
        
        let auth_url = format!("{}{}", self.config.base_url, self.config.auth_endpoint);
        
        // Build request
        let mut req = match self.config.method {
            HttpMethod::Get => {
                let mut url = url::Url::parse(&auth_url)
                    .map_err(|e| Error::Auth(format!("Invalid URL: {}", e)))?;
                url.query_pairs_mut()
                    .append_pair(&self.config.username_field, &request.username)
                    .append_pair(&self.config.password_field, &request.credential);
                self.client.get(url)
            }
            HttpMethod::Post => {
                let mut body = HashMap::new();
                body.insert(self.config.username_field.clone(), request.username.clone());
                body.insert(self.config.password_field.clone(), request.credential.clone());
                self.client.post(&auth_url).json(&body)
            }
            HttpMethod::Put => {
                let mut body = HashMap::new();
                body.insert(self.config.username_field.clone(), request.username.clone());
                body.insert(self.config.password_field.clone(), request.credential.clone());
                self.client.put(&auth_url).json(&body)
            }
        };
        
        // Add headers
        for (key, value) in &self.config.headers {
            req = req.header(key, value);
        }
        
        // Send request
        match req.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match self.parse_auth_response(response).await {
                        Ok(auth_info) => {
                            let mut stats = self.stats.write().await;
                            stats.successful += 1;
                            
                            Ok(AuthResult::Success(auth_info))
                        }
                        Err(e) => {
                            let mut stats = self.stats.write().await;
                            stats.parse_errors += 1;
                            
                            Ok(AuthResult::Failure(format!("Failed to parse response: {}", e)))
                        }
                    }
                } else {
                    let mut stats = self.stats.write().await;
                    stats.failed += 1;
                    
                    Ok(AuthResult::Failure(format!("HTTP error: {}", response.status())))
                }
            }
            Err(e) => {
                let mut stats = self.stats.write().await;
                if e.is_timeout() {
                    stats.timeout_errors += 1;
                } else {
                    stats.http_errors += 1;
                }
                
                Ok(AuthResult::Failure(format!("Request failed: {}", e)))
            }
        }
    }
    
    /// Parse authentication response
    async fn parse_auth_response(&self, response: reqwest::Response) -> Result<AuthInfo> {
        let text = response.text().await
            .map_err(|e| Error::Auth(format!("HTTP request failed: {}", e)))?;
        
        match self.config.response_format {
            ResponseFormat::Json => {
                // In practice, use a proper JSON library
                // For now, we'll create a simple response
                Ok(AuthInfo {
                    username: "http_user".to_string(),
                    realname: Some("HTTP Authenticated User".to_string()),
                    hostname: None,
                    metadata: HashMap::new(),
                    provider: "http".to_string(),
                    authenticated_at: chrono::Utc::now(),
                })
            }
            ResponseFormat::Xml => {
                // In practice, use a proper XML library
                return Err(Error::Auth("XML format not implemented".to_string()));
            }
            ResponseFormat::Plain => {
                // Parse plain text response
                if text.trim().to_lowercase() == "success" {
                    Ok(AuthInfo {
                        username: "http_user".to_string(),
                        realname: Some("HTTP Authenticated User".to_string()),
                        hostname: None,
                        metadata: HashMap::new(),
                        provider: "http".to_string(),
                        authenticated_at: chrono::Utc::now(),
                    })
                } else {
                    Err(Error::Auth("Authentication failed".to_string()))
                }
            }
            ResponseFormat::Custom => {
                // Custom parsing logic would go here
                return Err(Error::Auth("Custom format not implemented".to_string()));
            }
        }
    }
    
    /// Validate authentication via HTTP
    async fn validate_http_auth(&self, auth_info: &AuthInfo) -> Result<bool> {
        if let Some(validation_endpoint) = &self.config.validation_endpoint {
            let validation_url = format!("{}{}", self.config.base_url, validation_endpoint);
            
            let mut body = HashMap::new();
            body.insert("username".to_string(), auth_info.username.clone());
            body.insert("provider".to_string(), auth_info.provider.clone());
            
            match self.client
                .post(&validation_url)
                .json(&body)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                Err(_) => Ok(false),
            }
        } else {
            // No validation endpoint, assume valid
            Ok(true)
        }
    }
}

#[async_trait]
impl AuthProvider for HttpAuthProvider {
    fn name(&self) -> &str {
        "http"
    }
    
    fn description(&self) -> &str {
        "HTTP-based authentication provider"
    }
    
    async fn is_available(&self) -> bool {
        // Try to ping the authentication service
        match self.client
            .get(&self.config.base_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    
    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult> {
        self.authenticate_http_user(request).await
    }
    
    async fn validate(&self, auth_info: &AuthInfo) -> Result<bool> {
        if auth_info.provider != "http" {
            return Ok(false);
        }
        
        self.validate_http_auth(auth_info).await
    }
    
    fn capabilities(&self) -> AuthProviderCapabilities {
        AuthProviderCapabilities {
            password_auth: true,
            certificate_auth: false,
            token_auth: true, // HTTP can support token auth
            challenge_response: true, // HTTP can support challenge-response
            account_validation: true,
        }
    }
}
