//! Module-specific numeric replies system
//! 
//! Allows modules to define their own numeric replies without cluttering the core numeric.rs file.

use crate::Message;
use std::collections::HashMap;

/// A module-specific numeric reply
#[derive(Debug, Clone)]
pub struct ModuleNumeric {
    /// The numeric code
    pub code: u16,
    /// The name of the numeric (e.g., "RPL_HELPTXT")
    pub name: String,
    /// The module that owns this numeric
    pub module: String,
}

/// Manager for module-specific numeric replies
#[derive(Debug)]
pub struct ModuleNumericManager {
    /// Registered numerics by name
    numerics: HashMap<String, ModuleNumeric>,
    /// Numerics by code for quick lookup
    codes: HashMap<u16, ModuleNumeric>,
}

impl ModuleNumericManager {
    /// Create a new module numeric manager
    pub fn new() -> Self {
        Self {
            numerics: HashMap::new(),
            codes: HashMap::new(),
        }
    }

    /// Register a numeric reply for a module
    pub fn register_numeric(&mut self, module: &str, name: &str, code: u16) -> Result<(), String> {
        let numeric = ModuleNumeric {
            code,
            name: name.to_string(),
            module: module.to_string(),
        };

        // Check for conflicts
        if self.numerics.contains_key(name) {
            return Err(format!("Numeric '{}' is already registered", name));
        }
        if self.codes.contains_key(&code) {
            return Err(format!("Numeric code {} is already registered", code));
        }

        self.numerics.insert(name.to_string(), numeric.clone());
        self.codes.insert(code, numeric);
        Ok(())
    }

    /// Get a numeric by name
    pub fn get_numeric(&self, name: &str) -> Option<&ModuleNumeric> {
        self.numerics.get(name)
    }

    /// Get a numeric by code
    pub fn get_numeric_by_code(&self, code: u16) -> Option<&ModuleNumeric> {
        self.codes.get(&code)
    }

    /// Check if a numeric is registered
    pub fn has_numeric(&self, name: &str) -> bool {
        self.numerics.contains_key(name)
    }

    /// Get all numerics for a specific module
    pub fn get_module_numerics(&self, module: &str) -> Vec<&ModuleNumeric> {
        self.numerics.values()
            .filter(|n| n.module == module)
            .collect()
    }

    /// Unregister all numerics for a module
    pub fn unregister_module(&mut self, module: &str) {
        let to_remove: Vec<_> = self.numerics.iter()
            .filter(|(_, n)| n.module == module)
            .map(|(name, numeric)| (name.clone(), numeric.code))
            .collect();

        for (name, code) in to_remove {
            self.numerics.remove(&name);
            self.codes.remove(&code);
        }
    }
}

/// Extension trait for Client to support module numerics
pub trait ModuleNumericClient {
    /// Send a module-specific numeric reply
    fn send_module_numeric(&self, manager: &ModuleNumericManager, numeric_name: &str, params: &[&str]) -> crate::Result<()>;
}

impl ModuleNumericClient for crate::Client {
    fn send_module_numeric(&self, manager: &ModuleNumericManager, numeric_name: &str, params: &[&str]) -> crate::Result<()> {
        if let Some(numeric) = manager.get_numeric(numeric_name) {
            let message = Message::new(
                crate::MessageType::Custom(format!("{:03}", numeric.code)),
                params.iter().map(|s| s.to_string()).collect()
            );
            self.send(message)
        } else {
            Err(crate::Error::MessageParse(format!("Unknown numeric: {}", numeric_name)))
        }
    }
}

/// Helper macro to define module numerics
#[macro_export]
macro_rules! define_module_numerics {
    ($module:ident, $manager:expr, {
        $($name:ident = $code:expr),* $(,)?
    }) => {
        $(
            $manager.register_numeric(stringify!($module), stringify!($name), $code)
                .expect(&format!("Failed to register numeric {}", stringify!($name)));
        )*
    };
}

/// Helper macro to send module numerics
#[macro_export]
macro_rules! send_module_numeric {
    ($client:expr, $manager:expr, $numeric:ident, $($param:expr),*) => {
        $client.send_module_numeric($manager, stringify!($numeric), &[$($param),*])
    };
}
