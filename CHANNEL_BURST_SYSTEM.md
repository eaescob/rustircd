# Channel Burst System for RustIRCD

The Channel Burst system provides server-to-server synchronization of channel information, enabling distributed IRC networks to maintain consistent channel state across multiple servers.

## Overview

The Channel Burst system allows servers in a network to:
- Synchronize channel information (topics, modes, metadata)
- Share channel creation and modification events
- Maintain consistent channel state across the network
- Handle server connections and disconnections gracefully

## Architecture

### Core Components

1. **ChannelBurstExtension** - Module-specific burst handler
2. **ExtensionManager** - Manages burst extensions and message routing
3. **BurstType::Channel** - Channel-specific burst type
4. **MessageType::ChannelBurst** - Channel burst message format

### Integration Points

- **Module Layer**: Channel module implements `BurstExtension` trait
- **Server Layer**: Server provides burst preparation and processing methods
- **Network Layer**: Extension manager routes burst messages to registered extensions

## Channel Burst Message Format

### Message Structure
```
CBURST <channel> <channel_id> <server> <created_timestamp> [parameters...]
```

### Required Parameters
- `<channel>` - Channel name (e.g., `#general`)
- `<channel_id>` - Unique channel identifier (UUID)
- `<server>` - Server where channel was created
- `<created_timestamp>` - RFC3339 timestamp of channel creation

### Optional Parameters

#### Topic Information
- `TOPIC <topic> <setter> <timestamp>` - Channel topic with setter and time
- `NOTOPIC` - No topic set

#### Channel Modes
- `+<modes>` - Channel modes (e.g., `+nt` for no external messages, topic ops only)

#### Channel Properties
- `KEY <key>` - Channel key/password
- `LIMIT <number>` - User limit

#### Access Control
- `BANMASKS <masks>` - Ban masks (comma-separated)
- `EXCEPTMASKS <masks>` - Exception masks (comma-separated)
- `INVITEMASKS <masks>` - Invite masks (comma-separated)

#### Metadata
- `MEMBERS <count>` - Current member count

### Example Messages

#### Basic Channel with Topic
```
CBURST #general 123e4567-e89b-12d3-a456-426614174000 server1.example.com 2024-01-01T12:00:00Z TOPIC "Welcome to #general!" admin!admin@server1.example.com 2024-01-01T12:30:00Z +nt MEMBERS 5
```

#### Private Channel with Key and Limit
```
CBURST #private 123e4567-e89b-12d3-a456-426614174001 server1.example.com 2024-01-01T12:00:00Z NOTOPIC +ikl KEY secret123 LIMIT 10 MEMBERS 3
```

#### Moderated Channel with Ban Masks
```
CBURST #moderated 123e4567-e89b-12d3-a456-426614174002 server1.example.com 2024-01-01T12:00:00Z TOPIC "Moderated channel - be respectful" moderator!mod@server1.example.com 2024-01-01T13:00:00Z +mn BANMASKS "*!*@spammer.com,*!*@baduser.net" MEMBERS 8
```

## Implementation Guide

### 1. Channel Module Integration

Create a `ChannelBurstExtension` in your channel module:

```rust
use rustircd_core::extensions::{BurstExtension, BurstType};

pub struct ChannelBurstExtension {
    channels: Arc<RwLock<HashMap<String, Channel>>>,
    database: Arc<Database>,
    server_name: String,
}

#[async_trait]
impl BurstExtension for ChannelBurstExtension {
    async fn on_prepare_burst(&self, target_server: &str, burst_type: &BurstType) -> Result<Vec<Message>> {
        if !matches!(burst_type, BurstType::Channel) {
            return Ok(Vec::new());
        }
        
        // Collect all local channels and create burst messages
        // ... implementation details
    }
    
    async fn on_receive_burst(&self, source_server: &str, burst_type: &BurstType, messages: &[Message]) -> Result<()> {
        if !matches!(burst_type, BurstType::Channel) {
            return Ok(());
        }
        
        // Process incoming channel burst messages
        // ... implementation details
    }
    
    // ... other required methods
}
```

### 2. Server Integration

Register the extension in your server initialization:

```rust
// In server initialization
let channel_burst_extension = Box::new(ChannelBurstExtension::new(
    channels,
    database.clone(),
    config.server.name.clone(),
));

extension_manager.register_burst_extension(channel_burst_extension).await?;
```

### 3. Burst Handling

Use the server's burst handling methods:

