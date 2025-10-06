# Rehash System for RustIRCD

The rehash system provides runtime configuration reloading capabilities for the Rust IRC Daemon, allowing operators to update server configuration without restarting the server.

## Overview

The rehash system consists of:
- **RehashService**: Core service for configuration reloading
- **REHASH Command**: IRC command for operators to trigger reloads
- **LOCops Integration**: Rehash functionality through LOCops commands
- **Module Support**: Extensible system for different configuration types

## Features

### 1. Main Configuration Reload (`REHASH`)
Reloads the main configuration file (`config.toml`) including:
- Server settings (name, description, version)
- Network settings (operators, super servers)
- Connection settings (ports, bind address)
- Security settings (TLS, authentication)
- Module settings (enabled modules, configuration)
- Database settings
- Broadcasting settings
- Services settings

### 2. SSL/TLS Reload (`REHASH SSL`)
Reloads TLS/SSL settings including:
- Certificate files
- Private key files
- Cipher suites
- TLS version settings
- CA certificate files

### 3. MOTD Reload (`REHASH MOTD`)
Reloads the Message of the Day file so users see updated messages when they use the `/MOTD` command.

### 4. Modules Reload (`REHASH MODULES`)
Reloads module configuration and settings without restarting the server.

## Usage

### IRC Commands

As an operator, you can use these commands:

```
/REHASH           - Reload main configuration
/REHASH SSL       - Reload TLS settings
/REHASH MOTD      - Reload MOTD file
/REHASH MODULES   - Reload all modules
```

### LOCops Commands

You can also use rehash through the LOCops system:

```
/LOCops REHASH           - Reload main configuration
/LOCops REHASH SSL       - Reload TLS settings
/LOCops REHASH MOTD      - Reload MOTD file
/LOCops REHASH MODULES   - Reload all modules
```

## Implementation Details

### RehashService

The `RehashService` is the core component that handles configuration reloading:

```rust
pub struct RehashService {
    config: Arc<RwLock<Config>>,
    motd_manager: Arc<MotdManager>,
    config_path: String,
}
```

#### Key Methods

- `reload_main_config()` - Reloads the main configuration file
- `reload_ssl()` - Reloads SSL/TLS settings
- `reload_motd()` - Reloads MOTD file
- `reload_modules()` - Reloads all modules
- `reload_section(section)` - Reloads a specific configuration section

### Module Integration

The rehash system is integrated with the module system through:

1. **Module Trait Extension**: Added `handle_message_with_server()` method
2. **ModuleManager Enhancement**: Added `handle_message_with_server()` method
3. **Server Integration**: Updated to pass server reference to modules

### Admin Module

The admin module provides the REHASH command implementation:

```rust
async fn handle_rehash(
    &self, 
    client: &Client, 
    user: &User, 
    args: &[String], 
    server: Option<&Server>
) -> Result<()>
```

## Configuration

### Prerequisites

1. **Operator Privileges**: Only operators can use rehash commands
2. **Valid Configuration**: Configuration files must be valid TOML
3. **File Permissions**: Server must have read access to configuration files

### File Structure

```
config.toml          - Main configuration file
replies.toml         - Custom replies configuration (optional)
motd.txt            - Message of the Day file (optional)
certs/              - TLS certificate directory
├── server.crt      - Server certificate
└── server.key      - Private key
```

## Error Handling

The rehash system provides comprehensive error handling:

- **Permission Errors**: Non-operators receive permission denied messages
- **File Errors**: Missing or invalid configuration files are reported
- **Validation Errors**: Invalid configuration syntax is caught and reported
- **TLS Errors**: Certificate and key file validation errors are reported

## Security Considerations

1. **Operator-Only Access**: Only operators can execute rehash commands
2. **Configuration Validation**: All configurations are validated before applying
3. **File Path Validation**: Absolute and relative paths are properly resolved
4. **TLS Security**: Certificate and key files are validated before use

## Examples

### Basic Usage

```rust
use rustircd_core::{Config, Server, RehashService};

// Create server
let mut server = Server::new(config).await;
server.init().await?;

// Access rehash service
let rehash_service = server.rehash_service();

// Reload main configuration
rehash_service.reload_main_config().await?;

// Reload MOTD
rehash_service.reload_motd().await?;

// Reload SSL settings
rehash_service.reload_ssl().await?;

// Reload modules
rehash_service.reload_modules().await?;
```

### IRC Client Usage

```
/OPER admin secret
:admin!admin@localhost OPER admin
:server 381 admin :You are now an IRC operator

/REHASH
:server 999 admin :REHASH: Reloading main configuration...
:server 999 admin :REHASH: Main configuration reloaded successfully

/REHASH MOTD
:server 999 admin :REHASH MOTD: Reloading MOTD file...
:server 999 admin :REHASH MOTD: MOTD file reloaded successfully
```

## Testing

The rehash system includes comprehensive tests:

```rust
#[tokio::test]
async fn test_rehash_service_creation() {
    let config = Config::default();
    let config_arc = Arc::new(RwLock::new(config));
    let motd_manager = Arc::new(MotdManager::new());
    let rehash_service = RehashService::new(
        config_arc,
        motd_manager,
        "test_config.toml".to_string(),
    );
    
    let info = rehash_service.get_config_info().await;
    assert!(info.contains("rustircd"));
}
```

## Future Enhancements

1. **Hot Module Loading**: Dynamic module loading/unloading
2. **Configuration Validation**: Enhanced validation with detailed error messages
3. **Rollback Support**: Ability to rollback configuration changes
4. **Audit Logging**: Detailed logging of rehash operations
5. **Remote Rehash**: Rehash commands from remote servers
6. **Configuration Templates**: Predefined configuration templates

## Troubleshooting

### Common Issues

1. **Permission Denied**: Ensure you have operator privileges
2. **File Not Found**: Check file paths and permissions
3. **Invalid Configuration**: Validate TOML syntax
4. **TLS Errors**: Verify certificate and key files exist and are valid

### Debug Information

Enable debug logging to see detailed rehash operations:

```toml
[logging]
level = "debug"
```

### Configuration Validation

Test configuration before applying:

```bash
rustircd --config config.toml --test-config
```

## Conclusion

The rehash system provides a powerful and flexible way to manage server configuration at runtime. It integrates seamlessly with the existing module system and provides comprehensive error handling and security features.

For more information, see the [examples/rehash_example.rs](examples/rehash_example.rs) file and the [core/src/rehash.rs](core/src/rehash.rs) implementation.
