//! Core Extensions
//! 
//! This module contains core extensions inspired by Solanum IRCd.
//! Each extension is in its own file for better modularity and maintainability.

pub mod identify_msg;
pub mod account_tracking;
pub mod server_time;
pub mod batch;
pub mod ip_cloak_v2;

// Re-export the main extension types
pub use identify_msg::IdentifyMessageExtension;
pub use account_tracking::{AccountTrackingExtension, AccountInfo};
pub use server_time::ServerTimeExtension;
pub use batch::{BatchExtension, BatchInfo};
pub use ip_cloak_v2::{IpCloakExtension, IpCloakConfig, IpCloakStats, IpCloakConfigBuilder};

/// Core extension manager for managing all core extensions
pub struct CoreExtensionManager {
    /// Extension manager
    extension_manager: std::sync::Arc<crate::extensions::ExtensionManager>,
    /// Account tracking extension
    account_tracking: std::sync::Arc<AccountTrackingExtension>,
    /// Identify message extension
    identify_message: std::sync::Arc<IdentifyMessageExtension>,
    /// Server time extension
    server_time: std::sync::Arc<ServerTimeExtension>,
    /// Batch extension
    batch: std::sync::Arc<BatchExtension>,
    /// IP cloaking extension
    ip_cloak: std::sync::Arc<IpCloakExtension>,
}

impl CoreExtensionManager {
    /// Create a new core extension manager
    pub fn new(service_name: String) -> Self {
        let extension_manager = std::sync::Arc::new(crate::extensions::ExtensionManager::new());
        let account_tracking = std::sync::Arc::new(AccountTrackingExtension::new(service_name.clone()));
        let identify_message = std::sync::Arc::new(IdentifyMessageExtension::new(service_name.clone(), true));
        let server_time = std::sync::Arc::new(ServerTimeExtension::new(true));
        let batch = std::sync::Arc::new(BatchExtension::new());
        let ip_cloak = std::sync::Arc::new(IpCloakExtension::new(IpCloakConfig::default()));
        
        Self {
            extension_manager,
            account_tracking,
            identify_message,
            server_time,
            batch,
            ip_cloak,
        }
    }
    
    /// Initialize all core extensions
    pub async fn initialize(&self) -> Result<(), crate::Error> {
        // Register all extensions
        self.extension_manager.register_user_extension(Box::new(self.account_tracking.clone())).await?;
        self.extension_manager.register_message_tag_extension(Box::new(self.identify_message.clone())).await?;
        self.extension_manager.register_message_tag_extension(Box::new(self.server_time.clone())).await?;
        self.extension_manager.register_message_extension(Box::new(self.batch.clone())).await?;
        self.extension_manager.register_user_extension(Box::new(self.ip_cloak.clone())).await?;
        
        Ok(())
    }
    
    /// Get the extension manager
    pub fn get_extension_manager(&self) -> std::sync::Arc<crate::extensions::ExtensionManager> {
        self.extension_manager.clone()
    }
    
    /// Get account tracking extension
    pub fn get_account_tracking(&self) -> std::sync::Arc<AccountTrackingExtension> {
        self.account_tracking.clone()
    }
    
    /// Get identify message extension
    pub fn get_identify_message(&self) -> std::sync::Arc<IdentifyMessageExtension> {
        self.identify_message.clone()
    }
    
    /// Get server time extension
    pub fn get_server_time(&self) -> std::sync::Arc<ServerTimeExtension> {
        self.server_time.clone()
    }
    
    /// Get batch extension
    pub fn get_batch(&self) -> std::sync::Arc<BatchExtension> {
        self.batch.clone()
    }
    
    /// Get IP cloaking extension
    pub fn get_ip_cloak(&self) -> std::sync::Arc<IpCloakExtension> {
        self.ip_cloak.clone()
    }
}

impl Default for CoreExtensionManager {
    fn default() -> Self {
        Self::new("services.example.org".to_string())
    }
}
