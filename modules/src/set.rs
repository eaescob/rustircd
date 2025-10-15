//! Set Module
//! 
//! Provides server configuration management for operators.
//! Based on Ratbox's m_set.c module.

use rustircd_core::{
    async_trait, Client, Error, Message, MessageType, Module,
    ModuleNumericManager, module::{ModuleResult, ModuleStatsResponse, ModuleContext},
    NumericReply, Result, User
};
use tracing::info;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Set module for server configuration management
pub struct SetModule {
    /// Current server configuration settings
    settings: RwLock<HashMap<String, SettingValue>>,
    /// Configuration metadata
    config_meta: HashMap<String, SettingMetadata>,
}

/// A configuration setting value
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Float(f64),
}

/// Metadata for a configuration setting
#[derive(Debug, Clone)]
pub struct SettingMetadata {
    pub name: String,
    pub description: String,
    pub setting_type: SettingType,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub allowed_values: Option<Vec<String>>,
    pub requires_restart: bool,
    pub operator_only: bool,
}

/// Types of configuration settings
#[derive(Debug, Clone, PartialEq)]
pub enum SettingType {
    String,
    Integer,
    Boolean,
    Float,
    Enum(Vec<String>),
}

impl SetModule {
    /// Create a new set module with default settings
    pub fn new() -> Self {
        let mut module = Self {
            settings: RwLock::new(HashMap::new()),
            config_meta: HashMap::new(),
        };
        
        module.initialize_default_settings();
        module
    }
    