```rust
// Prepare burst for sending
let messages = server.prepare_channel_burst("target-server.example.com").await?;

// Process incoming burst
server.handle_channel_burst("source-server.example.com", &messages).await?;
```

## Channel Data Synchronization

### Local vs Remote Channels

- **Local Channels**: Created on this server, marked with `is_local: true`
- **Remote Channels**: Received from other servers, marked with `is_local: false`

### Synchronization Strategy

1. **Channel Creation**: Only local channels are included in outgoing bursts
2. **Channel Updates**: Changes to local channels trigger new bursts
3. **Channel Deletion**: Remote channels are cleaned up when server disconnects
4. **Conflict Resolution**: Local channels take precedence over remote channels

### Data Consistency

- **Unique IDs**: Each channel has a UUID for network-wide identification
- **Server Attribution**: Channels are tagged with their origin server
- **Timestamp Tracking**: Creation and modification times are preserved
- **Mode Synchronization**: Channel modes are synchronized across servers

## Error Handling

### Message Parsing Errors
- Malformed messages are logged and skipped
- Missing required parameters cause message rejection
- Invalid timestamps default to current time

### Network Errors
- Failed burst preparations are logged but don't stop server operation
- Burst processing failures are logged with detailed error information
- Partial bursts are processed successfully (failed messages are skipped)

### Recovery Mechanisms
- Server reconnection triggers full burst synchronization
- Missing channel data can be re-requested
- Database inconsistencies are logged for manual review

## Performance Considerations

### Burst Size Optimization
- Large networks may need burst pagination
- Channel count limits prevent oversized bursts
- Compression can be added for large channel lists

### Network Efficiency
- Bursts are sent only when needed (server connect, channel changes)
- Delta updates can be implemented for large networks
- Burst frequency can be throttled to prevent network flooding

### Memory Management
- Remote channels are cleaned up on server disconnect
- Channel data is stored efficiently with minimal duplication
- Memory usage scales with channel count and server count

## Security Considerations

### Channel Key Protection
- Channel keys are included in bursts (network-wide visibility)
- Consider key rotation for sensitive channels
- Implement access controls for burst message processing

### Server Authentication
- Burst messages should only be accepted from authenticated servers
- Server identity verification prevents channel hijacking
- Network topology validation ensures proper routing

### Data Validation
- All burst parameters should be validated
- Channel names must conform to IRC standards
- Mode parameters should be sanitized

## Testing

### Unit Tests
- Test burst message creation and parsing
- Verify channel data serialization/deserialization
- Test error handling for malformed messages

### Integration Tests
- Test server-to-server burst exchange
- Verify channel synchronization across multiple servers
- Test server connection/disconnection scenarios

### Performance Tests
- Measure burst processing time for large channel lists
- Test memory usage with many remote channels
- Verify network bandwidth usage during bursts

## Future Enhancements

### Member Synchronization
- Include channel member lists in bursts
- Synchronize member modes and permissions
- Handle member join/part events across servers

### Real-time Updates
- Send delta updates for channel changes
- Implement event-driven burst triggers
- Add channel modification notifications

### Advanced Features
- Channel merge/split operations
- Channel hierarchy support
- Channel metadata extensions

### Monitoring and Analytics
- Burst message statistics
- Channel synchronization metrics
- Network health monitoring

## Configuration

### Server Configuration
```toml
[server]
name = "example.com"
# ... other settings

[modules]
enabled_modules = ["channel"]  # Enable channel module for burst support
```

### Extension Configuration
```toml
[extensions.channel_burst]
enabled = true
max_burst_size = 1000
burst_timeout = 30
```

## Troubleshooting

### Common Issues

1. **Channels not synchronizing**
   - Check if channel module is enabled
   - Verify burst extension registration
   - Check server authentication

2. **Burst messages failing**
   - Review message format compliance
   - Check parameter validation
   - Verify server connectivity

3. **Memory usage growing**
   - Check remote channel cleanup
   - Verify server disconnection handling
   - Monitor channel count growth

### Debug Logging
Enable debug logging to see burst processing details:

```toml
[logging]
level = "debug"
```

This will show:
- Burst message preparation
- Message parsing and validation
- Channel synchronization events
- Error details and stack traces

## Examples

See the following example files:
- `examples/channel_burst_example.rs` - Basic burst functionality demonstration
- `examples/channel_burst_integration.rs` - Complete integration example

These examples show:
- How to create and register burst extensions
- Channel burst message format and parsing
- Server integration and burst handling
- Error handling and logging
