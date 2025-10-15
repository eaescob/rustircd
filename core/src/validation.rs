//! Comprehensive configuration validation system
//!
//! This module provides detailed validation of all configuration settings,
//! including cross-references, file paths, and network configuration.

use crate::Config;
use std::path::Path;
use std::collections::HashSet;

/// Validation result with detailed information
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub is_valid: bool,
    /// List of errors found
    pub errors: Vec<ValidationError>,
    /// List of warnings (non-fatal issues)
    pub warnings: Vec<ValidationWarning>,
    /// List of informational messages
    pub info: Vec<String>,
}

/// Validation error with context
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error category
    pub category: ErrorCategory,
    /// Error message
    pub message: String,
    /// Suggested fix
    pub suggestion: Option<String>,
    /// Configuration section where error occurred
    pub section: String,
}

/// Validation warning (non-fatal)
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,
    /// Configuration section
    pub section: String,
    /// Optional suggestion for improvement
    pub suggestion: Option<String>,
}

/// Error categories for better organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Missing required field
    MissingRequired,
    /// Invalid value
    InvalidValue,
    /// Invalid cross-reference
    InvalidReference,
    /// File not found
    FileNotFound,
    /// Duplicate value
    Duplicate,
    /// Security issue
    Security,
    /// Network configuration
    Network,
    /// Ordering issue
    Ordering,
}

impl ValidationResult {
    /// Create a new successful validation result
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: ValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Add an info message
    pub fn add_info(&mut self, info: String) {
        self.info.push(info);
    }

    /// Merge another validation result
    pub fn merge(&mut self, other: ValidationResult) {
        if !other.is_valid {
            self.is_valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.info.extend(other.info);
    }
}

/// Comprehensive configuration validator
pub struct ConfigValidator {
    config: Config,
}

impl ConfigValidator {
    /// Create a new validator
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Run all validation checks
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Run all validation checks
        result.merge(self.validate_server_section());
        result.merge(self.validate_classes_section());
        result.merge(self.validate_network_section());
        result.merge(self.validate_connection_section());
        result.merge(self.validate_security_section());
        result.merge(self.validate_modules_section());
        result.merge(self.validate_services_section());
        result.merge(self.validate_cross_references());
        result.merge(self.validate_file_paths());
        result.merge(self.validate_security_best_practices());

        result
    }

    /// Validate server section
    fn validate_server_section(&self) -> ValidationResult {
        let mut result = ValidationResult::success();
        let section = "server";

        // Check required fields
        if self.config.server.name.is_empty() {
            result.add_error(ValidationError {
                category: ErrorCategory::MissingRequired,
                message: "Server name cannot be empty".to_string(),
                suggestion: Some("Set server.name = \"your.server.name\"".to_string()),
                section: section.to_string(),
            });
        }

        if self.config.server.description.is_empty() {
            result.add_warning(ValidationWarning {
                message: "Server description is empty".to_string(),
                section: section.to_string(),
                suggestion: Some("Add a descriptive server.description".to_string()),
            });
        }

        // Validate server name format
        if !self.config.server.name.contains('.') && self.config.server.name != "localhost" {
            result.add_warning(ValidationWarning {
                message: "Server name should be a fully qualified domain name".to_string(),
                section: section.to_string(),
                suggestion: Some("Use a format like 'irc.example.com'".to_string()),
            });
        }

        // Validate limits
        if self.config.server.max_clients == 0 {
            result.add_error(ValidationError {
                category: ErrorCategory::InvalidValue,
                message: "max_clients must be greater than 0".to_string(),
                suggestion: Some("Set server.max_clients = 1000 or higher".to_string()),
                section: section.to_string(),
            });
        }

        if self.config.server.max_nickname_length < 9 {
            result.add_warning(ValidationWarning {
                message: format!("max_nickname_length is {} but RFC 1459 recommends at least 9", 
                    self.config.server.max_nickname_length),
                section: section.to_string(),
                suggestion: Some("Set server.max_nickname_length = 9 or higher".to_string()),
            });
        }

        result.add_info(format!("Server: {} (max {} clients)", 
            self.config.server.name, self.config.server.max_clients));

        result
    }

