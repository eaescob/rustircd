//! Extension registry system for better module management
//! 
//! This module provides a centralized registry for managing extensions and modules,
//! inspired by Solanum's modular architecture.

use crate::{User, Message, Client, Result, Error, ExtensionManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Extension registry for managing all extensions and modules
pub struct ExtensionRegistry {
    /// Core extension manager
    core_extension_manager: Arc<ExtensionManager>,
    /// Registered extensions
    extensions: Arc<RwLock<HashMap<String, RegisteredExtension>>>,
    /// Extension dependencies
    dependencies: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Extension load order
    load_order: Arc<RwLock<Vec<String>>>,
}

/// Registered extension information
#[derive(Debug, Clone)]
pub struct RegisteredExtension {
    /// Extension name
    pub name: String,
    /// Extension type
    pub extension_type: ExtensionType,
    /// Extension priority
    pub priority: u32,
    /// Whether extension is enabled
    pub enabled: bool,
    /// Extension dependencies
    pub dependencies: Vec<String>,
    /// Extension metadata
    pub metadata: ExtensionMetadata,
}

/// Extension types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionType {
    /// Core extension
    Core,
    /// Module extension
    Module,
    /// Service extension
    Service,
    /// Custom extension
    Custom,
}

/// Extension metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    /// Extension version
    pub version: String,
    /// Extension description
    pub description: String,
    /// Extension author
    pub author: String,
    /// Extension license
    pub license: String,
    /// Extension homepage
    pub homepage: Option<String>,
    /// Extension capabilities
    pub capabilities: Vec<String>,
}

/// Extension registration builder
pub struct ExtensionRegistrationBuilder {
    name: String,
    extension_type: ExtensionType,
    priority: u32,
    enabled: bool,
    dependencies: Vec<String>,
    metadata: ExtensionMetadata,
}

impl ExtensionRegistrationBuilder {
    /// Create a new registration builder
    pub fn new(name: String, extension_type: ExtensionType) -> Self {
        Self {
            name,
            extension_type,
            priority: 100,
            enabled: true,
            dependencies: Vec::new(),
            metadata: ExtensionMetadata {
                version: "1.0.0".to_string(),
                description: "No description".to_string(),
                author: "Unknown".to_string(),
                license: "MIT".to_string(),
                homepage: None,
                capabilities: Vec::new(),
            },
        }
    }
    
    /// Set extension priority
    pub fn priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set whether extension is enabled
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// Add dependency
    pub fn dependency(mut self, dependency: String) -> Self {
        self.dependencies.push(dependency);
        self
    }
    
    /// Set dependencies
    pub fn dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }
    
    /// Set metadata
    pub fn metadata(mut self, metadata: ExtensionMetadata) -> Self {
        self.metadata = metadata;
        self
    }
    
    /// Set version
    pub fn version(mut self, version: String) -> Self {
        self.metadata.version = version;
        self
    }
    
    /// Set description
    pub fn description(mut self, description: String) -> Self {
        self.metadata.description = description;
        self
    }
    
    /// Set author
    pub fn author(mut self, author: String) -> Self {
        self.metadata.author = author;
        self
    }
    
    /// Set license
    pub fn license(mut self, license: String) -> Self {
        self.metadata.license = license;
        self
    }
    
    /// Set homepage
    pub fn homepage(mut self, homepage: String) -> Self {
        self.metadata.homepage = Some(homepage);
        self
    }
    
    /// Add capability
    pub fn capability(mut self, capability: String) -> Self {
        self.metadata.capabilities.push(capability);
        self
    }
    
    /// Set capabilities
    pub fn capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.metadata.capabilities = capabilities;
        self
    }
    
    /// Build the registration
    pub fn build(self) -> RegisteredExtension {
        RegisteredExtension {
            name: self.name,
            extension_type: self.extension_type,
            priority: self.priority,
            enabled: self.enabled,
            dependencies: self.dependencies,
            metadata: self.metadata,
        }
    }
}

