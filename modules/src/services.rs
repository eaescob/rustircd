//! Services Module
//! 
//! Provides service registration and management system.
//! Based on Ratbox's m_services.c module.

use rustircd_core::{
    async_trait, Client, Error, Message, MessageType, Module, ModuleManager,
    ModuleNumericManager, module::{ModuleResult, ModuleStatsResponse},
    NumericReply, Result, User
};
use tracing::{debug, info, warn};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Services module for service registration and management
pub struct ServicesModule {
    /// Registered services
    services: RwLock<HashMap<String, Service>>,
    /// Service configuration
    config: ServiceConfig,
    /// Service statistics
    stats: RwLock<ServiceStatistics>,
}

/// A registered service
#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub description: String,
    pub version: String,
    pub server: String,
    pub registered_at: u64,
    pub last_seen: u64,
    pub is_active: bool,
    pub service_type: ServiceType,
    pub capabilities: Vec<String>,
    pub contact: Option<String>,
    pub location: Option<String>,
}

/// Types of services
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceType {
    NickServ,
    ChanServ,
    MemoServ,
    OperServ,
    BotServ,
    Custom(String),
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub max_services: usize,
    pub service_timeout: u64, // in seconds
    pub require_authentication: bool,
    pub allow_custom_services: bool,
    pub service_prefix: String,
    pub auto_cleanup: bool,
}

/// Service statistics
#[derive(Debug, Clone, Default)]
pub struct ServiceStatistics {
    pub total_services: usize,
    pub active_services: usize,
    pub inactive_services: usize,
    pub total_registrations: usize,
    pub total_deregistrations: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            max_services: 100,
            service_timeout: 3600, // 1 hour
            require_authentication: true,
            allow_custom_services: true,
            service_prefix: "Service".to_string(),
            auto_cleanup: true,
        }
    }
}