    /// Validate classes section
    fn validate_classes_section(&self) -> ValidationResult {
        let mut result = ValidationResult::success();
        let section = "classes";

        if self.config.classes.is_empty() {
            result.add_error(ValidationError {
                category: ErrorCategory::MissingRequired,
                message: "At least one connection class must be defined".to_string(),
                suggestion: Some("Add a [[classes]] section with name = \"default\"".to_string()),
                section: section.to_string(),
            });
            return result;
        }

        // Check for default class
        if !self.config.classes.iter().any(|c| c.name == "default") {
            result.add_warning(ValidationWarning {
                message: "No 'default' class defined - recommended for fallback".to_string(),
                section: section.to_string(),
                suggestion: Some("Add a class with name = \"default\"".to_string()),
            });
        }

        // Validate each class
        let mut seen_names = HashSet::new();
        for (idx, class) in self.config.classes.iter().enumerate() {
            // Check for duplicate names
            if seen_names.contains(&class.name) {
                result.add_error(ValidationError {
                    category: ErrorCategory::Duplicate,
                    message: format!("Duplicate class name: {}", class.name),
                    suggestion: Some("Each class must have a unique name".to_string()),
                    section: format!("classes[{}]", idx),
                });
            }
            seen_names.insert(class.name.clone());

            // Validate sendq/recvq
            if let Some(sendq) = class.max_sendq {
                if sendq < 1024 {
                    result.add_warning(ValidationWarning {
                        message: format!("Class '{}' has very small sendq: {} bytes", class.name, sendq),
                        section: format!("classes.{}", class.name),
                        suggestion: Some("Consider at least 8192 bytes (8KB) for sendq".to_string()),
                    });
                }
            }

            if let Some(recvq) = class.max_recvq {
                if recvq < 512 {
                    result.add_warning(ValidationWarning {
                        message: format!("Class '{}' has very small recvq: {} bytes", class.name, recvq),
                        section: format!("classes.{}", class.name),
                        suggestion: Some("Consider at least 4096 bytes (4KB) for recvq".to_string()),
                    });
                }
            }

            // Validate timing
            if let Some(ping_freq) = class.ping_frequency {
                if ping_freq < 30 {
                    result.add_warning(ValidationWarning {
                        message: format!("Class '{}' has very frequent ping: {}s", class.name, ping_freq),
                        section: format!("classes.{}", class.name),
                        suggestion: Some("Consider at least 60 seconds to avoid overhead".to_string()),
                    });
                }
            }
        }

        result.add_info(format!("Classes: {} defined ({})", 
            self.config.classes.len(),
            self.config.classes.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ")
        ));

        result
    }

    /// Validate network section
    fn validate_network_section(&self) -> ValidationResult {
        let mut result = ValidationResult::success();
        let section = "network";

        if self.config.network.name.is_empty() {
            result.add_error(ValidationError {
                category: ErrorCategory::MissingRequired,
                message: "Network name cannot be empty".to_string(),
                suggestion: Some("Set network.name = \"YourNetwork\"".to_string()),
                section: section.to_string(),
            });
        }

        // Validate server links
        for (idx, link) in self.config.network.links.iter().enumerate() {
            if link.name.is_empty() {
                result.add_error(ValidationError {
                    category: ErrorCategory::InvalidValue,
                    message: "Server link name cannot be empty".to_string(),
                    suggestion: None,
                    section: format!("network.links[{}]", idx),
                });
            }

            if link.password.is_empty() {
                result.add_error(ValidationError {
                    category: ErrorCategory::Security,
                    message: format!("Server link '{}' has no password", link.name),
                    suggestion: Some("Add a strong password for server authentication".to_string()),
                    section: format!("network.links[{}]", idx),
                });
            }

            // Validate class reference if specified
            if let Some(class_name) = &link.class {
                if !self.config.classes.iter().any(|c| &c.name == class_name) {
                    result.add_error(ValidationError {
                        category: ErrorCategory::InvalidReference,
                        message: format!("Server link '{}' references non-existent class '{}'", link.name, class_name),
                        suggestion: Some(format!("Define [[classes]] with name = \"{}\" before [network] section", class_name)),
                        section: format!("network.links[{}]", idx),
                    });
                }
            }
        }

        // Validate operators
        for (idx, operator) in self.config.network.operators.iter().enumerate() {
            if operator.nickname.is_empty() {
                result.add_error(ValidationError {
                    category: ErrorCategory::InvalidValue,
                    message: "Operator nickname cannot be empty".to_string(),
                    suggestion: None,
                    section: format!("network.operators[{}]", idx),
                });
            }

            if operator.password_hash.len() != 64 {
                result.add_error(ValidationError {
                    category: ErrorCategory::Security,
                    message: format!("Operator '{}' has invalid password hash (expected 64 hex chars)", operator.nickname),
                    suggestion: Some("Generate with: echo -n 'password' | sha256sum".to_string()),
                    section: format!("network.operators[{}]", idx),
                });
            }

            if operator.hostmask == "*@*" {
                result.add_warning(ValidationWarning {
                    message: format!("Operator '{}' allows connections from any host", operator.nickname),
                    section: format!("network.operators[{}]", idx),
                    suggestion: Some("Consider restricting with a specific hostmask pattern".to_string()),
                });
            }
        }

        result.add_info(format!("Network: {} ({} links, {} operators)", 
            self.config.network.name,
            self.config.network.links.len(),
            self.config.network.operators.len()
        ));

        result
    }

