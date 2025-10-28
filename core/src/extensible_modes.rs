//! Extensible user mode system
//!
//! This module allows messaging modules and other components to register
//! custom user modes that aren't part of the core IRC standard.

use std::collections::HashMap;
use std::sync::RwLock;
use lazy_static::lazy_static;

/// Information about a custom user mode
#[derive(Debug, Clone)]
pub struct CustomUserMode {
    /// The character representing this mode (e.g., 'g' for globops)
    pub character: char,
    /// Description of what this mode does
    pub description: String,
    /// Whether this mode requires operator privileges to set/unset
    pub requires_operator: bool,
    /// Whether this mode can only be set by the user themselves
    pub self_only: bool,
    /// Whether this mode can only be set via OPER command
    pub oper_only: bool,
    /// The module that registered this mode
    pub module_name: String,
}

/// Registry for custom user modes
pub struct ExtensibleModeRegistry {
    modes: HashMap<char, CustomUserMode>,
}

impl ExtensibleModeRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            modes: HashMap::new(),
        }
    }
    
    /// Register a custom user mode
    pub fn register_mode(&mut self, mode: CustomUserMode) -> Result<(), String> {
        // Check if mode is already registered
        if self.modes.contains_key(&mode.character) {
            return Err(format!("Mode '{}' is already registered", mode.character));
        }
        
        // Check if mode conflicts with core modes
        if is_core_mode(mode.character) {
            return Err(format!("Mode '{}' conflicts with core IRC mode", mode.character));
        }
        
        self.modes.insert(mode.character, mode);
        Ok(())
    }
    
    /// Unregister a custom user mode
    pub fn unregister_mode(&mut self, character: char, module_name: &str) -> Result<(), String> {
        if let Some(mode) = self.modes.get(&character) {
            if mode.module_name == module_name {
                self.modes.remove(&character);
                Ok(())
            } else {
                Err(format!("Mode '{}' was registered by a different module", character))
            }
        } else {
            Err(format!("Mode '{}' is not registered", character))
        }
    }
    
    /// Get information about a custom mode
    pub fn get_mode(&self, character: char) -> Option<&CustomUserMode> {
        self.modes.get(&character)
    }
    
    /// Check if a character is a valid (core or custom) user mode
    pub fn is_valid_mode(&self, character: char) -> bool {
        is_core_mode(character) || self.modes.contains_key(&character)
    }
    
    /// Get all registered custom modes
    pub fn get_all_modes(&self) -> Vec<&CustomUserMode> {
        self.modes.values().collect()
    }
    
    /// Get modes registered by a specific module
    pub fn get_modes_by_module(&self, module_name: &str) -> Vec<&CustomUserMode> {
        self.modes.values()
            .filter(|mode| mode.module_name == module_name)
            .collect()
    }
    
    /// Validate mode change for a custom mode
    pub fn validate_mode_change(
        &self,
        character: char,
        adding: bool,
        target_user: &str,
        requesting_user: &str,
        requesting_user_is_operator: bool,
    ) -> Result<(), String> {
        let mode = match self.get_mode(character) {
            Some(mode) => mode,
            None => return Err(format!("Mode '{}' is not registered", character)),
        };
        
        let is_self = target_user == requesting_user;
        
        // Check if mode can only be set by OPER command
        if adding && mode.oper_only {
            return Err("This mode can only be granted through OPER command".to_string());
        }
        
        // Check operator requirements
        if mode.requires_operator && !requesting_user_is_operator {
            return Err("Permission denied: Operator privileges required".to_string());
        }
        
        // Check if mode can only be set by the user themselves
        if !is_self && mode.self_only {
            return Err("You can only change your own modes".to_string());
        }
        
        Ok(())
    }
}

impl Default for ExtensibleModeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a character is a core IRC mode
fn is_core_mode(character: char) -> bool {
    matches!(character, 'a' | 'i' | 'r' | 'o' | 'O' | 's')
}

// Global registry instance
lazy_static! {
    static ref MODE_REGISTRY: RwLock<ExtensibleModeRegistry> = RwLock::new(ExtensibleModeRegistry::new());
}

