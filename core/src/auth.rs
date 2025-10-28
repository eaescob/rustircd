//! Authentication system for the IRC daemon
//! 
//! This module provides a flexible authentication framework that supports
//! multiple authentication providers including services integration and
//! external authentication systems.

use crate::{Result, Error};
use crate::audit::{AuditEvent, AuditEventType, AuditLogger};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Authentication result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    /// Authentication successful
    Success(AuthInfo),
    /// Authentication failed
    Failure(String),
    /// Authentication requires additional steps
    Challenge(String),
    /// Authentication in progress
    InProgress,
}

/// Authentication information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthInfo {
    /// Authenticated username/account name
    pub username: String,
    /// User's real name
    pub realname: Option<String>,
    /// User's hostname
    pub hostname: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Authentication provider that validated this user
    pub provider: String,
    /// Authentication timestamp
    pub authenticated_at: chrono::DateTime<chrono::Utc>,
}

/// Authentication request
#[derive(Debug, Clone)]
pub struct AuthRequest {
    /// Username to authenticate
    pub username: String,
    /// Password or credential
    pub credential: String,
    /// Authorization ID (optional, for SASL)
    pub authzid: Option<String>,
    /// Client information
    pub client_info: ClientInfo,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Client information for authentication
#[derive(Debug, Clone)]
pub struct ClientInfo {
    /// Client ID
    pub id: Uuid,
    /// Client's IP address
    pub ip: String,
    /// Client's hostname
    pub hostname: Option<String>,
    /// Whether connection is secure (TLS)
    pub secure: bool,
}

/// Authentication provider trait
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;
    
    /// Provider description
    fn description(&self) -> &str;
    
    /// Check if provider is available/enabled
    async fn is_available(&self) -> bool;
    
    /// Authenticate a user
    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult>;
    
    /// Validate an existing authentication
    async fn validate(&self, auth_info: &AuthInfo) -> Result<bool>;
    
    /// Get provider capabilities
    fn capabilities(&self) -> AuthProviderCapabilities;
}

/// Authentication provider capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthProviderCapabilities {
    /// Whether provider supports password authentication
    pub password_auth: bool,
    /// Whether provider supports certificate authentication
    pub certificate_auth: bool,
    /// Whether provider supports token authentication
    pub token_auth: bool,
    /// Whether provider supports challenge-response authentication
    pub challenge_response: bool,
    /// Whether provider supports account validation
    pub account_validation: bool,
}

impl Default for AuthProviderCapabilities {
    fn default() -> Self {
        Self {
            password_auth: true,
            certificate_auth: false,
            token_auth: false,
            challenge_response: false,
            account_validation: false,
        }
    }
}