    /// Validate connection section
    fn validate_connection_section(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if self.config.connection.ports.is_empty() {
            result.add_error(ValidationError {
                category: ErrorCategory::MissingRequired,
                message: "No ports configured - server cannot accept connections".to_string(),
                suggestion: Some("Add at least one [[connection.ports]] section".to_string()),
                section: "connection".to_string(),
            });
            return result;
        }

        // Check for at least one client port
        let has_client_port = self.config.connection.ports.iter().any(|p| {
            matches!(p.connection_type, crate::config::PortConnectionType::Client | crate::config::PortConnectionType::Both)
        });

        if !has_client_port {
            result.add_warning(ValidationWarning {
                message: "No client ports configured - only servers can connect".to_string(),
                section: "connection".to_string(),
                suggestion: Some("Add a client port (typically 6667 or 6697 for TLS)".to_string()),
            });
        }

        // Validate bind addresses
        for (idx, port) in self.config.connection.ports.iter().enumerate() {
            let bind_addr = self.config.get_bind_address_for_port(port);
            
            // Validate IP address format (basic check)
            if !self.is_valid_bind_address(&bind_addr) {
                result.add_error(ValidationError {
                    category: ErrorCategory::InvalidValue,
                    message: format!("Port {} has invalid bind address: {}", port.port, bind_addr),
                    suggestion: Some("Use a valid IP address (e.g., 0.0.0.0, 127.0.0.1, ::)".to_string()),
                    section: format!("connection.ports[{}]", idx),
                });
            }

            // Check TLS configuration
            if port.tls && !self.config.security.tls.enabled {
                result.add_error(ValidationError {
                    category: ErrorCategory::InvalidReference,
                    message: format!("Port {} configured for TLS but TLS is not enabled globally", port.port),
                    suggestion: Some("Set security.tls.enabled = true and configure cert_file/key_file".to_string()),
                    section: format!("connection.ports[{}]", idx),
                });
            }
        }

        result.add_info(format!("Ports: {} configured", self.config.connection.ports.len()));

        result
    }

    /// Validate security section
    fn validate_security_section(&self) -> ValidationResult {
        let mut result = ValidationResult::success();
        let section = "security";

        // Validate allow blocks
        for (idx, allow_block) in self.config.security.allow_blocks.iter().enumerate() {
            // Check class reference
            if !self.config.classes.iter().any(|c| c.name == allow_block.class) {
                result.add_error(ValidationError {
                    category: ErrorCategory::InvalidReference,
                    message: format!("Allow block {} references non-existent class '{}'", idx, allow_block.class),
                    suggestion: Some(format!("Define [[classes]] with name = \"{}\" before [security] section", allow_block.class)),
                    section: format!("security.allow_blocks[{}]", idx),
                });
            }

            // Check that at least one pattern is specified
            if allow_block.hosts.is_empty() && allow_block.ips.is_empty() {
                result.add_error(ValidationError {
                    category: ErrorCategory::InvalidValue,
                    message: format!("Allow block {} has no hosts or IPs defined", idx),
                    suggestion: Some("Add hosts = [\"*\"] or ips = [\"*\"] or both".to_string()),
                    section: format!("security.allow_blocks[{}]", idx),
                });
            }

            // Warn about overly permissive blocks
            if allow_block.hosts.contains(&"*".to_string()) && allow_block.ips.contains(&"*".to_string()) {
                result.add_warning(ValidationWarning {
                    message: format!("Allow block {} allows all hosts and all IPs", idx),
                    section: format!("security.allow_blocks[{}]", idx),
                    suggestion: Some("Consider restricting to specific hosts or IP ranges".to_string()),
                });
            }
        }

        // TLS validation
        if self.config.security.tls.enabled {
            if self.config.security.tls.cert_file.is_none() {
                result.add_error(ValidationError {
                    category: ErrorCategory::MissingRequired,
                    message: "TLS enabled but no certificate file specified".to_string(),
                    suggestion: Some("Set security.tls.cert_file = \"path/to/cert.pem\"".to_string()),
                    section: "security.tls".to_string(),
                });
            }

            if self.config.security.tls.key_file.is_none() {
                result.add_error(ValidationError {
                    category: ErrorCategory::MissingRequired,
                    message: "TLS enabled but no key file specified".to_string(),
                    suggestion: Some("Set security.tls.key_file = \"path/to/key.pem\"".to_string()),
                    section: "security.tls".to_string(),
                });
            }
        }

        // Check for overly permissive security
        if self.config.security.allowed_hosts.contains(&"*".to_string()) && 
           self.config.security.allow_blocks.is_empty() {
            result.add_warning(ValidationWarning {
                message: "All hosts are allowed without class-based restrictions".to_string(),
                section: section.to_string(),
                suggestion: Some("Consider using allow_blocks for better control".to_string()),
            });
        }

        result.add_info(format!("Security: {} allow blocks, TLS {}", 
            self.config.security.allow_blocks.len(),
            if self.config.security.tls.enabled { "enabled" } else { "disabled" }
        ));

        result
    }