impl ServicesModule {
    /// Create a new services module
    pub fn new() -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            config: ServiceConfig::default(),
            stats: RwLock::new(ServiceStatistics::default()),
        }
    }
    
    /// Create a new services module with custom configuration
    pub fn with_config(config: ServiceConfig) -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            config,
            stats: RwLock::new(ServiceStatistics::default()),
        }
    }
    
    /// Handle SERVICES command
    async fn handle_services(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if args.is_empty() {
            self.list_services(client, user).await?;
            return Ok(());
        }
        
        let subcommand = &args[0].to_uppercase();
        
        match subcommand.as_str() {
            "LIST" => {
                self.list_services(client, user).await?;
            }
            "INFO" => {
                if args.len() < 2 {
                    client.send_numeric(NumericReply::ErrNeedMoreParams, &["SERVICES INFO", "Not enough parameters"])?;
                    return Ok(());
                }
                self.show_service_info(client, user, &args[1]).await?;
            }
            "STATS" => {
                self.show_service_stats(client, user).await?;
            }
            "HELP" => {
                self.show_services_help(client, user).await?;
            }
            _ => {
                client.send_numeric(NumericReply::ErrUnknownCommand, &[subcommand, "Unknown SERVICES command"])?;
            }
        }
        
        Ok(())
    }
    
    /// Handle service registration
    async fn handle_service_registration(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.len() < 3 {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["SERVICE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let service_name = &args[0];
        let service_type = &args[1];
        let description = &args[2];
        let version = if args.len() > 3 { &args[3] } else { "1.0.0" };
        let server = if args.len() > 4 { &args[4] } else { "localhost" };
        
        // Validate service name
        if !self.is_valid_service_name(service_name) {
            client.send_numeric(NumericReply::ErrInvalidName, &[service_name, "Invalid service name"])?;
            return Ok(());
        }
        
        // Check if service already exists
        {
            let services = self.services.read().await;
            if services.contains_key(service_name) {
                client.send_numeric(NumericReply::ErrAlreadyRegistered, &[service_name, "Service already registered"])?;
                return Ok(());
            }
        }
        
        // Check service limit
        {
            let services = self.services.read().await;
            if services.len() >= self.config.max_services {
                client.send_numeric(NumericReply::ErrTooManyServices, &[&format!("Maximum {} services allowed", self.config.max_services)])?;
                return Ok(());
            }
        }
        
        // Parse service type
        let service_type = self.parse_service_type(service_type);
        let service_type_display = format!("{:?}", service_type);

        // Create service
        let service = Service {
            name: service_name.to_string(),
            description: description.to_string(),
            version: version.to_string(),
            server: server.to_string(),
            registered_at: self.get_current_time(),
            last_seen: self.get_current_time(),
            is_active: true,
            service_type,
            capabilities: Vec::new(),
            contact: None,
            location: None,
        };

        // Register service
        {
            let mut services = self.services.write().await;
            services.insert(service_name.to_string(), service);
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_services += 1;
            stats.active_services += 1;
            stats.total_registrations += 1;
        }

        client.send_numeric(NumericReply::RplService, &[service_name, "Service registered successfully"])?;

        info!("Service registered: {} ({}) by {}", service_name, service_type_display, user.nickname());
        
        Ok(())
    }
    
    /// Handle service deregistration
    async fn handle_service_deregistration(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        if !user.is_operator() {
            client.send_numeric(NumericReply::ErrNoPrivileges, &["Permission denied"])?;
            return Ok(());
        }
        
        if args.is_empty() {
            client.send_numeric(NumericReply::ErrNeedMoreParams, &["UNSERVICE", "Not enough parameters"])?;
            return Ok(());
        }
        
        let service_name = &args[0];
        
        // Remove service
        let removed = {
            let mut services = self.services.write().await;
            services.remove(service_name).is_some()
        };
        
        if removed {
            // Update statistics
            {
                let mut stats = self.stats.write().await;
                stats.total_services -= 1;
                stats.active_services -= 1;
                stats.total_deregistrations += 1;
            }
            
            client.send_numeric(NumericReply::RplService, &[service_name, "Service deregistered successfully"])?;
            info!("Service deregistered: {} by {}", service_name, user.nickname());
        } else {
            client.send_numeric(NumericReply::ErrNoSuchService, &[service_name, "No such service"])?;
        }
        
        Ok(())
    }
    
    /// List all services
    async fn list_services(&self, client: &Client, user: &User) -> Result<()> {
        let services = self.services.read().await;
        
        if services.is_empty() {
            client.send_numeric(NumericReply::RplService, &["*", "No services registered"])?;
            return Ok(());
        }
        
        client.send_numeric(NumericReply::RplService, &["Name", "Type", "Version", "Server", "Status"])?;
        
        for service in services.values() {
            let status = if service.is_active { "Active" } else { "Inactive" };
            let service_type = format!("{}", service.service_type);
            
            client.send_numeric(NumericReply::RplService, &[
                &service.name,
                &service_type,
                &service.version,
                &service.server,
                status
            ])?;
        }
        
        client.send_numeric(NumericReply::RplEndOfServices, &["End of services list"])?;
        
        Ok(())
    }
    
    /// Show service information
    async fn show_service_info(&self, client: &Client, user: &User, service_name: &str) -> Result<()> {
        let services = self.services.read().await;
        
        if let Some(service) = services.get(service_name) {
            client.send_numeric(NumericReply::RplService, &[&service.name, "Service Information"])?;
            client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Type: {}", service.service_type)])?;
            client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Description: {}", service.description)])?;
            client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Version: {}", service.version)])?;
            client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Server: {}", service.server)])?;
            client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Status: {}", if service.is_active { "Active" } else { "Inactive" })])?;
            client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Registered: {}", self.format_time(service.registered_at))])?;
            client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Last seen: {}", self.format_time(service.last_seen))])?;
            
            if let Some(contact) = &service.contact {
                client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Contact: {}", contact)])?;
            }
            
            if let Some(location) = &service.location {
                client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Location: {}", location)])?;
            }
            
            if !service.capabilities.is_empty() {
                client.send_numeric(NumericReply::RplService, &[&service.name, &format!("Capabilities: {}", service.capabilities.join(", "))])?;
            }
        } else {
            client.send_numeric(NumericReply::ErrNoSuchService, &[service_name, "No such service"])?;
        }
        
        Ok(())
    }
    
    /// Show service statistics
    async fn show_service_stats(&self, client: &Client, user: &User) -> Result<()> {
        let stats = self.stats.read().await;
        
        client.send_numeric(NumericReply::RplService, &["Service Statistics"])?;
        client.send_numeric(NumericReply::RplService, &[&format!("Total services: {}", stats.total_services)])?;
        client.send_numeric(NumericReply::RplService, &[&format!("Active services: {}", stats.active_services)])?;
        client.send_numeric(NumericReply::RplService, &[&format!("Inactive services: {}", stats.inactive_services)])?;
        client.send_numeric(NumericReply::RplService, &[&format!("Total registrations: {}", stats.total_registrations)])?;
        client.send_numeric(NumericReply::RplService, &[&format!("Total deregistrations: {}", stats.total_deregistrations)])?;
        
        Ok(())
    }
    
    /// Show services help
    async fn show_services_help(&self, client: &Client, user: &User) -> Result<()> {
        client.send_numeric(NumericReply::RplService, &["Services Help"])?;
        client.send_numeric(NumericReply::RplService, &["SERVICES - List all services"])?;
        client.send_numeric(NumericReply::RplService, &["SERVICES LIST - List all services"])?;
        client.send_numeric(NumericReply::RplService, &["SERVICES INFO <name> - Show service information"])?;
        client.send_numeric(NumericReply::RplService, &["SERVICES STATS - Show service statistics"])?;
        client.send_numeric(NumericReply::RplService, &["SERVICES HELP - Show this help"])?;
        client.send_numeric(NumericReply::RplService, &["SERVICE <name> <type> <desc> [version] [server] - Register service"])?;
        client.send_numeric(NumericReply::RplService, &["UNSERVICE <name> - Deregister service"])?;
        
        Ok(())
    }
    
    /// Check if service name is valid
    fn is_valid_service_name(&self, name: &str) -> bool {
        if name.is_empty() || name.len() > 50 {
            return false;
        }
        
        // Check for valid characters
        for ch in name.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
                return false;
            }
        }
        
        // Check prefix
        if !name.starts_with(&self.config.service_prefix) {
            return false;
        }
        
        true
    }
    
    /// Parse service type
    fn parse_service_type(&self, service_type: &str) -> ServiceType {
        match service_type.to_uppercase().as_str() {
            "NICKSERV" => ServiceType::NickServ,
            "CHANSERV" => ServiceType::ChanServ,
            "MEMOSERV" => ServiceType::MemoServ,
            "OPERSERV" => ServiceType::OperServ,
            "BOTSERV" => ServiceType::BotServ,
            _ => {
                if self.config.allow_custom_services {
                    ServiceType::Custom(service_type.to_string())
                } else {
                    ServiceType::Custom("Unknown".to_string())
                }
            }
        }
    }
    
    /// Get current time as Unix timestamp
    fn get_current_time(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Format time as readable string
    fn format_time(&self, timestamp: u64) -> String {
        use chrono::{DateTime, Utc, NaiveDateTime};
        let naive = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).unwrap_or_default();
        let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }
    
    /// Update service last seen time
    pub async fn update_service_last_seen(&self, service_name: &str) -> Result<()> {
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_name) {
            service.last_seen = self.get_current_time();
            service.is_active = true;
        }
        Ok(())
    }
    
    /// Mark service as inactive
    pub async fn mark_service_inactive(&self, service_name: &str) -> Result<()> {
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_name) {
            service.is_active = false;
        }
        Ok(())
    }
    
    /// Clean up inactive services
    pub async fn cleanup_inactive_services(&self) -> Result<()> {
        if !self.config.auto_cleanup {
            return Ok(());
        }
        
        let current_time = self.get_current_time();
        let mut services = self.services.write().await;
        let mut to_remove = Vec::new();
        
        for (name, service) in services.iter() {
            if !service.is_active && (current_time - service.last_seen) > self.config.service_timeout {
                to_remove.push(name.clone());
            }
        }
        
        for name in to_remove {
            services.remove(&name);
            info!("Cleaned up inactive service: {}", name);
        }
        
        Ok(())
    }
    
    /// Get service by name
    pub async fn get_service(&self, name: &str) -> Option<Service> {
        let services = self.services.read().await;
        services.get(name).cloned()
    }
    
    /// Get all services
    pub async fn get_all_services(&self) -> Vec<Service> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::NickServ => write!(f, "NickServ"),
            ServiceType::ChanServ => write!(f, "ChanServ"),
            ServiceType::MemoServ => write!(f, "MemoServ"),
            ServiceType::OperServ => write!(f, "OperServ"),
            ServiceType::BotServ => write!(f, "BotServ"),
            ServiceType::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[async_trait]
