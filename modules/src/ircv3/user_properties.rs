//! IRCv3 Changing User Properties

use rustircd_core::{Error, Result};
use std::collections::HashMap;
use uuid::Uuid;

/// User properties handler
pub struct UserProperties {
    /// User properties by user ID
    user_properties: HashMap<Uuid, UserPropertySet>,
    /// Property change history
    property_history: HashMap<Uuid, Vec<PropertyChange>>,
}

/// Set of user properties
#[derive(Debug, Clone)]
pub struct UserPropertySet {
    /// Real name
    pub realname: Option<String>,
    /// Hostname
    pub hostname: Option<String>,
    /// Username
    pub username: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// Custom properties
    pub custom: HashMap<String, String>,
}

/// Property change record
#[derive(Debug, Clone)]
pub struct PropertyChange {
    /// Property name
    pub property: String,
    /// Old value
    pub old_value: Option<String>,
    /// New value
    pub new_value: Option<String>,
    /// Change timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Change reason
    pub reason: Option<String>,
}

impl UserProperties {
    pub fn new() -> Self {
        Self {
            user_properties: HashMap::new(),
            property_history: HashMap::new(),
        }
    }
    
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing user properties");
        Ok(())
    }
    
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up user properties");
        Ok(())
    }
    
    /// Set user property
    pub fn set_property(&mut self, user_id: Uuid, property: String, value: Option<String>, reason: Option<String>) -> Result<()> {
        let old_value = self.get_property(&user_id, &property);
        
        let user_props = self.user_properties.entry(user_id).or_insert_with(|| UserPropertySet {
            realname: None,
            hostname: None,
            username: None,
            display_name: None,
            custom: HashMap::new(),
        });
        
        match property.as_str() {
            "realname" => {
                user_props.realname = value.clone();
            }
            "hostname" => {
                user_props.hostname = value.clone();
            }
            "username" => {
                user_props.username = value.clone();
            }
            "display_name" => {
                user_props.display_name = value.clone();
            }
            _ => {
                if let Some(ref val) = value {
                    user_props.custom.insert(property.clone(), val.clone());
                } else {
                    user_props.custom.remove(&property);
                }
            }
        }
        
        // Record change
        let change = PropertyChange {
            property: property.clone(),
            old_value: old_value.clone(),
            new_value: value.clone(),
            timestamp: chrono::Utc::now(),
            reason,
        };
        
        self.property_history.entry(user_id)
            .or_insert_with(Vec::new)
            .push(change);
        
        tracing::info!("Set property {} for user {}: {:?} -> {:?}", property, user_id, old_value, value);
        Ok(())
    }
    
    /// Get user property
    pub fn get_property(&self, user_id: &Uuid, property: &str) -> Option<String> {
        self.user_properties.get(user_id).and_then(|props| {
            match property {
                "realname" => props.realname.clone(),
                "hostname" => props.hostname.clone(),
                "username" => props.username.clone(),
                "display_name" => props.display_name.clone(),
                _ => props.custom.get(property).cloned(),
            }
        })
    }
    
    /// Get all user properties
    pub fn get_user_properties(&self, user_id: &Uuid) -> Option<&UserPropertySet> {
        self.user_properties.get(user_id)
    }
    
    /// Remove user property
    pub fn remove_property(&mut self, user_id: &Uuid, property: &str) -> Result<Option<String>> {
        let old_value = self.get_property(user_id, property);
        if let Some(user_props) = self.user_properties.get_mut(user_id) {
            
            match property {
                "realname" => {
                    user_props.realname = None;
                }
                "hostname" => {
                    user_props.hostname = None;
                }
                "username" => {
                    user_props.username = None;
                }
                "display_name" => {
                    user_props.display_name = None;
                }
                _ => {
                    user_props.custom.remove(property);
                }
            }
            
            // Record change
            let change = PropertyChange {
                property: property.to_string(),
                old_value: old_value.clone(),
                new_value: None,
                timestamp: chrono::Utc::now(),
                reason: None,
            };
            
            self.property_history.entry(*user_id)
                .or_insert_with(Vec::new)
                .push(change);
            
            Ok(old_value)
        } else {
            Err(Error::User("User not found".to_string()))
        }
    }
    
    /// Get property change history
    pub fn get_property_history(&self, user_id: &Uuid) -> Option<&Vec<PropertyChange>> {
        self.property_history.get(user_id)
    }
    
    /// Check if user has property
    pub fn has_property(&self, user_id: &Uuid, property: &str) -> bool {
        self.get_property(user_id, property).is_some()
    }
    
    /// Get all users with property
    pub fn get_users_with_property(&self, property: &str, value: &str) -> Vec<Uuid> {
        self.user_properties.iter()
            .filter(|(_, props)| {
                match property {
                    "realname" => props.realname.as_ref().map_or(false, |v| v == value),
                    "hostname" => props.hostname.as_ref().map_or(false, |v| v == value),
                    "username" => props.username.as_ref().map_or(false, |v| v == value),
                    "display_name" => props.display_name.as_ref().map_or(false, |v| v == value),
                    _ => props.custom.get(property).map_or(false, |v| v == value),
                }
            })
            .map(|(user_id, _)| *user_id)
            .collect()
    }
    
    /// Validate property value
    pub fn is_valid_property_value(property: &str, value: &str) -> bool {
        match property {
            "realname" => !value.is_empty() && value.len() <= 390,
            "hostname" => !value.is_empty() && value.len() <= 255 && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'),
            "username" => !value.is_empty() && value.len() <= 9 && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "display_name" => !value.is_empty() && value.len() <= 50,
            _ => !value.is_empty() && value.len() <= 100, // Custom properties
        }
    }
    
    /// Get property statistics
    pub fn get_property_stats(&self) -> PropertyStats {
        let mut property_counts = HashMap::new();
        let mut total_changes = 0;
        
        for history in self.property_history.values() {
            total_changes += history.len();
            for change in history {
                *property_counts.entry(change.property.clone()).or_insert(0) += 1;
            }
        }
        
        PropertyStats {
            total_users: self.user_properties.len(),
            total_changes,
            property_counts,
        }
    }
}

/// Property statistics
#[derive(Debug, Clone)]
pub struct PropertyStats {
    pub total_users: usize,
    pub total_changes: usize,
    pub property_counts: HashMap<String, usize>,
}