    /// Validate modules section
    fn validate_modules_section(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if self.config.modules.enabled_modules.is_empty() {
            result.add_warning(ValidationWarning {
                message: "No modules enabled - server will have minimal functionality".to_string(),
                section: "modules".to_string(),
                suggestion: Some("Consider enabling 'channel', 'ircv3', and 'optional' modules".to_string()),
            });
        }

        // Recommend channel module
        if !self.config.modules.enabled_modules.contains(&"channel".to_string()) {
            result.add_warning(ValidationWarning {
                message: "Channel module not enabled - users cannot join channels".to_string(),
                section: "modules".to_string(),
                suggestion: Some("Add 'channel' to enabled_modules".to_string()),
            });
        }

        result.add_info(format!("Modules: {} enabled", self.config.modules.enabled_modules.len()));

        result
    }

    /// Validate services section
    fn validate_services_section(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Validate service definitions
        for (idx, service) in self.config.services.services.iter().enumerate() {
            if service.enabled {
                // Check if service type is in enabled_services
                if !self.config.services.enabled_services.contains(&service.service_type) {
                    result.add_warning(ValidationWarning {
                        message: format!("Service '{}' is enabled but type '{}' is not in enabled_services", 
                            service.name, service.service_type),
                        section: format!("services.services[{}]", idx),
                        suggestion: Some(format!("Add '{}' to services.enabled_services", service.service_type)),
                    });
                }
            }
        }

        result
    }

    /// Validate cross-references between sections
    fn validate_cross_references(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Check that server links reference valid classes
        for link in &self.config.network.links {
            if let Some(class_name) = &link.class {
                if let Some(class) = self.config.get_class(class_name) {
                    result.add_info(format!("Server link '{}' → class '{}' (sendq: {})", 
                        link.name, 
                        class_name,
                        class.max_sendq.map(|s| format!("{}MB", s / 1048576)).unwrap_or_else(|| "default".to_string())
                    ));
                }
            }
        }

        // Check that allow blocks reference valid classes
        for (idx, block) in self.config.security.allow_blocks.iter().enumerate() {
            if let Some(class) = self.config.get_class(&block.class) {
                result.add_info(format!("Allow block {} → class '{}' (max_clients: {})", 
                    idx,
                    block.class,
                    class.max_clients.map(|c| c.to_string()).unwrap_or_else(|| "unlimited".to_string())
                ));
            }
        }

        result
    }

    /// Validate file paths
    fn validate_file_paths(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Check MOTD file
        if let Some(motd_file) = &self.config.server.motd_file {
            if !Path::new(motd_file).exists() {
                result.add_warning(ValidationWarning {
                    message: format!("MOTD file not found: {}", motd_file),
                    section: "server".to_string(),
                    suggestion: Some("Create the MOTD file or set motd_file to an existing file".to_string()),
                });
            }
        }

        // Check TLS certificate files
        if self.config.security.tls.enabled {
            if let Some(cert_file) = &self.config.security.tls.cert_file {
                if !Path::new(cert_file).exists() {
                    result.add_error(ValidationError {
                        category: ErrorCategory::FileNotFound,
                        message: format!("TLS certificate file not found: {}", cert_file),
                        suggestion: Some("Generate a certificate or provide the correct path".to_string()),
                        section: "security.tls".to_string(),
                    });
                }
            }

            if let Some(key_file) = &self.config.security.tls.key_file {
                if !Path::new(key_file).exists() {
                    result.add_error(ValidationError {
                        category: ErrorCategory::FileNotFound,
                        message: format!("TLS key file not found: {}", key_file),
                        suggestion: Some("Generate a key file or provide the correct path".to_string()),
                        section: "security.tls".to_string(),
                    });
                }
            }
        }

        result
    }

