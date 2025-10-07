# Atheme IRC Services Protocol Implementation

This document describes the Atheme IRC Services protocol implementation in RustIRCD's services library.

## Overview

Atheme communicates with IRC servers using a specific server-to-server protocol. This implementation handles the commands and responses that Atheme sends to RustIRCD.

## Protocol Commands Handled

### Connection Management

#### PING/PONG
- **PING**: Atheme sends PING to keep connection alive
- **PONG**: Response to PING from Atheme
- **Implementation**: `handle_atheme_ping()`, `handle_atheme_pong()`

#### SQUIT
- **Format**: `SQUIT <server> [reason]`
- **Purpose**: Atheme requests server disconnection
- **Implementation**: `handle_atheme_squit()`

### User Management

#### UID (User Introduction)
- **Format**: `UID <nick> <hopcount> <username> <host> <servertoken> <umodes> :<realname>`
- **Purpose**: Introduces a user from Atheme services to the network
- **Implementation**: `handle_atheme_uid()`

#### SVSNICK (Service Nickname Change)
- **Format**: `SVSNICK <oldnick> <newnick> <timestamp>`
- **Purpose**: Changes a user's nickname
- **Implementation**: `handle_atheme_svsnick()`

#### SETHOST (Service Host Change)
- **Format**: `SETHOST <nick> <host>`
- **Purpose**: Changes a user's hostname
- **Implementation**: `handle_atheme_sethost()`

### Channel Management

#### SJOIN (Service Join)
- **Format**: `SJOIN <timestamp> <channel> [<modes>] :<members>`
- **Purpose**: Services joining channels with specific modes
- **Implementation**: `handle_atheme_sjoin()`

#### SVSJOIN (Service Force Join)
- **Format**: `SVSJOIN <nick> <channel>`
- **Purpose**: Forces a user to join a channel
- **Implementation**: `handle_atheme_svsjoin()`

#### SVSPART (Service Force Part)
- **Format**: `SVSPART <nick> <channel> [reason]`
- **Purpose**: Forces a user to part a channel
- **Implementation**: `handle_atheme_svspart()`

### Mode Management

#### SVSMODE (Service Mode)
- **Format**: `SVSMODE <target> <modes>`
- **Purpose**: Changes modes on users or channels
- **Implementation**: `handle_atheme_svsmode()`

#### SVS2MODE (Service Mode v2)
- **Format**: `SVS2MODE <target> <modes>`
- **Purpose**: Enhanced mode changes
- **Implementation**: `handle_atheme_svs2mode()`

### Messaging

#### NOTICE
- **Format**: `NOTICE <target> :<text>`
- **Purpose**: Sends notices to users or channels
- **Implementation**: `handle_atheme_notice()`

#### PRIVMSG
- **Format**: `PRIVMSG <target> :<text>`
- **Purpose**: Sends messages to users or channels
- **Implementation**: `handle_atheme_privmsg()`

## Implementation Status

### ✅ Implemented
- Command parsing and routing
- Basic message handling structure
- Protocol command recognition
- Error handling and validation

### 🚧 TODO (Implementation Required)
- Database integration for user/channel management
- Network propagation of commands to other servers
- Message forwarding to local users/channels
- Connection management and message sending back to Atheme

## Integration Points

The Atheme integration connects to RustIRCD through:

1. **Message Parsing**: Uses RustIRCD's `Message` type for protocol parsing
2. **Database**: Integrates with RustIRCD's user and channel database
3. **Network**: Uses RustIRCD's server connection manager for propagation
4. **Logging**: Uses RustIRCD's tracing system for debugging

## Configuration

Atheme integration is configured through the `AthemeConfig` structure:

```rust
AthemeConfig {
    enabled: true,
    service_name: "services.example.org",
    hostname: "localhost",
    port: 6666,
    password: "password",
    tls: false,
    timeout_seconds: 30,
    reconnect_interval: 60,
    max_reconnect_attempts: 10,
}
```

## Usage

The Atheme integration is designed to be services-agnostic. The core RustIRCD doesn't need to know about Atheme-specific commands - they are handled entirely within the services library.

```rust
// Initialize Atheme integration
let atheme = AthemeIntegration::new(config);
atheme.initialize(&server_config).await?;

// Handle incoming messages
atheme.handle_atheme_message(&message).await?;
```

## Future Enhancements

1. **Complete Implementation**: Fill in all TODO items with actual functionality
2. **Service Bot Integration**: Handle NickServ, ChanServ, etc. bot messages
3. **Advanced Features**: Support for more Atheme-specific commands
4. **Error Recovery**: Better error handling and reconnection logic
5. **Performance**: Optimize message handling and database operations