impl ExtensionRegistry {
    /// Create a new extension registry
    pub fn new() -> Self {
        Self {
            core_extension_manager: Arc::new(ExtensionManager::new()),
            extensions: Arc::new(RwLock::new(HashMap::new())),
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            load_order: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Register an extension
    pub async fn register_extension(&self, extension: RegisteredExtension) -> Result<()> {
        let name = extension.name.clone();
        
        // Check if extension already exists
        {
            let extensions = self.extensions.read().await;
            if extensions.contains_key(&name) {
                return Err(Error::InvalidInput(format!("Extension '{}' already registered", name)));
            }
        }
        
        // Validate dependencies
        self.validate_dependencies(&extension).await?;
        
        // Register extension
        {
            let mut extensions = self.extensions.write().await;
            extensions.insert(name.clone(), extension);
        }
        
        // Update dependencies
        {
            let mut dependencies = self.dependencies.write().await;
            dependencies.insert(name.clone(), extension.dependencies.clone());
        }
        
        // Update load order
        self.update_load_order().await?;
        
        tracing::info!("Registered extension: {}", name);
        Ok(())
    }
    
    /// Unregister an extension
    pub async fn unregister_extension(&self, name: &str) -> Result<()> {
        // Check if extension exists
        {
            let extensions = self.extensions.read().await;
            if !extensions.contains_key(name) {
                return Err(Error::InvalidInput(format!("Extension '{}' not found", name)));
            }
        }
        
        // Check if other extensions depend on this one
        {
            let dependencies = self.dependencies.read().await;
            for (ext_name, deps) in dependencies.iter() {
                if deps.contains(&name.to_string()) {
                    return Err(Error::InvalidInput(format!(
                        "Cannot unregister extension '{}' because '{}' depends on it", 
                        name, ext_name
                    )));
                }
            }
        }
        
        // Remove extension
        {
            let mut extensions = self.extensions.write().await;
            extensions.remove(name);
        }
        
        // Remove from dependencies
        {
            let mut dependencies = self.dependencies.write().await;
            dependencies.remove(name);
        }
        
        // Update load order
        self.update_load_order().await?;
        
        tracing::info!("Unregistered extension: {}", name);
        Ok(())
    }
    
    /// Enable an extension
    pub async fn enable_extension(&self, name: &str) -> Result<()> {
        let mut extensions = self.extensions.write().await;
        if let Some(extension) = extensions.get_mut(name) {
            extension.enabled = true;
            tracing::info!("Enabled extension: {}", name);
            Ok(())
        } else {
            Err(Error::InvalidInput(format!("Extension '{}' not found", name)))
        }
    }
    
    /// Disable an extension
    pub async fn disable_extension(&self, name: &str) -> Result<()> {
        let mut extensions = self.extensions.write().await;
        if let Some(extension) = extensions.get_mut(name) {
            extension.enabled = false;
            tracing::info!("Disabled extension: {}", name);
            Ok(())
        } else {
            Err(Error::InvalidInput(format!("Extension '{}' not found", name)))
        }
    }
    
    /// Get extension information
    pub async fn get_extension(&self, name: &str) -> Option<RegisteredExtension> {
        let extensions = self.extensions.read().await;
        extensions.get(name).cloned()
    }
    
    /// List all extensions
    pub async fn list_extensions(&self) -> Vec<RegisteredExtension> {
        let extensions = self.extensions.read().await;
        extensions.values().cloned().collect()
    }
    
    /// List enabled extensions
    pub async fn list_enabled_extensions(&self) -> Vec<RegisteredExtension> {
        let extensions = self.extensions.read().await;
        extensions.values()
            .filter(|ext| ext.enabled)
            .cloned()
            .collect()
    }
    
    /// Get extension load order
    pub async fn get_load_order(&self) -> Vec<String> {
        let load_order = self.load_order.read().await;
        load_order.clone()
    }
    
    /// Get extension dependencies
    pub async fn get_dependencies(&self, name: &str) -> Vec<String> {
        let dependencies = self.dependencies.read().await;
        dependencies.get(name).cloned().unwrap_or_default()
    }
    
    /// Get extensions that depend on the given extension
    pub async fn get_dependents(&self, name: &str) -> Vec<String> {
        let dependencies = self.dependencies.read().await;
        dependencies.iter()
            .filter(|(_, deps)| deps.contains(&name.to_string()))
            .map(|(ext_name, _)| ext_name.clone())
            .collect()
    }
    
    /// Validate extension dependencies
    async fn validate_dependencies(&self, extension: &RegisteredExtension) -> Result<()> {
        let extensions = self.extensions.read().await;
        
        for dependency in &extension.dependencies {
            if !extensions.contains_key(dependency) {
                return Err(Error::InvalidInput(format!(
                    "Extension '{}' depends on '{}' which is not registered", 
                    extension.name, dependency
                )));
            }
        }
        
        Ok(())
    }
    
    /// Update extension load order based on dependencies
    async fn update_load_order(&self) -> Result<()> {
        let extensions = self.extensions.read().await;
        let dependencies = self.dependencies.read().await;
        
        let mut load_order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();
        
        // Topological sort
        for (name, extension) in extensions.iter() {
            if extension.enabled {
                self.visit_extension(
                    name, 
                    &extensions, 
                    &dependencies, 
                    &mut load_order, 
                    &mut visited, 
                    &mut visiting
                )?;
            }
        }
        
        // Sort by priority
        load_order.sort_by(|a, b| {
            let a_priority = extensions.get(a).map(|e| e.priority).unwrap_or(0);
            let b_priority = extensions.get(b).map(|e| e.priority).unwrap_or(0);
            a_priority.cmp(&b_priority)
        });
        
        let mut load_order_guard = self.load_order.write().await;
        *load_order_guard = load_order;
        
        Ok(())
    }
    
    /// Visit extension for topological sort
    fn visit_extension(
        &self,
        name: &str,
        extensions: &HashMap<String, RegisteredExtension>,
        dependencies: &HashMap<String, Vec<String>>,
        load_order: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if visiting.contains(name) {
            return Err(Error::InvalidInput(format!("Circular dependency detected involving '{}'", name)));
        }
        
        if visited.contains(name) {
            return Ok(());
        }
        
        visiting.insert(name.to_string());
        
        if let Some(deps) = dependencies.get(name) {
            for dep in deps {
                self.visit_extension(dep, extensions, dependencies, load_order, visited, visiting)?;
            }
        }
        
        visiting.remove(name);
        visited.insert(name.to_string());
        load_order.push(name.to_string());
        
        Ok(())
    }
    
    /// Get core extension manager
    pub fn get_core_extension_manager(&self) -> Arc<ExtensionManager> {
        self.core_extension_manager.clone()
    }
    
    /// Initialize all enabled extensions
    pub async fn initialize_extensions(&self) -> Result<()> {
        let load_order = self.get_load_order().await;
        
        for extension_name in load_order {
            if let Some(extension) = self.get_extension(&extension_name).await {
                if extension.enabled {
                    tracing::info!("Initializing extension: {}", extension_name);
                    // Here you would initialize the specific extension
                    // This would be implemented by the specific extension types
                }
            }
        }
        
        Ok(())
    }
    
    /// Get extension statistics
    pub async fn get_statistics(&self) -> ExtensionStatistics {
        let extensions = self.extensions.read().await;
        let load_order = self.load_order.read().await;
        
        let total_extensions = extensions.len();
        let enabled_extensions = extensions.values().filter(|e| e.enabled).count();
        let disabled_extensions = total_extensions - enabled_extensions;
        
        let mut extension_types = std::collections::HashMap::new();
        for extension in extensions.values() {
            let count = extension_types.entry(extension.extension_type.clone()).or_insert(0);
            *count += 1;
        }
        
        ExtensionStatistics {
            total_extensions,
            enabled_extensions,
            disabled_extensions,
            extension_types,
            load_order: load_order.clone(),
        }
    }
}

/// Extension statistics
#[derive(Debug, Clone)]
pub struct ExtensionStatistics {
    /// Total number of extensions
    pub total_extensions: usize,
    /// Number of enabled extensions
    pub enabled_extensions: usize,
    /// Number of disabled extensions
    pub disabled_extensions: usize,
    /// Extension counts by type
    pub extension_types: std::collections::HashMap<ExtensionType, usize>,
    /// Extension load order
    pub load_order: Vec<String>,
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension registry manager
pub struct ExtensionRegistryManager {
    registry: Arc<ExtensionRegistry>,
}

impl ExtensionRegistryManager {
    /// Create a new extension registry manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ExtensionRegistry::new()),
        }
    }
    
    /// Get the registry
    pub fn get_registry(&self) -> Arc<ExtensionRegistry> {
        self.registry.clone()
    }
    
    /// Register core extensions
    pub async fn register_core_extensions(&self) -> Result<()> {
        // Register identify message extension
        let identify_msg = ExtensionRegistrationBuilder::new(
            "identify-msg".to_string(),
            ExtensionType::Core
        )
        .description("Adds account information to messages".to_string())
        .author("Rust IRC Daemon".to_string())
        .capability("identify-msg".to_string())
        .priority(10)
        .build();
        
        self.registry.register_extension(identify_msg).await?;
        
        // Register account tracking extension
        let account_tracking = ExtensionRegistrationBuilder::new(
            "account-tracking".to_string(),
            ExtensionType::Core
        )
        .description("Tracks user account information".to_string())
        .author("Rust IRC Daemon".to_string())
        .capability("account-tracking".to_string())
        .priority(20)
        .build();
        
        self.registry.register_extension(account_tracking).await?;
        
        // Register server time extension
        let server_time = ExtensionRegistrationBuilder::new(
            "server-time".to_string(),
            ExtensionType::Core
        )
        .description("Provides server time information".to_string())
        .author("Rust IRC Daemon".to_string())
        .capability("server-time".to_string())
        .priority(30)
        .build();
        
        self.registry.register_extension(server_time).await?;
        
        // Register batch extension
        let batch = ExtensionRegistrationBuilder::new(
            "batch".to_string(),
            ExtensionType::Core
        )
        .description("Handles message batching".to_string())
        .author("Rust IRC Daemon".to_string())
        .capability("batch".to_string())
        .priority(40)
        .build();
        
        self.registry.register_extension(batch).await?;
        
        Ok(())
    }
    
    /// Register module extensions
    pub async fn register_module_extensions(&self) -> Result<()> {
        // Register oper module
        let oper = ExtensionRegistrationBuilder::new(
            "oper".to_string(),
            ExtensionType::Module
        )
        .description("Operator management module".to_string())
        .author("Rust IRC Daemon".to_string())
        .capability("oper".to_string())
        .priority(100)
        .build();
        
        self.registry.register_extension(oper).await?;
        
        // Register SASL module
        let sasl = ExtensionRegistrationBuilder::new(
            "sasl".to_string(),
            ExtensionType::Module
        )
        .description("SASL authentication module".to_string())
        .author("Rust IRC Daemon".to_string())
        .capability("sasl".to_string())
        .priority(110)
        .build();
        
        self.registry.register_extension(sasl).await?;
        
        Ok(())
    }
    
    /// Register service extensions
    pub async fn register_service_extensions(&self) -> Result<()> {
        // Register Atheme integration
        let atheme = ExtensionRegistrationBuilder::new(
            "atheme".to_string(),
            ExtensionType::Service
        )
        .description("Atheme services integration".to_string())
        .author("Rust IRC Daemon".to_string())
        .capability("atheme".to_string())
        .priority(200)
        .build();
        
        self.registry.register_extension(atheme).await?;
        
        Ok(())
    }
    
    /// Initialize all extensions
    pub async fn initialize_all(&self) -> Result<()> {
        // Register all extension types
        self.register_core_extensions().await?;
        self.register_module_extensions().await?;
        self.register_service_extensions().await?;
        
        // Initialize all enabled extensions
        self.registry.initialize_extensions().await?;
        
        Ok(())
    }
}

impl Default for ExtensionRegistryManager {
    fn default() -> Self {
        Self::new()
    }
}
