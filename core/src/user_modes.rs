//! User mode management system
//!
//! This module implements user mode management according to RFC 1459 and IRC standards.
//! User modes control various aspects of user behavior and capabilities.

use std::collections::HashSet;
use std::fmt;

/// Standard IRC user modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserMode {
    /// Away mode - user is away
    Away,
    /// Invisible mode - user doesn't appear in WHO lists
    Invisible,
    /// Wallops mode - user receives wallop messages
    Wallops,
    /// Restricted mode - user is restricted from certain actions
    Restricted,
    /// Operator mode - user has operator privileges
    Operator,
    /// Local operator mode - user has local operator privileges
    LocalOperator,
    /// Receive server notices
    ServerNotices,
}

impl UserMode {
    /// Get the character representation of the mode
    pub fn to_char(&self) -> char {
        match self {
            UserMode::Away => 'a',
            UserMode::Invisible => 'i',
            UserMode::Wallops => 'w',
            UserMode::Restricted => 'r',
            UserMode::Operator => 'o',
            UserMode::LocalOperator => 'O',
            UserMode::ServerNotices => 's',
        }
    }

    /// Get mode from character
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'a' => Some(UserMode::Away),
            'i' => Some(UserMode::Invisible),
            'w' => Some(UserMode::Wallops),
            'r' => Some(UserMode::Restricted),
            'o' => Some(UserMode::Operator),
            'O' => Some(UserMode::LocalOperator),
            's' => Some(UserMode::ServerNotices),
            _ => None,
        }
    }

    /// Get description of what this mode does
    pub fn description(&self) -> &'static str {
        match self {
            UserMode::Away => "User is away",
            UserMode::Invisible => "User is invisible in WHO lists",
            UserMode::Wallops => "User receives wallop messages",
            UserMode::Restricted => "User is restricted",
            UserMode::Operator => "User has operator privileges",
            UserMode::LocalOperator => "User has local operator privileges",
            UserMode::ServerNotices => "User receives server notices",
        }
    }

    /// Check if this mode requires operator privileges to set/unset
    pub fn requires_operator(&self) -> bool {
        match self {
            UserMode::Operator => true,
            UserMode::LocalOperator => true,
            UserMode::Restricted => true,
            _ => false,
        }
    }

    /// Check if this mode can only be set by the OPER command (not MODE command)
    pub fn oper_only(&self) -> bool {
        match self {
            UserMode::Operator => true,
            UserMode::LocalOperator => true,
            _ => false,
        }
    }

    /// Check if this mode can only be set by the user themselves
    pub fn self_only(&self) -> bool {
        match self {
            UserMode::Away => true,
            UserMode::Invisible => true,
            UserMode::Wallops => true,
            UserMode::ServerNotices => true,
            _ => false,
        }
    }

    /// Check if this mode affects message routing
    pub fn affects_routing(&self) -> bool {
        match self {
            UserMode::Away => true,
            UserMode::Invisible => true,
            _ => false,
        }
    }
}

impl fmt::Display for UserMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

/// User mode manager for handling mode changes and validation
#[derive(Debug, Clone)]
pub struct UserModeManager {
    /// Current user modes
    modes: HashSet<UserMode>,
    /// User's operator status (separate from mode)
    is_operator: bool,
}

impl UserModeManager {
    /// Create a new user mode manager
    pub fn new() -> Self {
        Self {
            modes: HashSet::new(),
            is_operator: false,
        }
    }

    /// Create from existing modes
    pub fn from_modes(modes: HashSet<char>) -> Self {
        let user_modes: HashSet<UserMode> = modes
            .iter()
            .filter_map(|&c| UserMode::from_char(c))
            .collect();
        
        Self {
            modes: user_modes,
            is_operator: modes.contains(&'o'),
        }
    }

    /// Add a mode to the user
    pub fn add_mode(&mut self, mode: UserMode) -> Result<(), String> {
        // Validate mode addition
        if mode.requires_operator() && !self.is_operator {
            return Err(format!("Mode {} requires operator privileges", mode.to_char()));
        }

        self.modes.insert(mode);
        Ok(())
    }

    /// Remove a mode from the user
    pub fn remove_mode(&mut self, mode: UserMode) -> Result<(), String> {
        // Validate mode removal
        if mode.requires_operator() && !self.is_operator {
            return Err(format!("Mode {} requires operator privileges", mode.to_char()));
        }

        self.modes.remove(&mode);
        Ok(())
    }

    /// Check if user has a specific mode
    pub fn has_mode(&self, mode: UserMode) -> bool {
        self.modes.contains(&mode)
    }

    /// Get all modes as characters
    pub fn get_modes_chars(&self) -> HashSet<char> {
        self.modes.iter().map(|m| m.to_char()).collect()
    }

    /// Get modes as a sorted string
    pub fn modes_string(&self) -> String {
        let mut modes: Vec<char> = self.modes.iter().map(|m| m.to_char()).collect();
        modes.sort();
        modes.into_iter().collect()
    }

