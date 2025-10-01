# LUSERS Command System for RustIRCD

The LUSERS command provides network statistics and user information, allowing clients to query the current state of the IRC network. This implementation is fully RFC 1459 compliant and provides comprehensive network monitoring capabilities.

## Overview

The LUSERS command allows clients to:
- Query network-wide user statistics
- View operator counts and status
- Monitor channel activity
- Check server connectivity
- Track connection statistics
- Monitor network health

## RFC 1459 Compliance

The LUSERS implementation follows RFC 1459 Section 4.3.1 specifications:

### Command Format
```
LUSERS [<mask> [<target>]]
```

### Response Format
The server responds with multiple numeric replies providing comprehensive network statistics.

## Numeric Replies

### RPL_LUSERCLIENT (251)
```
:server 251 * :There are <users> users and <services> services on <servers> servers
```
- **users**: Total number of users on the network
- **services**: Number of services (bots, etc.) on the network
- **servers**: Number of servers in the network

### RPL_LUSEROP (252)
```
:server 252 * <operators> :operator(s) online
```
- **operators**: Number of operators currently online

### RPL_LUSERUNKNOWN (253)
```
:server 253 * <unknown> :unknown connection(s)
```
- **unknown**: Number of unregistered connections

### RPL_LUSERCHANNELS (254)
```
:server 254 * <channels> :channels formed
```
- **channels**: Total number of channels created

### RPL_LUSERME (255)
```
:server 255 * :I have <clients> clients and <servers> servers
```
- **clients**: Number of clients connected to this server
- **servers**: Number of servers connected to this server

### RPL_LOCALUSERS (265)
```
:server 265 * :Current local users: <current>, max: <max>
```
- **current**: Current number of local users
- **max**: Maximum number of local users allowed

### RPL_GLOBALUSERS (266)
```
:server 266 * :Current global users: <current>, max: <max>
```
- **current**: Current number of global users
- **max**: Maximum number of global users allowed

## Implementation Details

### Server Integration

The LUSERS command is implemented in the server with the following method:

```rust
pub async fn handle_lusers(&self, client_id: uuid::Uuid, _message: Message) -> Result<()>
```

### Statistics Collection

The implementation collects statistics from multiple sources:

#### User Statistics
- **Total Users**: Count of all registered users
- **Local Users**: Users connected to this server
- **Global Users**: Users across the entire network
- **Operators**: Users with operator privileges

#### Connection Statistics
- **Registered Connections**: Fully registered users
- **Unknown Connections**: Unregistered connections
- **Server Connections**: Connected servers

#### Channel Statistics
- **Total Channels**: Number of channels created
- **Active Channels**: Channels with current activity

#### Server Statistics
- **Connected Servers**: Number of servers in the network
- **Server Capacity**: Maximum user limits

### Statistics Methods

The server provides several methods for collecting statistics:

```rust
async fn get_user_count(&self) -> u32
async fn get_operator_count(&self) -> u32
async fn get_channel_count(&self) -> u32
async fn get_server_count(&self) -> u32
async fn get_unknown_connection_count(&self) -> u32
async fn get_local_user_count(&self) -> u32
async fn get_global_user_count(&self) -> u32
```

## Configuration

### Server Configuration

LUSERS functionality is enabled by default and requires no special configuration. However, you can customize the behavior through server settings:

```toml
[server]
name = "example.com"
max_clients = 1000  # Affects local/global user limits

[modules]
enabled_modules = ["channel", "throttling"]  # Required for full statistics
```

### Numeric Reply Configuration

LUSERS numeric replies can be customized in `replies.toml`:

```toml
[replies.251]
code = 251
text = ":There are {param0} users and {param1} services on {param2} servers"
description = "RPL_LUSERCLIENT - LUSER client info"

[replies.252]
code = 252
text = "{param0} :operator(s) online"
description = "RPL_LUSEROP - LUSER operator info"

[replies.253]
code = 253
text = "{param0} :unknown connection(s)"
description = "RPL_LUSERUNKNOWN - LUSER unknown connections"

[replies.254]
code = 254
text = "{param0} :channels formed"
description = "RPL_LUSERCHANNELS - LUSER channels"

[replies.255]
code = 255
text = ":I have {param0} clients and {param1} servers"
description = "RPL_LUSERME - LUSER server info"

[replies.265]
code = 265
text = ":Current local users: {param0}, max: {param1}"
description = "RPL_LOCALUSERS - Local users info"

[replies.266]
code = 266
text = ":Current global users: {param0}, max: {param1}"
description = "RPL_GLOBALUSERS - Global users info"
```

## Usage Examples

### Basic LUSERS Command

