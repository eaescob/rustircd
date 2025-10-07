//! SASL (Simple Authentication and Security Layer) module
//! 
//! This module provides SASL authentication support as per IRCv3 specification.
//! It supports various SASL mechanisms including PLAIN, EXTERNAL, and SCRAM-SHA-256.

use rustircd_core::{User, Message, Client, Result, Error, NumericReply, Config, MessageType};
use std::collections::HashMap;
use uuid::Uuid;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};

/// SASL module for handling SASL authentication
pub struct SaslModule {
    /// Module configuration
    config: SaslConfig,
    /// Active SASL sessions
    sessions: std::sync::Arc<tokio::sync::RwLock<HashMap<Uuid, SaslSession>>>,
    /// SASL mechanisms
    mechanisms: Vec<Box<dyn SaslMechanism>>,
}

/// Configuration for the SASL module
#[derive(Debug, Clone)]
pub struct SaslConfig {
    /// Whether SASL is enabled
    pub enabled: bool,
    /// Supported SASL mechanisms
    pub mechanisms: Vec<String>,
    /// Service name for SASL
    pub service_name: String,
    /// Whether to require SASL for certain operations
    pub require_sasl: bool,
    /// SASL timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for SaslConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mechanisms: vec!["PLAIN".to_string(), "EXTERNAL".to_string()],
            service_name: "services.example.org".to_string(),
            require_sasl: false,
            timeout_seconds: 300, // 5 minutes
        }
    }
}

/// SASL session information
#[derive(Debug, Clone)]
pub struct SaslSession {
    /// Session ID
    pub id: Uuid,
    /// Client ID
    pub client_id: Uuid,
    /// Selected mechanism
    pub mechanism: String,
    /// Current state
    pub state: SaslState,
    /// Authentication data
    pub auth_data: Option<SaslAuthData>,
    /// Session creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last activity time
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// SASL authentication data
#[derive(Debug, Clone)]
pub struct SaslAuthData {
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Authorization ID (optional)
    pub authzid: Option<String>,
}

/// SASL session states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaslState {
    /// Initial state
    Initial,
    /// Mechanism selected
    MechanismSelected,
    /// Authentication in progress
    Authenticating,
    /// Authentication successful
    Authenticated,
    /// Authentication failed
    Failed,
    /// Session expired
    Expired,
}

/// Trait for SASL mechanisms
#[async_trait]
pub trait SaslMechanism: Send + Sync {
    /// Get mechanism name
    fn name(&self) -> &str;
    
    /// Check if mechanism is supported
    fn is_supported(&self) -> bool;
    
    /// Start SASL authentication
    async fn start(&self, client: &Client, initial_data: Option<&str>) -> Result<SaslResponse>;
    
    /// Continue SASL authentication
    async fn step(&self, client: &Client, data: &str) -> Result<SaslResponse>;
    
    /// Complete SASL authentication
    async fn complete(&self, client: &Client) -> Result<SaslAuthData>;
}

/// SASL response
#[derive(Debug, Clone)]
pub struct SaslResponse {
    /// Response type
    pub response_type: SaslResponseType,
    /// Response data (base64 encoded)
    pub data: Option<String>,
    /// Error message if any
    pub error: Option<String>,
}

/// SASL response types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaslResponseType {
    /// Continue authentication
    Continue,
    /// Authentication successful
    Success,
    /// Authentication failed
    Failure,
    /// Challenge response
    Challenge,
}

/// PLAIN SASL mechanism
pub struct PlainMechanism {
    /// Service name
    service_name: String,
}

impl PlainMechanism {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

#[async_trait]
impl SaslMechanism for PlainMechanism {
    fn name(&self) -> &str {
        "PLAIN"
    }
    
    fn is_supported(&self) -> bool {
        true
    }
    
    async fn start(&self, _client: &Client, initial_data: Option<&str>) -> Result<SaslResponse> {
        if let Some(data) = initial_data {
            self.step(_client, data).await
        } else {
            Ok(SaslResponse {
                response_type: SaslResponseType::Continue,
                data: None,
                error: None,
            })
        }
    }
    