    /// Set operator status
    pub fn set_operator(&mut self, is_operator: bool) {
        self.is_operator = is_operator;
        if is_operator {
            self.modes.insert(UserMode::Operator);
        } else {
            self.modes.remove(&UserMode::Operator);
            self.modes.remove(&UserMode::LocalOperator);
        }
    }

    /// Check if user is an operator
    pub fn is_operator(&self) -> bool {
        self.is_operator
    }

    /// Validate mode change for a user
    pub fn validate_mode_change(
        &self,
        mode: UserMode,
        adding: bool,
        target_user: &str,
        requesting_user: &str,
        requesting_user_is_operator: bool,
    ) -> Result<(), String> {
        // Check if user is trying to change their own modes
        let is_self = target_user == requesting_user;

        // Check if mode can only be set by OPER command (not MODE command)
        if adding && mode.oper_only() {
            return Err("Operator mode can only be granted through OPER command".to_string());
        }

        // Check operator requirements for removal
        if !adding && mode.requires_operator() && !requesting_user_is_operator {
            return Err("Permission denied".to_string());
        }

        // Check if mode can only be set by the user themselves
        if !is_self && mode.self_only() {
            return Err("You can only change your own modes".to_string());
        }

        // Check if mode is already set/unset
        let currently_has = self.has_mode(mode);
        if adding && currently_has {
            return Err(format!("Mode {} is already set", mode.to_char()));
        }
        if !adding && !currently_has {
            return Err(format!("Mode {} is not set", mode.to_char()));
        }

        Ok(())
    }

    /// Get mode information for display
    pub fn get_mode_info(&self) -> Vec<(char, &'static str)> {
        self.modes
            .iter()
            .map(|mode| (mode.to_char(), mode.description()))
            .collect()
    }

    /// Check if user should appear in WHO lists
    pub fn should_show_in_who(&self) -> bool {
        !self.has_mode(UserMode::Invisible)
    }

    /// Check if user should receive wallops
    pub fn should_receive_wallops(&self) -> bool {
        self.has_mode(UserMode::Wallops)
    }

    /// Check if user should receive server notices
    pub fn should_receive_server_notices(&self) -> bool {
        self.has_mode(UserMode::ServerNotices)
    }

    /// Check if user is away
    pub fn is_away(&self) -> bool {
        self.has_mode(UserMode::Away)
    }

    /// Check if user is restricted
    pub fn is_restricted(&self) -> bool {
        self.has_mode(UserMode::Restricted)
    }
}

impl Default for UserModeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard IRC user mode characters
pub const STANDARD_USER_MODES: &[char] = &['a', 'i', 'w', 'r', 'o', 'O', 's'];

/// Check if a character is a valid user mode
pub fn is_valid_user_mode(c: char) -> bool {
    UserMode::from_char(c).is_some()
}

/// Parse user mode string into individual modes
pub fn parse_user_mode_string(mode_string: &str) -> Vec<UserMode> {
    mode_string
        .chars()
        .filter_map(UserMode::from_char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_mode_char_conversion() {
        assert_eq!(UserMode::Away.to_char(), 'a');
        assert_eq!(UserMode::Invisible.to_char(), 'i');
        assert_eq!(UserMode::Operator.to_char(), 'o');
        
        assert_eq!(UserMode::from_char('a'), Some(UserMode::Away));
        assert_eq!(UserMode::from_char('i'), Some(UserMode::Invisible));
        assert_eq!(UserMode::from_char('o'), Some(UserMode::Operator));
        assert_eq!(UserMode::from_char('x'), None);
    }

    #[test]
    fn test_user_mode_manager() {
        let mut manager = UserModeManager::new();
        
        // Test adding modes
        assert!(manager.add_mode(UserMode::Invisible).is_ok());
        assert!(manager.has_mode(UserMode::Invisible));
        assert_eq!(manager.modes_string(), "i");
        
        // Test adding operator mode
        assert!(manager.add_mode(UserMode::Operator).is_err()); // Should fail without operator status
        
        manager.set_operator(true);
        assert!(manager.add_mode(UserMode::Operator).is_ok());
        assert!(manager.is_operator());
        
        // Test removing modes
        assert!(manager.remove_mode(UserMode::Invisible).is_ok());
        assert!(!manager.has_mode(UserMode::Invisible));
    }

    #[test]
    fn test_mode_validation() {
        let manager = UserModeManager::new();
        
        // Test self-only mode
        assert!(manager.validate_mode_change(
            UserMode::Invisible,
            true,
            "user1",
            "user1",
            false,
        ).is_ok());
        
        assert!(manager.validate_mode_change(
            UserMode::Invisible,
            true,
            "user1",
            "user2",
            false,
        ).is_err());
        
        // Test operator-only mode
        assert!(manager.validate_mode_change(
            UserMode::Operator,
            true,
            "user1",
            "user1",
            false,
        ).is_err());
        
        assert!(manager.validate_mode_change(
            UserMode::Operator,
            true,
            "user1",
            "user1",
            true,
        ).is_ok());
    }
}
