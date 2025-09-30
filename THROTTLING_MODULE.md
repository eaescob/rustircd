# Throttling Module for RustIRCD

The throttling module provides configurable connection rate limiting for RustIRCD, helping protect against connection flooding and abuse by implementing IP-based throttling with multiple stages.

## Features

- **IP-based throttling**: Tracks connection attempts per IP address
- **Configurable limits**: Set maximum connections per IP within a time window
- **Multi-stage throttling**: Progressive throttling with increasing durations
- **Automatic cleanup**: Removes expired throttle entries automatically
- **Configurable parameters**: All throttling behavior can be customized
- **In-memory tracking**: Fast lookups with automatic memory management

## Configuration

### Basic Configuration

Add the throttling module to your enabled modules list:

```toml
[modules]
enabled_modules = ["throttling", "channel", "ircv3"]
```

### Throttling Settings

Configure throttling behavior in the `modules.throttling` section:

```toml
[modules.throttling]
# Enable or disable throttling
enabled = true

# Maximum connections allowed per IP within the time window
max_connections_per_ip = 5

# Time window in seconds for connection counting
time_window_seconds = 60

# Initial throttling duration in seconds (stage 1)
initial_throttle_seconds = 10

# Maximum number of throttling stages
max_stages = 10

# Factor by which throttling increases between stages
stage_factor = 10

# Cleanup interval in seconds for expired throttle entries
cleanup_interval_seconds = 300
```

### Configuration Examples

#### Conservative Throttling (Strict)
```toml
[modules.throttling]
enabled = true
max_connections_per_ip = 2
time_window_seconds = 120
initial_throttle_seconds = 30
max_stages = 5
stage_factor = 5
cleanup_interval_seconds = 300
```

#### Relaxed Throttling (Permissive)
```toml
[modules.throttling]
enabled = true
max_connections_per_ip = 10
time_window_seconds = 30
initial_throttle_seconds = 5
max_stages = 15
stage_factor = 15
cleanup_interval_seconds = 300
```

#### Aggressive Throttling (Very Strict)
```toml
[modules.throttling]
enabled = true
max_connections_per_ip = 1
time_window_seconds = 300
initial_throttle_seconds = 60
max_stages = 8
stage_factor = 8
cleanup_interval_seconds = 300
```

## How It Works

### Connection Tracking

1. **Connection Counting**: Each connection attempt from an IP is recorded with a timestamp
2. **Time Window**: Only connections within the configured time window are counted
3. **Limit Enforcement**: When the connection limit is exceeded, throttling is applied

### Throttling Stages

The throttling system uses progressive stages with increasing durations:

1. **Stage 1**: Initial throttle duration (`initial_throttle_seconds`)
2. **Stage 2**: `initial_throttle_seconds * stage_factor`
3. **Stage 3**: `initial_throttle_seconds * stage_factor^2`
4. **Stage N**: `initial_throttle_seconds * stage_factor^(N-1)`

Maximum stage is capped at `max_stages`.

### Example Throttling Progression

With default settings:
- **Stage 1**: 10 seconds
- **Stage 2**: 100 seconds (10 × 10)
- **Stage 3**: 1000 seconds (10 × 10²)
- **Stage 4**: 10000 seconds (10 × 10³)
- And so on...

### Automatic Cleanup

The module automatically cleans up expired throttle entries every `cleanup_interval_seconds` to prevent memory leaks.

## Usage Examples

### Basic Server with Throttling

```rust
use rustircd_core::{Config, Server, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = Config::default();
    
    // Enable throttling module
    config.modules.enabled_modules.push("throttling".to_string());
    
    // Configure throttling
    config.modules.throttling.enabled = true;
    config.modules.throttling.max_connections_per_ip = 5;
    config.modules.throttling.time_window_seconds = 60;
    config.modules.throttling.initial_throttle_seconds = 10;
    
    // Create and start server
    let mut server = Server::new(config);
    server.init().await?;
    server.start().await?;
    
    Ok(())
}
```