    /// Validate security best practices
    fn validate_security_best_practices(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Check for TLS on client ports
        let has_tls_client_port = self.config.connection.ports.iter().any(|p| {
            p.tls && matches!(p.connection_type, crate::config::PortConnectionType::Client)
        });

        if !has_tls_client_port && self.config.security.tls.enabled {
            result.add_warning(ValidationWarning {
                message: "TLS is enabled but no TLS client port configured".to_string(),
                section: "connection.ports".to_string(),
                suggestion: Some("Add a port with tls = true and connection_type = \"Client\" (typically 6697)".to_string()),
            });
        }

        // Check for ident/DNS
        if !self.config.security.enable_ident && !self.config.security.enable_dns {
            result.add_warning(ValidationWarning {
                message: "Both ident and DNS lookups are disabled".to_string(),
                section: "security".to_string(),
                suggestion: Some("Enable at least one for better user identification".to_string()),
            });
        }

        // Check throttling
        if !self.config.modules.throttling.enabled {
            result.add_warning(ValidationWarning {
                message: "Connection throttling is disabled".to_string(),
                section: "modules.throttling".to_string(),
                suggestion: Some("Enable throttling to protect against connection floods".to_string()),
            });
        }

        result
    }

    /// Check if a bind address is valid
    fn is_valid_bind_address(&self, addr: &str) -> bool {
        // Basic validation - could be enhanced
        addr == "0.0.0.0" || 
        addr == "127.0.0.1" || 
        addr == "::" || 
        addr == "::1" ||
        addr.parse::<std::net::IpAddr>().is_ok()
    }
}

/// Pretty print validation results
pub fn print_validation_result(result: &ValidationResult) {
    println!("\n{}", "=".repeat(80));
    println!("Configuration Validation Report");
    println!("{}", "=".repeat(80));

    if result.is_valid {
        println!("\n✓ Configuration is VALID\n");
    } else {
        println!("\n✗ Configuration has ERRORS\n");
    }

    // Print errors
    if !result.errors.is_empty() {
        println!("ERRORS ({}):", result.errors.len());
        println!("{}", "-".repeat(80));
        for (idx, error) in result.errors.iter().enumerate() {
            println!("{}. [{:?}] {} - {}", 
                idx + 1,
                error.category,
                error.section,
                error.message
            );
            if let Some(suggestion) = &error.suggestion {
                println!("   → Suggestion: {}", suggestion);
            }
            println!();
        }
    }

    // Print warnings
    if !result.warnings.is_empty() {
        println!("WARNINGS ({}):", result.warnings.len());
        println!("{}", "-".repeat(80));
        for (idx, warning) in result.warnings.iter().enumerate() {
            println!("{}. {} - {}", 
                idx + 1,
                warning.section,
                warning.message
            );
            if let Some(suggestion) = &warning.suggestion {
                println!("   → Suggestion: {}", suggestion);
            }
            println!();
        }
    }

    // Print info
    if !result.info.is_empty() {
        println!("INFORMATION:");
        println!("{}", "-".repeat(80));
        for info in &result.info {
            println!("  • {}", info);
        }
        println!();
    }

    println!("{}", "=".repeat(80));

    if result.is_valid {
        if result.warnings.is_empty() {
            println!("✓ Perfect! Configuration is valid with no warnings.");
        } else {
            println!("✓ Configuration is valid but has {} warning(s) to review.", result.warnings.len());
        }
    } else {
        println!("✗ Configuration has {} error(s) that must be fixed.", result.errors.len());
    }

    println!("{}", "=".repeat(80));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[test]
    fn test_valid_config() {
        let config = Config::default();
        let validator = ConfigValidator::new(config);
        let result = validator.validate();
        
        assert!(result.is_valid);
    }

    #[test]
    fn test_missing_class_reference() {
        let mut config = Config::default();
        
        // Add a server link with non-existent class
        config.network.links.push(ServerLink {
            name: "test.server".to_string(),
            hostname: "test.server".to_string(),
            port: 6668,
            password: "password123".to_string(),
            tls: false,
            outgoing: true,
            class: Some("nonexistent".to_string()),
        });
        
        let validator = ConfigValidator::new(config);
        let result = validator.validate();
        
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| matches!(e.category, ErrorCategory::InvalidReference)));
    }

    #[test]
    fn test_duplicate_class_names() {
        let mut config = Config::default();
        
        // Add duplicate class
        config.classes.push(ConnectionClass {
            name: "default".to_string(),
            ..Default::default()
        });
        
        let validator = ConfigValidator::new(config);
        let result = validator.validate();
        
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| matches!(e.category, ErrorCategory::Duplicate)));
    }
}