    /// Initialize default server settings
    fn initialize_default_settings(&mut self) {
        // Server information settings
        self.add_setting(SettingMetadata {
            name: "server_name".to_string(),
            description: "Server name".to_string(),
            setting_type: SettingType::String,
            min_value: None,
            max_value: None,
            allowed_values: None,
            requires_restart: true,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "server_description".to_string(),
            description: "Server description".to_string(),
            setting_type: SettingType::String,
            min_value: None,
            max_value: None,
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "server_version".to_string(),
            description: "Server version string".to_string(),
            setting_type: SettingType::String,
            min_value: None,
            max_value: None,
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        // Connection settings
        self.add_setting(SettingMetadata {
            name: "max_clients".to_string(),
            description: "Maximum number of clients".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(1),
            max_value: Some(100000),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "max_channels_per_user".to_string(),
            description: "Maximum channels per user".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(1),
            max_value: Some(1000),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "max_nick_length".to_string(),
            description: "Maximum nickname length".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(1),
            max_value: Some(50),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "max_channel_length".to_string(),
            description: "Maximum channel name length".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(1),
            max_value: Some(50),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        // Timeout settings
        self.add_setting(SettingMetadata {
            name: "ping_timeout".to_string(),
            description: "PING timeout in seconds".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(30),
            max_value: Some(3600),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "connection_timeout".to_string(),
            description: "Connection timeout in seconds".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(30),
            max_value: Some(300),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        // Rate limiting settings
        self.add_setting(SettingMetadata {
            name: "max_message_length".to_string(),
            description: "Maximum message length".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(100),
            max_value: Some(8192),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "max_command_rate".to_string(),
            description: "Maximum commands per second per user".to_string(),
            setting_type: SettingType::Float,
            min_value: Some(1),
            max_value: Some(100),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        // Channel settings
        self.add_setting(SettingMetadata {
            name: "max_channel_members".to_string(),
            description: "Maximum members per channel".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(1),
            max_value: Some(10000),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "max_channel_modes".to_string(),
            description: "Maximum modes per channel".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(1),
            max_value: Some(100),
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        // Security settings
        self.add_setting(SettingMetadata {
            name: "require_ssl".to_string(),
            description: "Require SSL for all connections".to_string(),
            setting_type: SettingType::Boolean,
            min_value: None,
            max_value: None,
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        self.add_setting(SettingMetadata {
            name: "allow_insecure_connections".to_string(),
            description: "Allow non-SSL connections".to_string(),
            setting_type: SettingType::Boolean,
            min_value: None,
            max_value: None,
            allowed_values: None,
            requires_restart: false,
            operator_only: true,
        });
        
        // Logging settings
        self.add_setting(SettingMetadata {
            name: "log_level".to_string(),
            description: "Logging level".to_string(),
            setting_type: SettingType::Enum(vec!["error".to_string(), "warn".to_string(), "info".to_string(), "debug".to_string(), "trace".to_string()]),
            min_value: None,
            max_value: None,
            allowed_values: Some(vec!["error".to_string(), "warn".to_string(), "info".to_string(), "debug".to_string(), "trace".to_string()]),
            requires_restart: false,
            operator_only: true,
        });
        
        // Set default values
        self.set_default_values();
    }
    
    /// Add a setting metadata
    fn add_setting(&mut self, metadata: SettingMetadata) {
        self.config_meta.insert(metadata.name.clone(), metadata);
    }
    
    /// Set default values for all settings
    fn set_default_values(&mut self) {
        let mut settings = self.settings.try_write().unwrap();
        
        settings.insert("server_name".to_string(), SettingValue::String("rustircd.example.com".to_string()));
        settings.insert("server_description".to_string(), SettingValue::String("Rust IRC Daemon".to_string()));
        settings.insert("server_version".to_string(), SettingValue::String("rustircd-1.0.0".to_string()));
        settings.insert("max_clients".to_string(), SettingValue::Integer(1000));
        settings.insert("max_channels_per_user".to_string(), SettingValue::Integer(50));
        settings.insert("max_nick_length".to_string(), SettingValue::Integer(30));
        settings.insert("max_channel_length".to_string(), SettingValue::Integer(50));
        settings.insert("ping_timeout".to_string(), SettingValue::Integer(120));
        settings.insert("connection_timeout".to_string(), SettingValue::Integer(60));
        settings.insert("max_message_length".to_string(), SettingValue::Integer(512));
        settings.insert("max_command_rate".to_string(), SettingValue::Float(10.0));
        settings.insert("max_channel_members".to_string(), SettingValue::Integer(1000));
        settings.insert("max_channel_modes".to_string(), SettingValue::Integer(20));
        settings.insert("require_ssl".to_string(), SettingValue::Boolean(false));
        settings.insert("allow_insecure_connections".to_string(), SettingValue::Boolean(true));
        settings.insert("log_level".to_string(), SettingValue::String("info".to_string()));
    }
    
    /// Handle SET command
    async fn handle_set(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if args.is_empty() {
            // Show all settings
            self.show_all_settings(client, user).await?;
            return Ok(());
        }
        
        let setting_name = &args[0].to_lowercase();
        
        if args.len() == 1 {
            // Show specific setting
            self.show_setting(client, user, setting_name).await?;
        } else {
            // Set setting value
            let value = args[1..].join(" ");
            self.set_setting(client, user, setting_name, &value).await?;
        }
        
        Ok(())
    }
    
    /// Show all available settings
    async fn show_all_settings(&self, client: &Client, user: &User) -> Result<()> {
        client.send_numeric(NumericReply::RplSettings, &["Setting", "Value", "Type", "Description"])?;
        
        let settings = self.settings.read().await;
        let mut sorted_settings: Vec<_> = self.config_meta.iter().collect();
        sorted_settings.sort_by(|a, b| a.0.cmp(b.0));
        
        for (name, metadata) in sorted_settings {
            if metadata.operator_only && !user.is_operator() {
                continue;
            }
            
            let value = settings.get(name).map(|v| format!("{}", v)).unwrap_or_else(|| "Not set".to_string());
            let setting_type = format!("{}", metadata.setting_type);
            
            client.send_numeric(NumericReply::RplSettings, &[name, &value, &setting_type, &metadata.description])?;
        }
        
        client.send_numeric(NumericReply::RplEndOfSettings, &["End of SETTINGS"])?;
        
        Ok(())
    }
    
    /// Show a specific setting
    async fn show_setting(&self, client: &Client, user: &User, setting_name: &str) -> Result<()> {
        if let Some(metadata) = self.config_meta.get(setting_name) {
            if metadata.operator_only && !user.is_operator() {
                client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
                return Ok(());
            }
            
            let settings = self.settings.read().await;
            let value = settings.get(setting_name).map(|v| format!("{}", v)).unwrap_or_else(|| "Not set".to_string());
            
            client.send_numeric(NumericReply::RplSetting, &[setting_name, &value, &metadata.description])?;
            
            if metadata.requires_restart {
                client.send_numeric(NumericReply::RplSetting, &[setting_name, "Note: This setting requires server restart to take effect"])?;
            }
        } else {
            client.send_numeric(NumericReply::ErrNoSuchSetting, &[setting_name, "No such setting"])?;
        }
        
        Ok(())
    }
    
    /// Set a setting value
    async fn set_setting(&self, client: &Client, user: &User, setting_name: &str, value: &str) -> Result<()> {
        if let Some(metadata) = self.config_meta.get(setting_name) {
            if metadata.operator_only && !user.is_operator() {
                client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
                return Ok(());
            }
            
            // Parse and validate value
            match self.parse_setting_value(value, &metadata.setting_type) {
                Ok(parsed_value) => {
                    if self.validate_setting_value(&parsed_value, metadata)? {
                        let mut settings = self.settings.write().await;
                        settings.insert(setting_name.to_string(), parsed_value);
                        
                        client.send_numeric(NumericReply::RplSetting, &[setting_name, value, "Setting updated"])?;
                        
                        if metadata.requires_restart {
                            client.send_numeric(NumericReply::RplSetting, &[setting_name, "Note: Server restart required for this change to take effect"])?;
                        }
                        
                        info!("Setting {} changed to {} by {}", setting_name, value, user.nickname());
                    } else {
                        client.send_numeric(NumericReply::ErrInvalidValue, &[setting_name, "Invalid value"])?;
                    }
                }
                Err(e) => {
                    client.send_numeric(NumericReply::ErrInvalidValue, &[setting_name, &format!("Parse error: {}", e)])?;
                }
            }
        } else {
            client.send_numeric(NumericReply::ErrNoSuchSetting, &[setting_name, "No such setting"])?;
        }
        
        Ok(())
    }
    
    /// Parse a setting value based on its type
    fn parse_setting_value(&self, value: &str, setting_type: &SettingType) -> Result<SettingValue> {
        match setting_type {
            SettingType::String => Ok(SettingValue::String(value.to_string())),
            SettingType::Boolean => {
                match value.to_lowercase().as_str() {
                    "true" | "yes" | "on" | "1" => Ok(SettingValue::Boolean(true)),
                    "false" | "no" | "off" | "0" => Ok(SettingValue::Boolean(false)),
                    _ => Err(Error::Config("Invalid boolean value".to_string())),
                }
            }
            SettingType::Integer => {
                value.parse::<i64>()
                    .map(SettingValue::Integer)
                    .map_err(|_| Error::Config("Invalid integer value".to_string()))
            }
            SettingType::Float => {
                value.parse::<f64>()
                    .map(SettingValue::Float)
                    .map_err(|_| Error::Config("Invalid float value".to_string()))
            }
            SettingType::Enum(allowed_values) => {
                if allowed_values.contains(&value.to_string()) {
                    Ok(SettingValue::String(value.to_string()))
                } else {
                    Err(Error::Config(format!("Value must be one of: {}", allowed_values.join(", "))))
                }
            }
        }
    }
    
    /// Validate a setting value
    fn validate_setting_value(&self, value: &SettingValue, metadata: &SettingMetadata) -> Result<bool> {
        match value {
            SettingValue::Integer(int_val) => {
                if let Some(min) = metadata.min_value {
                    if *int_val < min {
                        return Err(Error::Config(format!("Value must be at least {}", min)));
                    }
                }
                if let Some(max) = metadata.max_value {
                    if *int_val > max {
                        return Err(Error::Config(format!("Value must be at most {}", max)));
                    }
                }
                Ok(true)
            }
            SettingValue::Float(float_val) => {
                if let Some(min) = metadata.min_value {
                    if *float_val < min as f64 {
                        return Err(Error::Config(format!("Value must be at least {}", min)));
                    }
                }
                if let Some(max) = metadata.max_value {
                    if *float_val > max as f64 {
                        return Err(Error::Config(format!("Value must be at most {}", max)));
                    }
                }
                Ok(true)
            }
            SettingValue::String(str_val) => {
                if let Some(allowed) = &metadata.allowed_values {
                    if !allowed.contains(str_val) {
                        return Err(Error::Config(format!("Value must be one of: {}", allowed.join(", "))));
                    }
                }
                Ok(true)
            }
            SettingValue::Boolean(_) => Ok(true),
        }
    }
    
    /// Get a setting value
    pub async fn get_setting(&self, name: &str) -> Option<SettingValue> {
        let settings = self.settings.read().await;
        settings.get(name).cloned()
    }
    
    /// Get all settings
    pub async fn get_all_settings(&self) -> HashMap<String, SettingValue> {
        let settings = self.settings.read().await;
        settings.clone()
    }
}

impl std::fmt::Display for SettingValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingValue::String(s) => write!(f, "{}", s),
            SettingValue::Integer(i) => write!(f, "{}", i),
            SettingValue::Boolean(b) => write!(f, "{}", b),
            SettingValue::Float(fl) => write!(f, "{}", fl),
        }
    }
}

impl std::fmt::Display for SettingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingType::String => write!(f, "string"),
            SettingType::Integer => write!(f, "integer"),
            SettingType::Boolean => write!(f, "boolean"),
            SettingType::Float => write!(f, "float"),
            SettingType::Enum(values) => write!(f, "enum({})", values.join(",")),
        }
    }
}

#[async_trait]
impl Module for SetModule {
    fn name(&self) -> &str {
        "set"
    }
    
    fn description(&self) -> &str {
        "Provides server configuration management for operators"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("{} module initialized", self.name());
        Ok(())
    }

    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled),
        };

        match message.command {
            MessageType::Custom(ref cmd) if cmd == "SET" => {
                self.handle_set(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_server_message(&mut self, _server: &str, _message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }

    async fn handle_user_registration(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }

    async fn handle_user_disconnection(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["message_handler".to_string()]
    }

    fn supports_capability(&self, capability: &str) -> bool {
        capability == "message_handler"
    }

    fn get_numeric_replies(&self) -> Vec<u16> {
        vec![]
    }

    fn handles_numeric_reply(&self, _numeric: u16) -> bool {
        false
    }

    async fn handle_numeric_reply(&mut self, _numeric: u16, _params: Vec<String>) -> Result<()> {
        Ok(())
    }

    async fn handle_stats_query(&mut self, _query: &str, _client_id: uuid::Uuid, _server: Option<&rustircd_core::Server>) -> Result<Vec<ModuleStatsResponse>> {
        Ok(vec![])
    }

    fn get_stats_queries(&self) -> Vec<String> {
        vec![]
    }

    fn register_numerics(&self, _manager: &mut ModuleNumericManager) -> Result<()> {
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("Set module cleaned up");
        Ok(())
    }
}

impl Default for SetModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_set_module_creation() {
        let module = SetModule::new();
        assert!(!module.config_meta.is_empty());
    }
    
    #[test]
    fn test_parse_setting_values() {
        let module = SetModule::new();
        
        // Test string parsing
        let result = module.parse_setting_value("test", &SettingType::String);
        assert!(matches!(result, Ok(SettingValue::String(s)) if s == "test"));
        
        // Test boolean parsing
        let result = module.parse_setting_value("true", &SettingType::Boolean);
        assert!(matches!(result, Ok(SettingValue::Boolean(true))));
        
        let result = module.parse_setting_value("false", &SettingType::Boolean);
        assert!(matches!(result, Ok(SettingValue::Boolean(false))));
        
        // Test integer parsing
        let result = module.parse_setting_value("42", &SettingType::Integer);
        assert!(matches!(result, Ok(SettingValue::Integer(42))));
        
        // Test float parsing
        let result = module.parse_setting_value("3.14", &SettingType::Float);
        assert!(matches!(result, Ok(SettingValue::Float(f)) if (f - 3.14).abs() < 0.001));
    }
    
    #[test]
    fn test_validate_setting_values() {
        let module = SetModule::new();
        let metadata = SettingMetadata {
            name: "test".to_string(),
            description: "Test setting".to_string(),
            setting_type: SettingType::Integer,
            min_value: Some(1),
            max_value: Some(100),
            allowed_values: None,
            requires_restart: false,
            operator_only: false,
        };
        
        // Test valid value
        let result = module.validate_setting_value(&SettingValue::Integer(50), &metadata);
        assert!(result.is_ok());
        
        // Test value too low
        let result = module.validate_setting_value(&SettingValue::Integer(0), &metadata);
        assert!(result.is_err());
        
        // Test value too high
        let result = module.validate_setting_value(&SettingValue::Integer(101), &metadata);
        assert!(result.is_err());
    }
}