```irc
> /LUSERS

< :server.example.com 251 * :There are 15 users and 2 services on 3 servers
< :server.example.com 252 * 2 :operator(s) online
< :server.example.com 253 * 0 :unknown connection(s)
< :server.example.com 254 * 8 :channels formed
< :server.example.com 255 * :I have 5 clients and 2 servers
< :server.example.com 265 * :Current local users: 5, max: 1000
< :server.example.com 266 * :Current global users: 15, max: 3000
```

### Empty Server

```irc
> /LUSERS

< :server.example.com 251 * :There are 0 users and 0 services on 1 servers
< :server.example.com 252 * 0 :operator(s) online
< :server.example.com 253 * 0 :unknown connection(s)
< :server.example.com 254 * 0 :channels formed
< :server.example.com 255 * :I have 0 clients and 0 servers
< :server.example.com 265 * :Current local users: 0, max: 1000
< :server.example.com 266 * :Current global users: 0, max: 1000
```

### Active Network

```irc
> /LUSERS

< :server.example.com 251 * :There are 150 users and 5 services on 5 servers
< :server.example.com 252 * 8 :operator(s) online
< :server.example.com 253 * 3 :unknown connection(s)
< :server.example.com 254 * 25 :channels formed
< :server.example.com 255 * :I have 30 clients and 4 servers
< :server.example.com 265 * :Current local users: 30, max: 1000
< :server.example.com 266 * :Current global users: 150, max: 5000
```

## Integration Points

### Statistics Manager

The LUSERS system integrates with the server's statistics manager:

- **Real-time Updates**: Statistics are calculated in real-time
- **Performance Tracking**: Monitors connection and usage patterns
- **Resource Monitoring**: Tracks server capacity and usage

### Database Integration

LUSERS statistics are collected from the server's database:

- **User Database**: Accurate user counts and information
- **Channel Database**: Channel creation and activity statistics
- **Server Database**: Network topology and connectivity

### Connection Handler

The connection handler provides connection statistics:

- **Connection States**: Tracks registered vs unregistered connections
- **Connection Types**: Distinguishes between client and server connections
- **Connection Limits**: Monitors server capacity and limits

### Module Integration

LUSERS integrates with various modules:

- **Channel Module**: Provides channel statistics
- **Throttling Module**: Tracks connection patterns and limits
- **Operator System**: Counts operators and their status

## Performance Considerations

### Statistics Calculation

- **Efficient Counting**: Statistics are calculated efficiently using database queries
- **Caching**: Frequently accessed statistics may be cached for performance
- **Real-time Updates**: Statistics are updated in real-time as users connect/disconnect

### Network Impact

- **Low Overhead**: LUSERS command has minimal network impact
- **Response Time**: Fast response times for network statistics
- **Scalability**: Scales well with network size

## Security Considerations

### Access Control

- **Public Access**: LUSERS command is available to all users
- **No Sensitive Data**: Only public statistics are disclosed
- **Operator Information**: Operator counts are public (no specific operator details)

### Information Disclosure

- **Network Topology**: Server counts reveal network structure
- **User Statistics**: User counts provide network activity information
- **Capacity Information**: Server limits may reveal infrastructure details

## Troubleshooting

### Common Issues

1. **Incorrect Statistics**
   - Check database connectivity
   - Verify module integration
   - Ensure statistics are being updated

2. **Missing Numeric Replies**
   - Check replies.toml configuration
   - Verify numeric reply definitions
   - Ensure proper message formatting

3. **Performance Issues**
   - Monitor statistics calculation time
   - Check database query performance
   - Consider caching for large networks

### Debug Information

Enable debug logging to see LUSERS processing:

```toml
[logging]
level = "debug"
```

This will show:
- LUSERS command processing
- Statistics collection
- Numeric reply generation
- Performance metrics

## Future Enhancements

### Planned Features

1. **Historical Statistics**
   - Track statistics over time
   - Provide trends and patterns
   - Historical data storage

2. **Advanced Filtering**
   - Filter statistics by criteria
   - Custom statistics queries
   - Detailed breakdowns

3. **Real-time Updates**
   - Push statistics updates
   - Live network monitoring
   - Real-time dashboards

4. **Network Analytics**
   - Usage pattern analysis
   - Performance metrics
   - Capacity planning tools

## Testing

### Unit Tests

The LUSERS system includes comprehensive unit tests:

- Statistics calculation accuracy
- Numeric reply formatting
- Error handling
- Performance benchmarks

### Integration Tests

Integration tests verify:

- End-to-end LUSERS functionality
- Database integration
- Module integration
- Network scenarios

### Performance Tests

Performance tests ensure:

- Fast statistics calculation
- Scalable network handling
- Memory efficiency
- Response time consistency

## Examples

See the following example files:
- `examples/lusers_example.rs` - Basic LUSERS functionality demonstration
- `examples/configs/lusers_example.toml` - Configuration examples

These examples show:
- LUSERS command usage
- Statistics collection
- Numeric reply formatting
- Configuration options
- Error handling