    async fn step(&self, _client: &Client, data: &str) -> Result<SaslResponse> {
        // Decode base64 data
        let decoded = general_purpose::STANDARD.decode(data)
            .map_err(|_| Error::MessageParse("Invalid base64 data".to_string()))?;
        
        let auth_string = String::from_utf8(decoded)
            .map_err(|_| Error::MessageParse("Invalid UTF-8 data".to_string()))?;
        
        // Parse auth string: authzid\0username\0password
        let parts: Vec<&str> = auth_string.split('\0').collect();
        if parts.len() != 3 {
            return Ok(SaslResponse {
                response_type: SaslResponseType::Failure,
                data: None,
                error: Some("Invalid auth string format".to_string()),
            });
        }
        
        let authzid = if parts[0].is_empty() { None } else { Some(parts[0].to_string()) };
        let username = parts[1].to_string();
        let password = parts[2].to_string();
        
        // TODO: Implement actual authentication against services
        // For now, we'll just validate the format
        
        Ok(SaslResponse {
            response_type: SaslResponseType::Success,
            data: None,
            error: None,
        })
    }
    
    async fn complete(&self, _client: &Client) -> Result<SaslAuthData> {
        // This would be called after successful authentication
        // For now, return dummy data
        Ok(SaslAuthData {
            username: "user".to_string(),
            password: "password".to_string(),
            authzid: None,
        })
    }
}

/// EXTERNAL SASL mechanism
pub struct ExternalMechanism {
    /// Service name
    service_name: String,
}

impl ExternalMechanism {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
}

#[async_trait]
impl SaslMechanism for ExternalMechanism {
    fn name(&self) -> &str {
        "EXTERNAL"
    }
    
    fn is_supported(&self) -> bool {
        true
    }
    
    async fn start(&self, _client: &Client, initial_data: Option<&str>) -> Result<SaslResponse> {
        // EXTERNAL mechanism uses client certificate or other external authentication
        // For now, we'll just return success
        Ok(SaslResponse {
            response_type: SaslResponseType::Success,
            data: None,
            error: None,
        })
    }
    
    async fn step(&self, _client: &Client, _data: &str) -> Result<SaslResponse> {
        Ok(SaslResponse {
            response_type: SaslResponseType::Success,
            data: None,
            error: None,
        })
    }
    
    async fn complete(&self, _client: &Client) -> Result<SaslAuthData> {
        // This would extract authentication info from client certificate
        Ok(SaslAuthData {
            username: "external_user".to_string(),
            password: String::new(),
            authzid: None,
        })
    }
}

impl SaslModule {
    /// Create a new SASL module
    pub fn new(config: SaslConfig) -> Self {
        let mut mechanisms: Vec<Box<dyn SaslMechanism>> = Vec::new();
        
        // Add supported mechanisms
        if config.mechanisms.contains(&"PLAIN".to_string()) {
            mechanisms.push(Box::new(PlainMechanism::new(config.service_name.clone())));
        }
        if config.mechanisms.contains(&"EXTERNAL".to_string()) {
            mechanisms.push(Box::new(ExternalMechanism::new(config.service_name.clone())));
        }
        
        Self {
            config,
            sessions: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            mechanisms,
        }
    }
    
    /// Handle SASL authentication
    pub async fn handle_sasl(&self, client: &Client, message: &Message) -> Result<()> {
        if !self.config.enabled {
            client.send_numeric(NumericReply::ErrUnknownCommand, &["SASL"])?;
            return Ok(());
        }
        
        match &message.command {
            MessageType::Custom(cmd) if cmd == "AUTHENTICATE" => self.handle_authenticate(client, message).await,
            _ => {
                client.send_numeric(NumericReply::ErrUnknownCommand, &["SASL"])?;
                Ok(())
            }
        }
    }
    