### Testing Throttling

```rust
use rustircd_core::ThrottlingManager;
use std::net::IpAddr;

#[tokio::main]
async fn main() -> Result<()> {
    let config = rustircd_core::config::ThrottlingConfig::default();
    let manager = ThrottlingManager::new(config);
    manager.init().await?;
    
    let test_ip: IpAddr = "192.168.1.100".parse().unwrap();
    
    // Test connection attempts
    for i in 1..=6 {
        let allowed = manager.check_connection_allowed(test_ip).await?;
        let (is_throttled, stage, remaining) = manager.get_throttle_status(test_ip).await;
        
        println!("Attempt {}: {} (stage: {}, remaining: {}s)", 
                 i, 
                 if allowed { "ALLOWED" } else { "BLOCKED" },
                 stage, 
                 remaining);
    }
    
    Ok(())
}
```

## API Reference

### ThrottlingManager

The `ThrottlingManager` provides the core throttling functionality:

```rust
impl ThrottlingManager {
    /// Create a new throttling manager with configuration
    pub fn new(config: ThrottlingConfig) -> Self;
    
    /// Initialize the manager (starts cleanup task)
    pub async fn init(&self) -> Result<()>;
    
    /// Check if an IP address is allowed to connect
    pub async fn check_connection_allowed(&self, ip_addr: IpAddr) -> Result<bool>;
    
    /// Get throttling status for an IP address
    /// Returns: (is_throttled, stage, remaining_seconds)
    pub async fn get_throttle_status(&self, ip_addr: IpAddr) -> (bool, u8, u64);
}
```

### ThrottlingConfig

Configuration structure for throttling behavior:

```rust
pub struct ThrottlingConfig {
    pub enabled: bool,
    pub max_connections_per_ip: usize,
    pub time_window_seconds: u64,
    pub initial_throttle_seconds: u64,
    pub max_stages: u8,
    pub stage_factor: u64,
    pub cleanup_interval_seconds: u64,
}
```

## Module Integration

The throttling module integrates with the RustIRCD server at the connection level:

1. **Connection Acceptance**: Throttling is checked before accepting new client connections
2. **Server Connections**: Only applies to client connections, not server-to-server connections
3. **Module System**: Registered as a standard RustIRCD module
4. **Configuration**: Uses the standard module configuration system

## Performance Considerations

- **Memory Usage**: In-memory storage with automatic cleanup
- **Lookup Performance**: O(1) hash map lookups for IP addresses
- **Cleanup Overhead**: Periodic cleanup task runs every 5 minutes by default
- **Concurrency**: Thread-safe with async/await support

## Security Features

- **IP-based Tracking**: Prevents abuse from single IP addresses
- **Progressive Penalties**: Repeated violations result in longer throttles
- **Automatic Reset**: Throttling expires automatically
- **Configurable Limits**: Adjustable for different security requirements

## Troubleshooting

### Common Issues

1. **Too Strict Throttling**: Reduce `max_connections_per_ip` or increase `time_window_seconds`
2. **Too Permissive**: Increase `max_connections_per_ip` or decrease `time_window_seconds`
3. **Memory Usage**: Ensure `cleanup_interval_seconds` is reasonable (300-600 seconds)

### Debugging

Enable debug logging to see throttling decisions:

```toml
# In your logging configuration
[tracing]
level = "debug"
```

This will show throttling decisions in the server logs.

## Example Configuration Files

See the following files for complete examples:

- `examples/configs/throttling.toml` - Complete server configuration with throttling
- `examples/throttling_example.rs` - Programmatic example and testing
- `config_enhanced.toml` - Enhanced configuration with all modules

## Contributing

To extend the throttling module:

1. Add new configuration options to `ThrottlingConfig`
2. Implement new throttling logic in `ThrottlingManager`
3. Update the module interface in `modules/src/throttling.rs`
4. Add tests for new functionality
5. Update documentation and examples

## License

This throttling module is part of RustIRCD and follows the same license terms.
