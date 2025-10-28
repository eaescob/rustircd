//! Security audit logging module
//!
//! This module provides comprehensive security event logging for authentication,
//! authorization, and operator actions as recommended by the security audit.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Security audit event types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Authentication events
    AuthSuccess,
    AuthFailure,
    AuthChallenge,

    /// Authorization events
    AuthzSuccess,
    AuthzFailure,

    /// Operator events
    OperAuth,
    OperAuthFailure,
    OperAction,
    OperPrivilegeGrant,
    OperPrivilegeRevoke,

    /// Configuration events
    ConfigChange,
    ConfigReload,

    /// Connection events
    ConnectionThrottle,
    ConnectionBan,

    /// Server events
    ServerConnect,
    ServerDisconnect,
    ServerSquit,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthSuccess => write!(f, "auth_success"),
            Self::AuthFailure => write!(f, "auth_failure"),
            Self::AuthChallenge => write!(f, "auth_challenge"),
            Self::AuthzSuccess => write!(f, "authz_success"),
            Self::AuthzFailure => write!(f, "authz_failure"),
            Self::OperAuth => write!(f, "oper_auth"),
            Self::OperAuthFailure => write!(f, "oper_auth_failure"),
            Self::OperAction => write!(f, "oper_action"),
            Self::OperPrivilegeGrant => write!(f, "oper_privilege_grant"),
            Self::OperPrivilegeRevoke => write!(f, "oper_privilege_revoke"),
            Self::ConfigChange => write!(f, "config_change"),
            Self::ConfigReload => write!(f, "config_reload"),
            Self::ConnectionThrottle => write!(f, "connection_throttle"),
            Self::ConnectionBan => write!(f, "connection_ban"),
            Self::ServerConnect => write!(f, "server_connect"),
            Self::ServerDisconnect => write!(f, "server_disconnect"),
            Self::ServerSquit => write!(f, "server_squit"),
        }
    }
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event type
    pub event_type: AuditEventType,

    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// User involved (nickname)
    pub user: Option<String>,

    /// User ID (UUID)
    pub user_id: Option<Uuid>,

    /// Username
    pub username: Option<String>,

    /// Hostname
    pub hostname: Option<String>,

    /// IP address
    pub ip: Option<String>,

    /// Authentication method (e.g., "SASL_PLAIN", "OPER", "SERVICES")
    pub method: Option<String>,

    /// Command executed
    pub command: Option<String>,

    /// Target of the action (e.g., server name, channel name, target user)
    pub target: Option<String>,

    /// Required flag or permission
    pub required_flag: Option<String>,

    /// Reason or additional details
    pub reason: Option<String>,

    /// Error message (for failures)
    pub error: Option<String>,

    /// Whether the connection was secure (TLS)
    pub secure: Option<bool>,

    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            event_type,
            timestamp: chrono::Utc::now(),
            user: None,
            user_id: None,
            username: None,
            hostname: None,
            ip: None,
            method: None,
            command: None,
            target: None,
            required_flag: None,
            reason: None,
            error: None,
            secure: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set user information
    pub fn with_user(mut self, nick: impl Into<String>) -> Self {
        self.user = Some(nick.into());
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, id: Uuid) -> Self {
        self.user_id = Some(id);
        self
    }

    /// Set username
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set hostname
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    /// Set IP address
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip = Some(ip.into());
        self
    }

    /// Set authentication method
    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Set command
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Set target
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Set required flag
    pub fn with_required_flag(mut self, flag: impl Into<String>) -> Self {
        self.required_flag = Some(flag.into());
        self
    }

    /// Set reason
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set error
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set secure flag
    pub fn with_secure(mut self, secure: bool) -> Self {
        self.secure = Some(secure);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Log this event
    pub fn log(&self) {
        match self.event_type {
            AuditEventType::AuthFailure
            | AuditEventType::OperAuthFailure
            | AuditEventType::AuthzFailure
            | AuditEventType::ConnectionThrottle
            | AuditEventType::ConnectionBan => {
                // Log security-relevant failures as warnings
                tracing::warn!(
                    event = %self.event_type,
                    user = ?self.user,
                    user_id = ?self.user_id,
                    username = ?self.username,
                    hostname = ?self.hostname,
                    ip = ?self.ip,
                    method = ?self.method,
                    command = ?self.command,
                    target = ?self.target,
                    required_flag = ?self.required_flag,
                    reason = ?self.reason,
                    error = ?self.error,
                    secure = ?self.secure,
                    metadata = ?self.metadata,
                    timestamp = %self.timestamp.to_rfc3339(),
                    "Security event"
                );
            }
            AuditEventType::OperAction
            | AuditEventType::OperAuth
            | AuditEventType::OperPrivilegeGrant
            | AuditEventType::OperPrivilegeRevoke
            | AuditEventType::ServerSquit
            | AuditEventType::ConfigChange
            | AuditEventType::ConfigReload => {
                // Log operator actions and sensitive events as info
                tracing::info!(
                    event = %self.event_type,
                    user = ?self.user,
                    user_id = ?self.user_id,
                    username = ?self.username,
                    hostname = ?self.hostname,
                    ip = ?self.ip,
                    method = ?self.method,
                    command = ?self.command,
                    target = ?self.target,
                    required_flag = ?self.required_flag,
                    reason = ?self.reason,
                    secure = ?self.secure,
                    metadata = ?self.metadata,
                    timestamp = %self.timestamp.to_rfc3339(),
                    "Security event"
                );
            }
            _ => {
                // Log other events as debug
                tracing::debug!(
                    event = %self.event_type,
                    user = ?self.user,
                    user_id = ?self.user_id,
                    username = ?self.username,
                    hostname = ?self.hostname,
                    ip = ?self.ip,
                    method = ?self.method,
                    command = ?self.command,
                    target = ?self.target,
                    required_flag = ?self.required_flag,
                    reason = ?self.reason,
                    secure = ?self.secure,
                    metadata = ?self.metadata,
                    timestamp = %self.timestamp.to_rfc3339(),
                    "Security event"
                );
            }
        }
    }
}