/// Register a custom user mode globally
pub fn register_custom_mode(mode: CustomUserMode) -> Result<(), String> {
    let mut registry = MODE_REGISTRY.write()
        .map_err(|e| format!("Lock poisoned: {}", e))?;
    registry.register_mode(mode)
}

/// Unregister a custom user mode globally
pub fn unregister_custom_mode(character: char, module_name: &str) -> Result<(), String> {
    let mut registry = MODE_REGISTRY.write()
        .map_err(|e| format!("Lock poisoned: {}", e))?;
    registry.unregister_mode(character, module_name)
}

/// Check if a character is a valid user mode (core or custom)
pub fn is_valid_user_mode(character: char) -> bool {
    match MODE_REGISTRY.read() {
        Ok(registry) => registry.is_valid_mode(character),
        Err(e) => {
            tracing::error!("Lock poisoned in is_valid_user_mode: {}", e);
            false
        }
    }
}

/// Get information about a custom mode
pub fn get_custom_mode(character: char) -> Option<CustomUserMode> {
    match MODE_REGISTRY.read() {
        Ok(registry) => registry.get_mode(character).cloned(),
        Err(e) => {
            tracing::error!("Lock poisoned in get_custom_mode: {}", e);
            None
        }
    }
}

/// Validate a custom mode change
pub fn validate_custom_mode_change(
    character: char,
    adding: bool,
    target_user: &str,
    requesting_user: &str,
    requesting_user_is_operator: bool,
) -> Result<(), String> {
    let registry = MODE_REGISTRY.read()
        .map_err(|e| format!("Lock poisoned: {}", e))?;
    registry.validate_mode_change(character, adding, target_user, requesting_user, requesting_user_is_operator)
}

/// Get all custom modes
pub fn get_all_custom_modes() -> Vec<CustomUserMode> {
    match MODE_REGISTRY.read() {
        Ok(registry) => registry.get_all_modes().into_iter().cloned().collect(),
        Err(e) => {
            tracing::error!("Lock poisoned in get_all_custom_modes: {}", e);
            Vec::new()
        }
    }
}

/// Get custom modes by module
pub fn get_custom_modes_by_module(module_name: &str) -> Vec<CustomUserMode> {
    match MODE_REGISTRY.read() {
        Ok(registry) => registry.get_modes_by_module(module_name).into_iter().cloned().collect(),
        Err(e) => {
            tracing::error!("Lock poisoned in get_custom_modes_by_module: {}", e);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_mode_registration() {
        let mut registry = ExtensibleModeRegistry::new();
        
        let globops_mode = CustomUserMode {
            character: 'g',
            description: "Receive global operator notices".to_string(),
            requires_operator: false,
            self_only: true,
            oper_only: false,
            module_name: "globops".to_string(),
        };
        
        assert!(registry.register_mode(globops_mode.clone()).is_ok());
        assert!(registry.is_valid_mode('g'));
        assert!(!registry.is_valid_mode('x'));
        
        let retrieved = registry.get_mode('g').unwrap();
        assert_eq!(retrieved.description, "Receive global operator notices");
    }

    #[test]
    fn test_mode_conflict() {
        let mut registry = ExtensibleModeRegistry::new();
        
        let core_mode = CustomUserMode {
            character: 'o',
            description: "Operator mode".to_string(),
            requires_operator: true,
            self_only: false,
            oper_only: true,
            module_name: "test".to_string(),
        };
        
        assert!(registry.register_mode(core_mode).is_err());
    }

    #[test]
    fn test_mode_validation() {
        let mut registry = ExtensibleModeRegistry::new();
        
        let operator_only_mode = CustomUserMode {
            character: 'g',
            description: "Admin mode".to_string(),
            requires_operator: true,
            self_only: false,
            oper_only: false,
            module_name: "admin".to_string(),
        };
        
        registry.register_mode(operator_only_mode).unwrap();
        
        // Non-operator should not be able to set operator-required mode
        assert!(registry.validate_mode_change('g', true, "user1", "user2", false).is_err());
        
        // Operator should be able to set it
        assert!(registry.validate_mode_change('g', true, "user1", "user2", true).is_ok());
    }
}