impl Module for ServicesModule {
    fn name(&self) -> &str {
        "services"
    }
    
    fn description(&self) -> &str {
        "Provides service registration and management system"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("{} module initialized", self.name());
        Ok(())
    }

    async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ModuleResult> {
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled),
        };

        match message.command {
            MessageType::Custom(ref cmd) if cmd == "SERVICES" => {
                self.handle_services(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "SERVICE" => {
                self.handle_service_registration(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            MessageType::Custom(ref cmd) if cmd == "UNSERVICE" => {
                self.handle_service_deregistration(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
    }

    async fn handle_server_message(&mut self, _server: &str, _message: &Message) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }

    async fn handle_user_registration(&mut self, _user: &User) -> Result<()> {
        Ok(())
    }

    async fn handle_user_disconnection(&mut self, _user: &User) -> Result<()> {
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
        info!("Services module cleaned up");
        Ok(())
    }
}

impl Default for ServicesModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_service_config_default() {
        let config = ServiceConfig::default();
        assert_eq!(config.max_services, 100);
        assert_eq!(config.service_timeout, 3600);
        assert!(config.require_authentication);
        assert!(config.allow_custom_services);
        assert_eq!(config.service_prefix, "Service");
        assert!(config.auto_cleanup);
    }
    
    #[test]
    fn test_services_module_creation() {
        let module = ServicesModule::new();
        assert_eq!(module.config.max_services, 100);
        assert_eq!(module.config.service_prefix, "Service");
    }
    
    #[test]
    fn test_service_name_validation() {
        let module = ServicesModule::new();
        
        assert!(module.is_valid_service_name("ServiceBot"));
        assert!(module.is_valid_service_name("Service-123"));
        assert!(module.is_valid_service_name("Service_Test"));
        
        assert!(!module.is_valid_service_name(""));
        assert!(!module.is_valid_service_name("Bot")); // No prefix
        assert!(!module.is_valid_service_name("Service@Bot")); // Invalid character
        assert!(!module.is_valid_service_name(&"a".repeat(51))); // Too long
    }
    
    #[test]
    fn test_service_type_parsing() {
        let module = ServicesModule::new();
        
        assert_eq!(module.parse_service_type("NICKSERV"), ServiceType::NickServ);
        assert_eq!(module.parse_service_type("CHANSERV"), ServiceType::ChanServ);
        assert_eq!(module.parse_service_type("MEMOSERV"), ServiceType::MemoServ);
        assert_eq!(module.parse_service_type("OPERSERV"), ServiceType::OperServ);
        assert_eq!(module.parse_service_type("BOTSERV"), ServiceType::BotServ);
        
        if module.config.allow_custom_services {
            assert_eq!(module.parse_service_type("CUSTOM"), ServiceType::Custom("CUSTOM".to_string()));
        }
    }
    
    #[tokio::test]
    async fn test_service_statistics() {
        let module = ServicesModule::new();
        let stats = module.stats.read().await;

        assert_eq!(stats.total_services, 0);
        assert_eq!(stats.active_services, 0);
        assert_eq!(stats.inactive_services, 0);
        assert_eq!(stats.total_registrations, 0);
        assert_eq!(stats.total_deregistrations, 0);
    }
}