/// Audit logger for security events
#[derive(Debug, Clone)]
pub struct AuditLogger {
    /// Whether audit logging is enabled
    enabled: bool,

    /// Minimum level for audit events (0 = all, 1 = info+, 2 = warn+)
    min_level: u8,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(enabled: bool, min_level: u8) -> Self {
        Self {
            enabled,
            min_level,
        }
    }

    /// Log an audit event
    pub fn log(&self, event: &AuditEvent) {
        if !self.enabled {
            return;
        }

        // Filter by level
        let should_log = match event.event_type {
            AuditEventType::AuthFailure
            | AuditEventType::OperAuthFailure
            | AuditEventType::AuthzFailure
            | AuditEventType::ConnectionThrottle
            | AuditEventType::ConnectionBan => {
                // Warning level events
                self.min_level <= 2
            }
            AuditEventType::OperAction
            | AuditEventType::OperAuth
            | AuditEventType::OperPrivilegeGrant
            | AuditEventType::OperPrivilegeRevoke
            | AuditEventType::ServerSquit
            | AuditEventType::ConfigChange
            | AuditEventType::ConfigReload => {
                // Info level events
                self.min_level <= 1
            }
            _ => {
                // Debug level events
                self.min_level == 0
            }
        };

        if should_log {
            event.log();
        }
    }

    /// Check if audit logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set enabled state
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get minimum log level
    pub fn min_level(&self) -> u8 {
        self.min_level
    }

    /// Set minimum log level
    pub fn set_min_level(&mut self, level: u8) {
        self.min_level = level;
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(true, 0)
    }
}

/// Helper macros for creating audit events
#[macro_export]
macro_rules! audit_auth_success {
    ($user:expr, $method:expr) => {
        $crate::audit::AuditEvent::new($crate::audit::AuditEventType::AuthSuccess)
            .with_user($user)
            .with_method($method)
    };
}

#[macro_export]
macro_rules! audit_auth_failure {
    ($user:expr, $method:expr, $error:expr) => {
        $crate::audit::AuditEvent::new($crate::audit::AuditEventType::AuthFailure)
            .with_user($user)
            .with_method($method)
            .with_error($error)
    };
}

#[macro_export]
macro_rules! audit_authz_failure {
    ($user:expr, $command:expr, $flag:expr) => {
        $crate::audit::AuditEvent::new($crate::audit::AuditEventType::AuthzFailure)
            .with_user($user)
            .with_command($command)
            .with_required_flag($flag)
    };
}

#[macro_export]
macro_rules! audit_oper_action {
    ($user:expr, $command:expr, $target:expr) => {
        $crate::audit::AuditEvent::new($crate::audit::AuditEventType::OperAction)
            .with_user($user)
            .with_command($command)
            .with_target($target)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditEventType::AuthSuccess)
            .with_user("alice")
            .with_method("SASL_PLAIN")
            .with_ip("192.0.2.1");

        assert_eq!(event.event_type, AuditEventType::AuthSuccess);
        assert_eq!(event.user, Some("alice".to_string()));
        assert_eq!(event.method, Some("SASL_PLAIN".to_string()));
        assert_eq!(event.ip, Some("192.0.2.1".to_string()));
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::new(true, 1);
        assert!(logger.is_enabled());
        assert_eq!(logger.min_level(), 1);
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(AuditEventType::AuthSuccess.to_string(), "auth_success");
        assert_eq!(AuditEventType::OperAction.to_string(), "oper_action");
        assert_eq!(AuditEventType::AuthzFailure.to_string(), "authz_failure");
    }
}