/// Authentication manager
pub struct AuthManager {
    /// Registered authentication providers
    providers: Arc<RwLock<HashMap<String, Arc<dyn AuthProvider>>>>,
    /// Primary provider for authentication
    primary_provider: Arc<RwLock<Option<String>>>,
    /// Fallback providers (in order of preference)
    fallback_providers: Arc<RwLock<Vec<String>>>,
    /// Authentication cache
    auth_cache: Arc<RwLock<HashMap<Uuid, (AuthInfo, chrono::DateTime<chrono::Utc>)>>>,
    /// Cache TTL in seconds
    cache_ttl: u64,
    /// Audit logger for security events
    audit_logger: AuditLogger,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(cache_ttl: u64) -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            primary_provider: Arc::new(RwLock::new(None)),
            fallback_providers: Arc::new(RwLock::new(Vec::new())),
            auth_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            audit_logger: AuditLogger::default(),
        }
    }

    /// Create a new authentication manager with audit logger
    pub fn with_audit_logger(cache_ttl: u64, audit_logger: AuditLogger) -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            primary_provider: Arc::new(RwLock::new(None)),
            fallback_providers: Arc::new(RwLock::new(Vec::new())),
            auth_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            audit_logger,
        }
    }
    
    /// Register an authentication provider
    pub async fn register_provider(&self, provider: Arc<dyn AuthProvider>) -> Result<()> {
        let name = provider.name().to_string();
        let mut providers = self.providers.write().await;
        
        if providers.contains_key(&name) {
            return Err(Error::Auth(format!("Provider '{}' is already registered", name)));
        }
        
        providers.insert(name.clone(), provider);
        
        // Set as primary if it's the first provider
        {
            let primary = self.primary_provider.read().await;
            if primary.is_none() {
                drop(primary);
                let mut primary = self.primary_provider.write().await;
                *primary = Some(name.clone());
            }
        }
        
        tracing::info!("Registered authentication provider: {}", name);
        Ok(())
    }
    
    /// Unregister an authentication provider
    pub async fn unregister_provider(&self, name: &str) -> Result<()> {
        let mut providers = self.providers.write().await;
        
        if providers.remove(name).is_none() {
            return Err(Error::Auth(format!("Provider '{}' not found", name)));
        }
        
        // Update primary provider if it was removed
        {
            let mut primary = self.primary_provider.write().await;
            if primary.as_ref() == Some(&name.to_string()) {
                *primary = providers.keys().next().map(|k| k.clone());
            }
        }
        
        // Remove from fallback providers
        {
            let mut fallbacks = self.fallback_providers.write().await;
            fallbacks.retain(|p| p != name);
        }
        
        tracing::info!("Unregistered authentication provider: {}", name);
        Ok(())
    }
    
    /// Set the primary authentication provider
    pub async fn set_primary_provider(&self, name: &str) -> Result<()> {
        let providers = self.providers.read().await;
        
        if !providers.contains_key(name) {
            return Err(Error::Auth(format!("Provider '{}' not found", name)));
        }
        
        // Move current primary to fallback if it exists
        {
            let primary = self.primary_provider.read().await;
            let mut fallbacks = self.fallback_providers.write().await;
            
            if let Some(current_primary) = primary.as_ref() {
                if current_primary != name && !fallbacks.contains(current_primary) {
                    fallbacks.push(current_primary.clone());
                }
            }
            
            // Remove from fallback if it was there
            fallbacks.retain(|p| p != name);
        }
        
        // Set as primary
        {
            let mut primary = self.primary_provider.write().await;
            *primary = Some(name.to_string());
        }
        
        tracing::info!("Set primary authentication provider: {}", name);
        Ok(())
    }
    
    /// Add a fallback provider
    pub async fn add_fallback_provider(&self, name: &str) -> Result<()> {
        let providers = self.providers.read().await;
        
        if !providers.contains_key(name) {
            return Err(Error::Auth(format!("Provider '{}' not found", name)));
        }
        
        let name_string = name.to_string();
        {
            let primary = self.primary_provider.read().await;
            let mut fallbacks = self.fallback_providers.write().await;
            
            if !fallbacks.contains(&name_string) && primary.as_ref() != Some(&name_string) {
                fallbacks.push(name_string);
            }
        }
        
        tracing::info!("Added fallback authentication provider: {}", name);
        Ok(())
    }
    
    /// Authenticate a user using available providers
    pub async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult> {
        // Check cache first
        if let Some(cached_auth) = self.get_cached_auth(request.client_info.id).await {
            // Validate cached authentication
            if self.validate_cached_auth(&cached_auth).await {
                return Ok(AuthResult::Success(cached_auth));
            } else {
                // Remove invalid cached auth
                self.remove_cached_auth(request.client_info.id).await;
            }
        }
        
        // Try primary provider first
        let primary_name = {
            let primary = self.primary_provider.read().await;
            primary.clone()
        };
        if let Some(primary_name) = primary_name {
            if let Some(provider) = self.get_provider(&primary_name).await {
                if provider.is_available().await {
                    match provider.authenticate(request).await {
                        Ok(AuthResult::Success(auth_info)) => {
                            // Audit log successful authentication
                            let audit_event = AuditEvent::new(AuditEventType::AuthSuccess)
                                .with_user(&request.username)
                                .with_user_id(request.client_info.id)
                                .with_ip(&request.client_info.ip)
                                .with_hostname(request.client_info.hostname.as_deref().unwrap_or("unknown"))
                                .with_method(&auth_info.provider)
                                .with_secure(request.client_info.secure)
                                .with_metadata("provider", auth_info.provider.clone());
                            self.audit_logger.log(&audit_event);

                            // Cache successful authentication
                            self.cache_auth(request.client_info.id, &auth_info).await;
                            return Ok(AuthResult::Success(auth_info));
                        }
                        Ok(AuthResult::Challenge(ref challenge)) => {
                            // Audit log challenge event
                            let audit_event = AuditEvent::new(AuditEventType::AuthChallenge)
                                .with_user(&request.username)
                                .with_user_id(request.client_info.id)
                                .with_ip(&request.client_info.ip)
                                .with_method(&primary_name)
                                .with_reason(challenge);
                            self.audit_logger.log(&audit_event);

                            return Ok(AuthResult::Challenge(challenge.clone()));
                        }
                        Ok(result) => {
                            // In progress, return as-is
                            return Ok(result);
                        }
                        Err(e) => {
                            tracing::warn!("Primary auth provider '{}' failed: {}", primary_name, e);

                            // Audit log authentication failure
                            let audit_event = AuditEvent::new(AuditEventType::AuthFailure)
                                .with_user(&request.username)
                                .with_user_id(request.client_info.id)
                                .with_ip(&request.client_info.ip)
                                .with_hostname(request.client_info.hostname.as_deref().unwrap_or("unknown"))
                                .with_method(&primary_name)
                                .with_secure(request.client_info.secure)
                                .with_error(format!("{}", e));
                            self.audit_logger.log(&audit_event);
                        }
                    }
                }
            }
        }
        
        // Try fallback providers
        let fallbacks = self.fallback_providers.read().await;
        for fallback in fallbacks.iter() {
            if let Some(provider) = self.get_provider(fallback).await {
                if provider.is_available().await {
                    match provider.authenticate(request).await {
                        Ok(AuthResult::Success(auth_info)) => {
                            // Audit log successful authentication
                            let audit_event = AuditEvent::new(AuditEventType::AuthSuccess)
                                .with_user(&request.username)
                                .with_user_id(request.client_info.id)
                                .with_ip(&request.client_info.ip)
                                .with_hostname(request.client_info.hostname.as_deref().unwrap_or("unknown"))
                                .with_method(&auth_info.provider)
                                .with_secure(request.client_info.secure)
                                .with_metadata("provider", auth_info.provider.clone())
                                .with_metadata("fallback", "true");
                            self.audit_logger.log(&audit_event);

                            // Cache successful authentication
                            self.cache_auth(request.client_info.id, &auth_info).await;
                            return Ok(AuthResult::Success(auth_info));
                        }
                        Ok(result) => {
                            // Challenge or in progress, return as-is
                            return Ok(result);
                        }
                        Err(e) => {
                            tracing::warn!("Fallback auth provider '{}' failed: {}", fallback, e);

                            // Audit log authentication failure
                            let audit_event = AuditEvent::new(AuditEventType::AuthFailure)
                                .with_user(&request.username)
                                .with_user_id(request.client_info.id)
                                .with_ip(&request.client_info.ip)
                                .with_hostname(request.client_info.hostname.as_deref().unwrap_or("unknown"))
                                .with_method(fallback)
                                .with_secure(request.client_info.secure)
                                .with_error(format!("{}", e))
                                .with_metadata("fallback", "true");
                            self.audit_logger.log(&audit_event);
                        }
                    }
                }
            }
        }

        // All providers failed - audit log final failure
        let audit_event = AuditEvent::new(AuditEventType::AuthFailure)
            .with_user(&request.username)
            .with_user_id(request.client_info.id)
            .with_ip(&request.client_info.ip)
            .with_hostname(request.client_info.hostname.as_deref().unwrap_or("unknown"))
            .with_method("all_providers")
            .with_secure(request.client_info.secure)
            .with_error("Authentication failed with all providers");
        self.audit_logger.log(&audit_event);

        Ok(AuthResult::Failure("Authentication failed with all providers".to_string()))
    }
    
    /// Validate cached authentication
    async fn validate_cached_auth(&self, auth_info: &AuthInfo) -> bool {
        if let Some(provider) = self.get_provider(&auth_info.provider).await {
            if provider.is_available().await {
                if let Ok(valid) = provider.validate(auth_info).await {
                    return valid;
                }
            }
        }
        false
    }
    
    /// Get authentication provider
    async fn get_provider(&self, name: &str) -> Option<Arc<dyn AuthProvider>> {
        let providers = self.providers.read().await;
        providers.get(name).cloned()
    }
    
    /// Cache authentication information
    async fn cache_auth(&self, client_id: Uuid, auth_info: &AuthInfo) {
        let mut cache = self.auth_cache.write().await;
        cache.insert(client_id, (auth_info.clone(), chrono::Utc::now()));
    }
    
    /// Get cached authentication
    async fn get_cached_auth(&self, client_id: Uuid) -> Option<AuthInfo> {
        let cache = self.auth_cache.read().await;
        cache.get(&client_id).and_then(|(auth_info, timestamp)| {
            let elapsed = chrono::Utc::now().signed_duration_since(*timestamp);
            if elapsed.num_seconds() < self.cache_ttl as i64 {
                Some(auth_info.clone())
            } else {
                None
            }
        })
    }
    
    /// Remove cached authentication
    async fn remove_cached_auth(&self, client_id: Uuid) {
        let mut cache = self.auth_cache.write().await;
        cache.remove(&client_id);
    }
    
    /// Get all registered providers
    pub async fn get_providers(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Check if any authentication providers are registered
    pub async fn has_providers(&self) -> bool {
        let providers = self.providers.read().await;
        !providers.is_empty()
    }

    /// Check if any authentication providers are available (registered and enabled)
    pub async fn has_available_providers(&self) -> bool {
        let providers = self.providers.read().await;
        for provider in providers.values() {
            if provider.is_available().await {
                return true;
            }
        }
        false
    }

    /// Get provider capabilities
    pub async fn get_provider_capabilities(&self, name: &str) -> Option<AuthProviderCapabilities> {
        if let Some(provider) = self.get_provider(name).await {
            Some(provider.capabilities())
        } else {
            None
        }
    }
    
    /// Clean up expired cache entries
    pub async fn cleanup_cache(&self) -> Result<()> {
        let now = chrono::Utc::now();
        let mut cache = self.auth_cache.write().await;
        
        cache.retain(|_, (_, timestamp)| {
            let elapsed = now.signed_duration_since(*timestamp);
            elapsed.num_seconds() < self.cache_ttl as i64
        });
        
        Ok(())
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new(3600) // 1 hour cache TTL
    }
}