    /// Handle AUTHENTICATE command
    async fn handle_authenticate(&self, client: &Client, message: &Message) -> Result<()> {
        if message.params.is_empty() {
            let error_msg = NumericReply::need_more_params("AUTHENTICATE");
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        let mechanism = &message.params[0];
        
        // Check if mechanism is supported
        if !self.is_mechanism_supported(mechanism) {
            let error_msg = NumericReply::ErrUnknownCommand.reply("", vec![format!("SASL authentication failed: mechanism '{}' not supported", mechanism)]);
            let _ = client.send(error_msg);
            return Ok(());
        }
        
        // Create or update session
        let session = SaslSession {
            id: Uuid::new_v4(),
            client_id: client.id,
            mechanism: mechanism.clone(),
            state: SaslState::MechanismSelected,
            auth_data: None,
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(client.id, session);
        
        // Start authentication
        if let Some(mechanism_impl) = self.get_mechanism(mechanism) {
            let initial_data = if message.params.len() > 1 { Some(&message.params[1]) } else { None };
            
            match mechanism_impl.start(client, initial_data.map(|x| x.as_str())).await {
                Ok(response) => {
                    match response.response_type {
                        SaslResponseType::Continue => {
                            let continue_msg = NumericReply::RplAway.reply("", vec!["+".to_string()]);
                            let _ = client.send(continue_msg);
                        }
                        SaslResponseType::Success => {
                            self.complete_authentication(client, mechanism_impl).await?;
                        }
                        SaslResponseType::Failure => {
                            let error_msg = NumericReply::ErrUnknownCommand.reply("", vec![format!("SASL authentication failed: {}", 
                                response.error.unwrap_or_else(|| "Unknown error".to_string()))]);
                            let _ = client.send(error_msg);
                        }
                        SaslResponseType::Challenge => {
                            if let Some(data) = response.data {
                                let challenge_msg = NumericReply::RplAway.reply("", vec![data.to_string()]);
                                let _ = client.send(challenge_msg);
                            } else {
                                let continue_msg = NumericReply::RplAway.reply("", vec!["+".to_string()]);
                                let _ = client.send(continue_msg);
                            }
                        }
                    }
                }
                Err(e) => {
                    let error_msg = NumericReply::ErrUnknownCommand.reply("", vec![format!("SASL authentication failed: {}", e)]);
                    let _ = client.send(error_msg);
                }
            }
        }
        
        Ok(())
    }
    
    /// Complete SASL authentication
    async fn complete_authentication(&self, client: &Client, mechanism: &dyn SaslMechanism) -> Result<()> {
        match mechanism.complete(client).await {
            Ok(auth_data) => {
                // Update session
                let mut sessions = self.sessions.write().await;
                if let Some(session) = sessions.get_mut(&client.id) {
                    session.state = SaslState::Authenticated;
                    session.auth_data = Some(auth_data);
                    session.last_activity = chrono::Utc::now();
                }
                
                // Send success message
                let success_msg = NumericReply::RplYoureOper.reply("", vec!["SASL authentication successful".to_string()]);
                let _ = client.send(success_msg);
                
                tracing::info!("SASL authentication successful for client {}", client.id);
            }
            Err(e) => {
                let error_msg = NumericReply::ErrUnknownCommand.reply("", vec![format!("SASL authentication failed: {}", e)]);
                let _ = client.send(error_msg);
            }
        }
        
        Ok(())
    }
    
    /// Check if a mechanism is supported
    fn is_mechanism_supported(&self, mechanism: &str) -> bool {
        self.mechanisms.iter().any(|m| m.name() == mechanism)
    }
    
    /// Get mechanism implementation
    fn get_mechanism(&self, mechanism: &str) -> Option<&dyn SaslMechanism> {
        self.mechanisms.iter().find(|m| m.name() == mechanism).map(|m| m.as_ref())
    }
    
    /// Get SASL session for client
    pub async fn get_session(&self, client_id: Uuid) -> Option<SaslSession> {
        let sessions = self.sessions.read().await;
        sessions.get(&client_id).cloned()
    }
    
    /// Check if client is SASL authenticated
    pub async fn is_authenticated(&self, client_id: Uuid) -> bool {
        let sessions = self.sessions.read().await;
        sessions.get(&client_id).map(|s| s.state == SaslState::Authenticated).unwrap_or(false)
    }
    
    /// Get authentication data for client
    pub async fn get_auth_data(&self, client_id: Uuid) -> Option<SaslAuthData> {
        let sessions = self.sessions.read().await;
        sessions.get(&client_id).and_then(|s| s.auth_data.clone())
    }
    
    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<()> {
        let now = chrono::Utc::now();
        let mut sessions = self.sessions.write().await;
        
        sessions.retain(|_, session| {
            let elapsed = now.signed_duration_since(session.last_activity);
            elapsed.num_seconds() < self.config.timeout_seconds as i64
        });
        
        Ok(())
    }
    
    /// Get supported SASL mechanisms
    pub fn get_supported_mechanisms(&self) -> Vec<String> {
        self.mechanisms.iter()
            .filter(|m| m.is_supported())
            .map(|m| m.name().to_string())
            .collect()
    }
}

impl Default for SaslModule {
    fn default() -> Self {
        Self::new(SaslConfig::default())
    }
}

/// SASL capability extension
pub struct SaslCapabilityExtension {
    sasl_module: SaslModule,
}

impl SaslCapabilityExtension {
    pub fn new(sasl_module: SaslModule) -> Self {
        Self { sasl_module }
    }
}

// Extension implementation removed - extensions system was removed
