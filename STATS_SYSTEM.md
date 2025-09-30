# Enhanced STATS System for RustIRCD

The enhanced STATS system provides RFC 1459 compliant statistics reporting with extensible module support, allowing modules to define their own STATS query letters.

## Features

- **RFC 1459 Compliance**: Full implementation of standard STATS query types
- **Module Extensibility**: Modules can define custom STATS query letters
- **Real-time Statistics**: Live tracking of server metrics and command usage
- **Throttling Integration**: Built-in throttling module with "T" STATS query
- **Configurable Replies**: All STATS responses can be customized
- **Security Controls**: Configurable information disclosure with operator access control
- **Privacy Protection**: Hide sensitive information like IPs and hostmasks when configured

## RFC 1459 Standard STATS Commands

### `/STATS l` - Server Links
Lists all connected servers with connection statistics.

**Response Format:**
```
:server 211 * l server.example.com 0 0 0 0 0 0
:server 219 * l :End of STATS report
```

**Parameters:**
- Server name
- Send queue size
- Messages sent
- Bytes sent  
- Messages received
- Bytes received
- Time online (seconds)

### `/STATS m` - Command Usage Statistics
Shows the most frequently used commands with usage counts.

**Response Format:**
```
:server 212 * PRIVMSG 150 0 0
:server 212 * JOIN 89 0 0
:server 219 * m :End of STATS report
```

**Parameters:**
- Command name
- Usage count
- Total bytes
- Remote count

### `/STATS o` - Online Operators
Lists all operators currently online.

**Response Format:**
```
:server 243 * O admin@* * admin 0 Operator
:server 219 * o :End of STATS report
```

**Parameters:**
- Hostmask
- Name
- Port (0 for users)
- Class

### `/STATS u` - Server Uptime
Shows server uptime in seconds.

**Response Format:**
```
:server 242 * :Server Up 3600 seconds
:server 219 * u :End of STATS report
```

### `/STATS y` - Class Information
Shows connection class configuration.

**Response Format:**
```
:server 218 * Y default 120 600 1024
:server 219 * y :End of STATS report
```

**Parameters:**
- Class name
- Ping frequency (seconds)
- Connect frequency (seconds)
- Maximum send queue

### `/STATS c` - Connection Information
Shows connection statistics.

**Response Format:**
```
:server 212 * CONNECTIONS 150 15000 0
:server 219 * c :End of STATS report
```

## Module-Specific STATS

### `/STATS T` - Throttling Module
Shows throttling statistics including IP addresses, stages, and remaining throttle times.

**Response Format:**
```
:server 244 * THROTTLING 192.168.1.100 THROTTLED stage=2 remaining=45s
:server 244 * THROTTLING 192.168.1.101 ACTIVE connections=3
:server 219 * T :End of STATS report
```

**Information Provided:**
- IP addresses being tracked
- Current throttling stage (0-10)
- Remaining throttle time in seconds
- Number of active connections

## Implementation Details

### Statistics Tracking

The `StatisticsManager` tracks:
- Total connections and disconnections
- Message counts and byte counts
- Command usage statistics
- Server uptime
- Current client/server counts

### Module Integration

Modules can implement STATS queries by:

1. **Implementing the Module trait methods:**
```rust
async fn handle_stats_query(&mut self, query: &str, client_id: uuid::Uuid) -> Result<Vec<ModuleStatsResponse>> {
    // Handle custom STATS query
}

fn get_stats_queries(&self) -> Vec<String> {
    vec!["T".to_string()] // Return supported query letters
}
```

2. **Returning ModuleStatsResponse:**
```rust
ModuleStatsResponse::ModuleStats("THROTTLING".to_string(), "192.168.1.100 THROTTLED stage=2 remaining=45s".to_string())
```

### Configuration

#### STATS Security Settings

Control information disclosure in STATS commands:

```toml
[server]
# Show server IPs/hostnames in STATS commands (even for operators)
# When false, even operators cannot see server details for enhanced security
show_server_details_in_stats = true  # Default: true
```

#### STATS Reply Customization

STATS responses can be customized in `replies.toml`:

```toml
[replies.211]
code = 211
text = "l {server} {sendq} {sent_messages} {sent_bytes} {received_messages} {received_bytes} {time_online}"
description = "RPL_STATSLINKINFO - Stats link info"

[replies.212]
code = 212
text = "m {command} {count} {bytes} {remote_count}"
description = "RPL_STATSCOMMANDS - Stats commands"

[replies.219]
code = 219
text = "{letter} :End of STATS report"
description = "RPL_ENDOFSTATS - End of stats"
```

## Usage Examples

### Basic STATS Usage
```
/STATS l          # Show server links
/STATS m          # Show command usage
/STATS o          # Show online operators
/STATS u          # Show server uptime
/STATS y          # Show class information
/STATS c          # Show connection info
```

### Module STATS Usage
```
/STATS T          # Show throttling statistics
```

### Expected Output
```
:stats.example.com 211 * l server1.example.com 0 0 0 0 0 0
:stats.example.com 219 * l :End of STATS report
```

## Security Features

### Access Control

- **Operator-only information**: Sensitive data is only shown to operators
- **Configurable disclosure**: Admins can control what information is shown even to operators
- **Privacy protection**: IP addresses and hostmasks are hidden from non-operators
- **Granular control**: Different STATS commands respect different security levels

### Security Levels

1. **Public Information**: Available to all users (basic command usage, uptime)
2. **Operator Information**: Available to operators only (hostmasks, detailed stats)
3. **Admin Information**: Available to operators only if configured (server IPs, detailed throttling)

### Example Security Output

**Non-Operator Access:**
```
:server 243 * O ***@*** * admin 0 Operator
:server 244 * THROTTLING 3 throttled IPs, 2 active IPs
```

**Operator Access (with show_server_details_in_stats=true):**
```
:server 243 * O admin@192.168.1.100 * admin 0 Operator
:server 244 * 192.168.1.100 THROTTLED stage=2 remaining=45s
```

**Operator Access (with show_server_details_in_stats=false):**
```
:server 243 * O ***@*** * admin 0 Operator
:server 244 * THROTTLING 3 throttled IPs, 2 active IPs
```

## Error Handling

- **Unknown queries**: Return empty response with "UNKNOWN" command
- **Module errors**: Logged but don't stop other modules from responding
- **Invalid parameters**: Handled gracefully with appropriate error messages
- **Access denied**: Sensitive information is hidden rather than showing errors

## Performance Considerations

- Statistics are tracked in-memory for fast access
- Module STATS queries are processed asynchronously
- Cleanup routines prevent memory leaks in statistics tracking
- Throttling statistics are cleaned up automatically

## Future Enhancements

- **Persistent statistics**: Save statistics to database
- **Historical data**: Track statistics over time periods
- **More module STATS**: Additional modules can define their own queries
- **Custom formatting**: Allow modules to define custom response formats
- **Statistics aggregation**: Combine statistics from multiple servers

## Testing

Use the provided example to test the STATS system:

```bash
cargo run --example stats_example
```

Then connect with an IRC client and test various STATS commands:

```
/connect localhost 6667
/nick testuser
/user testuser 0 * :Test User
/stats T
/stats m
/stats u
```

The throttling module will show detailed information about IP tracking and throttling states, making it easy to monitor connection patterns and throttling effectiveness.
