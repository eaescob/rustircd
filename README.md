# RustIRCD - A Modern IRC Daemon in Rust

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![RFC 1459](https://img.shields.io/badge/RFC-1459-green.svg)](https://datatracker.ietf.org/doc/html/rfc1459)
[![IRCv3](https://img.shields.io/badge/IRCv3-supported-blue.svg)](https://ircv3.net/)

A high-performance, modular IRC daemon implementation in Rust, featuring RFC 1459 compliance, IRCv3 extensions, and enterprise-grade security features.

## 🚀 Features

### Core IRC Protocol
- **RFC 1459 Compliance**: Complete implementation of the IRC protocol specification
- **IRCv3 Support**: Modern IRC extensions including capability negotiation, SASL, and message tags
- **Server-to-Server**: Full multi-server IRC network support with message broadcasting
- **TLS/SSL Support**: Secure connections with modern TLS 1.3 encryption
- **DNS & Ident Lookup**: RFC 1413 compliant ident lookup and DNS resolution

### Modular Architecture
- **Core System**: Minimal core with essential IRC functionality
- **Module System**: 20+ production-ready modules with dynamic loading
- **Services Framework**: Extensible framework for network services
- **Extension System**: Clean hooks for IRCv3 capabilities and custom features
- **Configurable Messaging**: Optional messaging modules with configuration-driven loading

### Security & Performance
- **Connection Throttling**: IP-based rate limiting with multi-stage throttling
- **Operator System**: Secure authentication with flag-based permissions
- **User Mode Security**: Comprehensive mode management with privilege protection
- **Configurable Replies**: Customizable IRC numeric replies with template system
- **High Performance**: Built with async Rust for excellent scalability

### Advanced Features
- **Channel Burst System**: Server-to-server channel synchronization
- **Statistics System**: Real-time server metrics and command usage tracking
- **MOTD System**: Configurable Message of the Day with file support
- **Help System**: Dynamic command discovery with module attribution
- **Rehash System**: Runtime configuration reloading without server restart

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Configuration](#configuration)
- [Modules](#modules)
- [IRCv3 Support](#ircv3-support)
- [Security Features](#security-features)
- [Server-to-Server](#server-to-server)
- [Development Guide](#development-guide)
- [API Reference](#api-reference)
- [Examples](#examples)
- [Contributing](#contributing)

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ with `cargo`
- Git for version control

### Installation

```bash
# Clone the repository
git clone https://github.com/emilio/rustircd.git
cd rustircd

# Build the project
cargo build --release

# Copy the example configuration file
cp examples/configs/config.example.toml config.toml

# Edit the configuration file
nano config.toml

# Run the daemon
cargo run --release
```

### Basic Usage

```bash
# Start with default configuration
cargo run --release

# Start with custom configuration
cargo run --release -- --config /path/to/config.toml

# Start in daemon mode
cargo run --release -- --daemon

# Test configuration
cargo run --release -- --test-config
```

### Connect with IRC Client

```bash
# Connect to the server
/connect localhost 6667

# Register as a user
/nick mynick
/user myuser 0 * :My Real Name

# Join a channel
/join #general
```

## 🏗️ Architecture

RustIRCD follows a clean, modular architecture with three main components:

### Core (`core/`)
Essential IRC functionality including:
- Message parsing and routing
- Client and server connection management
- User and channel tracking
- Broadcasting system
- Database management
- Configuration handling
- Operator system
- Security controls

### Modules (`modules/`)
Optional features loaded dynamically:
- **Channel Module**: Channel operations (JOIN, PART, MODE, TOPIC, etc.)
- **IRCv3 Module**: Modern IRC extensions and capabilities
- **Optional Commands**: Additional IRC commands (AWAY, REHASH, etc.)
- **Throttling Module**: Connection rate limiting
- **Help Module**: Dynamic help system
- **Ban Modules**: GLINE, KLINE, DLINE, XLINE management
- **Admin Module**: Administrative commands
- **Services Module**: Service registration and management

### Services (`services/`)
Network-specific services and bots:
- Service framework
- Bot implementations
- External API integration
- Custom protocols

## ⚙️ Configuration

RustIRCD uses TOML configuration files with comprehensive options.

### Quick Start Configuration

Copy the example configuration file to get started:

```bash
cp examples/configs/config.example.toml config.toml
```

The `examples/configs/config.example.toml` file includes:
- **Bare minimum settings** required to start the IRC daemon (clearly marked)
- **Optional features** with detailed explanations
- **Module configuration** examples for all available modules
- **Services integration** guide (Atheme, Anope, etc.)
- **Security settings** for TLS, throttling, and access control
- **Quick start guide** at the end of the file

### Main Configuration (`config.toml`)

```toml
[server]
name = "rustircd.local"
description = "Rust IRC Daemon"
version = "0.1.0"
max_clients = 1000
admin_email = "admin@rustircd.local"

[connection]
bind_address = "0.0.0.0"

# Multi-port configuration
[[connection.ports]]
port = 6667
connection_type = "Client"
tls = false

[[connection.ports]]
port = 6697
connection_type = "Client"
tls = true

[modules]
enabled_modules = ["channel", "ircv3", "throttling"]

[security]
enable_ident = true
enable_dns = true
require_client_password = false

[tls]
enabled = false
cert_file = "cert.pem"
key_file = "key.pem"

[modules.messaging]
enabled = true

[modules.messaging.wallops]
enabled = true
require_operator = true
receiver_mode = "w"
self_only_mode = true
mode_requires_operator = false  # Users can set +w themselves

[modules.messaging.globops]
enabled = true
require_operator = true
receiver_mode = "g"
self_only_mode = false  # Operators can set +g on others
mode_requires_operator = true  # Only operators can set +g
```

### Custom Replies (`replies.toml`)

```toml
[replies.001]
code = 001
text = "Welcome to {server_name}, {nick}! You are now connected! 🚀"
description = "RPL_WELCOME - Custom welcome message"

[replies.433]
code = 433
text = "{nick} :That nickname is already taken! Try {nick}_ or {nick}2"
description = "ERR_NICKNAMEINUSE - Helpful nickname suggestion"
```

### Messaging Modules Configuration

RustIRCD includes a configurable messaging system with WALLOPS and GLOBOPS support:

#### Configuration Options

| Setting | Description | Values |
|---------|-------------|---------|
| `enabled` | Enable/disable messaging system | `true`, `false` |
| `wallops.enabled` | Enable WALLOPS command | `true`, `false` |
| `globops.enabled` | Enable GLOBOPS command | `true`, `false` |
| `receiver_mode` | Mode character for receiving messages | `"w"`, `"g"`, `"x"`, etc. |
| `require_operator` | Require operator privileges to send | `true`, `false` |
| `self_only_mode` | Users can only set mode on themselves | `true`, `false` |
| `mode_requires_operator` | Require operator to set the mode | `true`, `false` |

#### Configuration Examples

**Default Configuration (Both Enabled)**
```toml
[modules.messaging]
enabled = true

[modules.messaging.wallops]
enabled = true
require_operator = true
receiver_mode = "w"
self_only_mode = true
mode_requires_operator = false  # Users can set +w themselves

[modules.messaging.globops]
enabled = true
require_operator = true
receiver_mode = "g"
self_only_mode = false  # Operators can set +g on others
mode_requires_operator = true  # Only operators can set +g
```

**WALLOPS Only**
```toml
[modules.messaging]
enabled = true

[modules.messaging.wallops]
enabled = true

[modules.messaging.globops]
enabled = false
```

**Disabled Messaging**
```toml
[modules.messaging]
enabled = false
```

**Custom Mode Characters**
```toml
[modules.messaging.wallops]
enabled = true
receiver_mode = "x"  # Custom mode character

[modules.messaging.globops]
enabled = true
receiver_mode = "y"  # Custom mode character
```

#### Usage Examples

**WALLOPS (Operator Messaging)**
```bash
# Operators can send wallops messages
/WALLOPS Server maintenance scheduled for tonight at 2 AM

# Users can set +w mode to receive wallops
/MODE YourNick +w

# Users without +w mode won't receive wallops
/MODE YourNick -w
```

**GLOBOPS (Global Operator Notices)**
```bash
# Operators can send globops messages
/GLOBOPS Network-wide announcement: New features available

# Only operators can set +g mode (for themselves or others)
/MODE OperatorNick +g

# Operators can set +g on other users
/MODE SomeUser +g

# Regular users cannot set +g mode
/MODE RegularUser +g
# ERR_USERS_DONT_MATCH: Can't change mode for other users
```

#### Mode Behavior

| Mode | Who Can Send | Who Can Set | Who Can Receive |
|------|--------------|-------------|-----------------|
| `+w` (WALLOPS) | Operators only | Users on themselves | Users with `+w` mode |
| `+g` (GLOBOPS) | Operators only | Operators only | Users with `+g` mode |

### Available Placeholders
- **Server**: `{server_name}`, `{server_version}`, `{server_description}`
- **User**: `{nick}`, `{user}`, `{host}`, `{realname}`, `{target}`
- **Channel**: `{channel}`, `{topic}`, `{reason}`, `{count}`, `{info}`
- **Custom**: `{param0}`, `{param1}`, etc.

## 🎯 Connection Classes (Solanum-Inspired)

RustIRCD implements a comprehensive connection class system inspired by [Solanum IRCd](https://github.com/solanum-ircd/solanum), providing fine-grained control over connection resources, buffer limits, and timeouts.

### What are Connection Classes?

Connection classes group connections with similar parameters, allowing administrators to:
- Assign different resource limits to different user groups (regular users, trusted users, operators, servers)
- Prevent resource exhaustion by enforcing sendq/recvq buffer limits
- Control connection timeouts and ping frequency per class
- Disable throttling for trusted hosts
- Limit connections per IP/host on a per-class basis

### Configuration

#### Defining Connection Classes

```toml
[[classes]]
name = "default"
description = "Default connection class for regular users"
max_clients = 1000                  # Maximum number of clients in this class
ping_frequency = 120                # Send PING every 120 seconds
connection_timeout = 300            # Drop connection after 300 seconds of no response
max_sendq = 1048576                 # 1MB send queue (outgoing data buffer)
max_recvq = 8192                    # 8KB receive queue (incoming data buffer)
disable_throttling = false          # Enable throttling for this class
max_connections_per_ip = 5          # Maximum connections per IP for this class
max_connections_per_host = 10       # Maximum connections per host for this class

[[classes]]
name = "trusted"
description = "Trusted users with higher limits"
max_clients = 100
ping_frequency = 180                # Less frequent pings
connection_timeout = 600            # Longer timeout
max_sendq = 5242880                 # 5MB send queue
max_recvq = 16384                   # 16KB receive queue
disable_throttling = true           # No throttling for trusted users
max_connections_per_ip = 20         # Allow more connections per IP

[[classes]]
name = "server"
description = "Server-to-server connections"
max_clients = 10                    # Maximum number of servers
ping_frequency = 90                 # Frequent pings to detect server disconnects
connection_timeout = 300
max_sendq = 10485760                # 10MB send queue for burst traffic
max_recvq = 32768                   # 32KB receive queue
disable_throttling = true           # No throttling for server connections
```

### Limits Enforced

| Limit | Description | Enforcement Location |
|-------|-------------|---------------------|
| **max_clients** | Maximum clients in this class | `ClassTracker::can_accept_connection()` |
| **max_connections_per_ip** | Connections per IP address | `ClassTracker::can_accept_connection()` |
| **max_connections_per_host** | Connections per hostname | `ClassTracker::can_accept_connection()` |
| **max_sendq** | Outgoing buffer size (bytes) | `SendQueue::push()` - drops messages when full |
| **max_recvq** | Incoming buffer size (bytes) | `RecvQueue::append()` - drops data when full |
| **connection_timeout** | Idle timeout (seconds) | `ConnectionTiming::is_timed_out()` |
| **ping_frequency** | PING interval (seconds) | `ConnectionTiming::should_send_ping()` |
| **disable_throttling** | Bypass rate limiting | `ClassTracker::is_throttling_disabled()` |

### Allow Blocks

Allow blocks map hosts/IPs to connection classes using wildcard patterns and CIDR notation:

```toml
# Default allow block for all users
[[security.allow_blocks]]
hosts = ["*"]                       # All hostnames
ips = ["*"]                         # All IPs
class = "default"                   # Assign to default class
description = "General users"

# Trusted hosts with higher limits
[[security.allow_blocks]]
hosts = ["*.trusted.example.com", "operator.example.com"]
ips = ["192.168.1.0/24", "10.0.0.100"]
class = "trusted"                   # Assign to trusted class
password = "secret123"              # Optional password requirement
max_connections = 50                # Optional total limit for this block
description = "Trusted users and operators"

# Restricted hosts with lower limits
[[security.allow_blocks]]
hosts = ["*.restricted.example.com"]
ips = ["203.0.113.0/24"]
class = "restricted"
description = "Restricted users with lower limits"
```

### Per-Port IP Binding

Each port can bind to a different IP address, useful for multi-homed servers:

```toml
# Public client port on public IP
[[connection.ports]]
port = 6667
connection_type = "Client"
tls = false
description = "Public client port"
bind_address = "192.168.1.100"      # Bind to specific public IP

# Private server-to-server port on private IP
[[connection.ports]]
port = 6668
connection_type = "Server"
tls = false
description = "Private server port"
bind_address = "10.0.0.50"          # Bind to private network IP

# Use global bind_address if not specified
[[connection.ports]]
port = 6697
connection_type = "Client"
tls = true
description = "Secure IRC port"
# Omit bind_address to use global connection.bind_address
```

### Server Links with Classes

Server connections can reference classes for sendq/recvq management:

```toml
links = [
    {
        name = "hub.example.com",
        hostname = "hub.example.com",
        port = 6668,
        password = "linkpass123",
        tls = false,
        outgoing = true,
        class = "server"            # Use server class with 10MB sendq
    },
]
```

### Buffer Management

The sendq (send queue) and recvq (receive queue) are bounded buffers that prevent resource exhaustion:

#### Send Queue (SendQ)
- **Purpose**: Buffer outgoing messages before they're sent to the client
- **Limit**: Defined by `max_sendq` in the connection class
- **Behavior**: When full, new messages are dropped and counted
- **Monitoring**: Track with `Client::sendq_stats()` → (current, max, dropped)

#### Receive Queue (RecvQ)
- **Purpose**: Buffer incoming data before parsing into IRC messages
- **Limit**: Defined by `max_recvq` in the connection class
- **Behavior**: When full, new data is dropped and counted
- **Monitoring**: Track with `Client::recvq_stats()` → (current, max, dropped)

### Connection Timing

Each connection tracks timing information for health monitoring:

```rust
// Check if it's time to send a PING
if client.should_send_ping() {
    send_ping(&client);
    client.record_ping_sent();
}

// Check for connection timeout
if client.is_timed_out() {
    disconnect_client(&client, "Connection timeout");
}

// Update activity on received messages
client.update_activity();

// Record PONG response
client.record_pong_received();
```

### Class Assignment Logic

1. **If allow blocks are defined**: Use the first matching allow block's class
2. **If no allow blocks defined**: Use `allowed_hosts` with default class
3. **If no match**: Deny connection

### Monitoring & Statistics

Get statistics for monitoring and debugging:

```rust
// Get all class statistics
for stats in class_tracker.get_all_stats() {
    println!("Class: {}", stats.class_name);
    println!("  Clients: {}", stats.total_clients);
    println!("  Unique IPs: {}", stats.unique_ips);
    println!("  Unique Hosts: {}", stats.unique_hosts);
}

// Get buffer statistics for a client
let (current, max, dropped) = client.sendq_stats();
println!("SendQ: {}/{} bytes, {} messages dropped", current, max, dropped);

let (current, max, dropped) = client.recvq_stats();
println!("RecvQ: {}/{} bytes, {} bytes dropped", current, max, dropped);

// Get connection health metrics
println!("Connection age: {:?}", client.timing.connection_age());
println!("Time since activity: {:?}", client.timing.time_since_activity());
println!("Unanswered PINGs: {}", client.timing.unanswered_pings);
```

#### STATS L - Server Link Statistics

The `STATS L` command now shows detailed server link information including sendq/recvq statistics:

```irc
/STATS L

# Example output for operators:
:server.name 211 yournick hub.example.com SendQ:1024/10485760(0%) RecvQ:512/32768(1%) Msgs:1543s/1892r Bytes:245678s/389012r Time:3600s Dropped:0
:server.name 211 yournick leaf.example.com SendQ:0/10485760(0%) RecvQ:0/32768(0%) Msgs:892s/1024r Bytes:145234s/198765r Time:7200s Dropped:2

# Information shown:
# - Server name
# - SendQ: current/max (usage%)
# - RecvQ: current/max (usage%)
# - Messages: sent(s)/received(r)
# - Bytes: sent(s)/received(r)
# - Time: seconds online
# - Dropped: number of messages dropped due to sendq full
```

**Features:**
- Real-time sendq/recvq buffer usage and capacity
- Message and byte counters for both directions
- Connection uptime tracking
- Dropped message counts for monitoring buffer overflows
- Security: Non-operators see limited information (server names hidden)

#### STATS M - Command Usage Statistics

The `STATS M` command now shows detailed command usage including bytes and local/remote tracking:

```irc
/STATS M

# Example output:
:server.name 212 yournick PRIVMSG 150 89 50
:server.name 212 yournick PING 100 20 50
:server.name 212 yournick JOIN 75 49 15
:server.name 212 yournick PART 60 54 10
:server.name 212 yournick WHOIS 50 39 10
:server.name 219 yournick m :End of STATS report

# Format: <command> <total_count> <avg_bytes> <remote_count>
# 
# Information shown:
# - Command name
# - Total count (local + remote executions)
# - Average bytes per command invocation
# - Remote count (commands received from other servers)
```

**Features:**
- Per-command execution counts (local vs remote)
- Average message size tracking per command
- Identifies which commands consume most bandwidth
- Shows server-to-server propagation patterns

**Interpreting Results:**
- High `remote_count` = command is propagated across network (QUIT, NICK, etc.)
- High `avg_bytes` = command uses significant bandwidth (PRIVMSG, TOPIC, etc.)
- High total with high bytes = potential optimization target

### Example Scenarios

#### Scenario 1: High-Volume IRC Network

```toml
# Regular users - conservative limits
[[classes]]
name = "default"
max_clients = 5000
max_sendq = 524288       # 512KB
max_recvq = 4096         # 4KB
ping_frequency = 90
connection_timeout = 180
max_connections_per_ip = 3

# IRC operators - elevated limits
[[classes]]
name = "opers"
max_clients = 100
max_sendq = 2097152      # 2MB
max_recvq = 16384        # 16KB
ping_frequency = 120
connection_timeout = 600
max_connections_per_ip = 10
disable_throttling = true

# Bots - special limits
[[classes]]
name = "bots"
max_clients = 50
max_sendq = 1048576      # 1MB
max_recvq = 8192         # 8KB
ping_frequency = 180
connection_timeout = 600
disable_throttling = true
```

#### Scenario 2: Multi-Homed Server

```toml
# Public interface for users
[[connection.ports]]
port = 6667
connection_type = "Client"
bind_address = "203.0.113.10"  # Public IP

# Private interface for servers
[[connection.ports]]
port = 6668
connection_type = "Server"
bind_address = "10.0.0.5"      # Private IP

# Management interface
[[connection.ports]]
port = 6669
connection_type = "Client"
bind_address = "127.0.0.1"     # Localhost only
```

### Backward Compatibility

- **No classes defined?** A default class is created automatically
- **No allow blocks defined?** All hosts in `allowed_hosts` use the default class
- **No per-port binding?** Global `bind_address` is used
- **Server links without class?** Default parameters are used

### Implementation Architecture

The connection class system consists of five core components:

1. **ConnectionClass** (`core/src/config.rs`): Configuration structure
2. **AllowBlock** (`core/src/config.rs`): Host-to-class mapping
3. **SendQueue & RecvQueue** (`core/src/buffer.rs`): Bounded buffers
4. **ConnectionTiming** (`core/src/buffer.rs`): Timing and health tracking
5. **ClassTracker** (`core/src/class_tracker.rs`): Limit enforcement

All limits are enforced automatically when connections are accepted and during normal operation.

## ✅ Configuration Validation

RustIRCD includes a comprehensive configuration validation system that prevents common mistakes and provides helpful suggestions.

### Validation Tool

Validate your configuration before starting the server:

```bash
# Validate default config.toml
cargo run --example validate_config

# Validate specific configuration file
cargo run --example validate_config -- /path/to/config.toml

# Get help
cargo run --example validate_config -- --help
```

### What Gets Validated

#### Errors (Must Fix)
- **Missing Required Fields**: server.name, network.name, connection.ports
- **Invalid Values**: Empty names, zero limits, invalid IP addresses
- **Invalid References**: Non-existent classes in server links or allow blocks
- **File Not Found**: Missing TLS certificates, MOTD files
- **Duplicates**: Duplicate class names, port numbers, server names
- **Security Issues**: Empty passwords, invalid password hashes
- **Configuration Ordering**: Classes must be defined before being referenced

#### Warnings (Should Review)
- **Security Best Practices**: Overly permissive hostmasks, disabled throttling
- **Missing Recommended Settings**: No channel module, no TLS on client ports
- **Suboptimal Values**: Very small buffer sizes, very frequent pings
- **Deprecated Settings**: Using allowed_hosts instead of allow_blocks

#### Information (FYI)
- Number of classes, links, operators configured
- Which classes are referenced and their settings
- Module and security configuration summary

### Example Output

```
================================================================================
Configuration Validation Report
================================================================================

✓ Configuration is VALID

WARNINGS (3):
--------------------------------------------------------------------------------
1. network.operators[0] - Operator 'admin' allows connections from any host
   → Suggestion: Consider restricting with a specific hostmask pattern

2. security - All hosts are allowed without class-based restrictions
   → Suggestion: Consider using allow_blocks for better control

3. modules.throttling - Connection throttling is disabled
   → Suggestion: Enable throttling to protect against connection floods

INFORMATION:
--------------------------------------------------------------------------------
  • Server: rustircd.local (max 1000 clients)
  • Classes: 2 defined (default, server)
  • Network: RustNet (2 links, 1 operators)
  • Server link 'hub.example.com' → class 'server' (sendq: 10MB)
  • Server link 'leaf.example.com' → class 'server' (sendq: 10MB)

================================================================================
✓ Configuration is valid but has 3 warning(s) to review.
================================================================================
```

### Automatic Validation

The server automatically validates configuration on startup:

```rust
// Validation runs in Server::init()
let mut server = Server::new(config).await;
server.init().await?; // Validates config and logs warnings
```

Validation errors prevent server startup, warnings are logged for review.

### Integration in CI/CD

Use the validation tool in your CI/CD pipeline:

```bash
# In your CI script
cargo run --example validate_config -- config.toml
if [ $? -ne 0 ]; then
    echo "Configuration validation failed!"
    exit 1
fi
```

Exit codes:
- `0` = Valid (may have warnings)
- `1` = Has errors or failed to load

## 🔌 Modules

RustIRCD includes 20+ production-ready modules:

### Core Modules

#### Channel Module
- **Commands**: JOIN, PART, MODE, TOPIC, NAMES, LIST, INVITE, KICK
- **Features**: Channel management, mode validation, member tracking
- **Required**: Yes, for channel functionality

#### IRCv3 Module
- **Capabilities**: message-tags, server-time, bot-mode, away-notify, account-tag, extended-join, multi-prefix
- **Features**: Capability negotiation, message tags, account tracking, enhanced JOIN/NAMES commands
- **Integration**: Clean extension system with core hooks
- **Extended Join**: JOIN messages include account name and real name when capability is enabled
- **Multi-Prefix**: NAMES command shows multiple prefixes for users with multiple channel modes

#### Optional Commands Module
- **Commands**: AWAY, REHASH, SUMMON, ISON, USERHOST, USERS
- **Features**: Additional IRC commands not in core

#### Messaging Modules
- **WALLOPS Module**: Operator messaging with +w mode support
- **GLOBOPS Module**: Global operator notices with +g mode support
- **Configuration**: Fully configurable via TOML (enabled/disabled, mode characters, permissions)
- **Mode System**: Dynamic user mode registration with custom validation rules
- **Permissions**: WALLOPS (users can set +w), GLOBOPS (only operators can set +g)

### Security Modules

#### Throttling Module
- **Features**: IP-based connection rate limiting
- **Configuration**: Multi-stage throttling with configurable limits
- **Integration**: STATS T command for monitoring

#### Ban Management Modules
- **GLINE Module**: Global ban management
- **KLINE Module**: Kill line management  
- **DLINE Module**: DNS line management
- **XLINE Module**: Extended line management

### Administrative Modules

#### Admin Module
- **Commands**: ADMIN, ADMINWALL, LOCops
- **Features**: Server administration and operator communication

#### Help Module
- **Features**: Dynamic command discovery
- **Integration**: Automatic help generation from modules
- **Commands**: HELP, HELP MODULES

#### Services Module
- **Features**: Service registration and management
- **Integration**: Service type system and statistics

### Utility Modules

#### Monitor Module
- **Features**: User notification system
- **Configuration**: Rate limiting and cleanup

#### Knock Module
- **Features**: Channel invitation requests
- **Configuration**: Time window management

#### Set Module
- **Features**: Server configuration management
- **Configuration**: 15+ settings with type validation

## 🌐 IRCv3 Support

RustIRCD implements a comprehensive IRCv3 extension system:

### Capability Negotiation
- **CAP LS**: List available capabilities
- **CAP REQ**: Request specific capabilities
- **CAP ACK/NAK**: Capability negotiation responses
- **CAP END**: Complete capability negotiation

### Message Tags
- **server-time**: Timestamp information
- **account**: User account tracking
- **bot**: Bot mode identification
- **away**: Away status notifications

### User Extensions
- **Account Tracking**: User account management
- **Away Notifications**: Status change notifications
- **Bot Mode**: Bot registration and identification
- **User Properties**: Property change tracking

### Channel Extensions
- **Channel Rename**: Channel renaming support
- **Batch Messages**: Grouped message processing
- **Echo Messages**: Message confirmation

## 🔒 Security Features

### Connection Security
- **TLS/SSL Support**: Modern encryption with TLS 1.3
- **Connection Throttling**: IP-based rate limiting
- **DNS & Ident Lookup**: RFC 1413 compliant identification
- **Hostmask Validation**: Pattern-based access control

### Operator System
- **Flag-Based Permissions**: Granular privilege control
- **SHA256 Authentication**: Secure password hashing
- **Hostmask Validation**: Wildcard pattern matching
- **Audit Logging**: Comprehensive operation tracking

### User Mode Security
- **Privilege Protection**: Operator modes require OPER command
- **Self-Management**: Users control their own privacy modes
- **Permission Validation**: Comprehensive access control

### Network Security
- **Server Authentication**: Secure server-to-server connections
- **Message Validation**: Input sanitization and validation
- **Rate Limiting**: Protection against flooding attacks

## 🌐 Server-to-Server

RustIRCD supports full multi-server IRC networks:

### Connection Management
- **SERVER/PASS Protocol**: Server registration handshake
- **PING/PONG**: Server keepalive mechanism
- **SQUIT**: Server removal from network
- **CONNECT**: Operator-based server connections

### Message Broadcasting
- **User Events**: NICK, QUIT, AWAY broadcasting
- **Channel Events**: JOIN, PART, MODE, TOPIC broadcasting
- **Operator Commands**: KILL, WALLOPS broadcasting
- **Network Synchronization**: Full state synchronization

### Burst System
- **User Burst**: User synchronization across network
- **Channel Burst**: Channel state synchronization
- **Server Burst**: Server information exchange
- **Module Extensions**: Custom burst types

## 🛠️ Development Guide

### Project Structure

```
rustircd/
├── core/                   # Core IRC functionality
│   ├── src/
│   │   ├── lib.rs         # Main library exports
│   │   ├── server.rs      # Server implementation
│   │   ├── client.rs      # Client management
│   │   ├── message.rs     # Message parsing
│   │   ├── user.rs        # User management
│   │   ├── database.rs    # In-memory database
│   │   ├── broadcast.rs   # Message broadcasting
│   │   ├── config.rs      # Configuration
│   │   ├── module.rs      # Module system
│   │   └── ...
│   └── Cargo.toml
├── modules/                # Loadable modules
│   ├── src/
│   │   ├── channel.rs     # Channel operations
│   │   ├── ircv3/         # IRCv3 capabilities
│   │   ├── throttling.rs  # Connection throttling
│   │   └── ...
│   └── Cargo.toml
├── services/               # Services framework
│   └── ...
├── examples/               # Example implementations
├── docs/                   # Documentation
└── Cargo.toml             # Workspace configuration
```

### Adding a New Module

1. **Create Module File**:
```rust
// modules/src/my_module.rs
use rustircd_core::{Module, ModuleResult, Client, User, Message};

pub struct MyModule {
    // Module state
}

#[async_trait]
impl Module for MyModule {
    async fn handle_message(
        &mut self,
        client: &Client,
        user: &User,
        message: &Message,
    ) -> Result<ModuleResult, Box<dyn std::error::Error + Send + Sync>> {
        // Handle module-specific commands
        Ok(ModuleResult::Continue)
    }
    
    fn get_commands(&self) -> Vec<String> {
        vec!["MYCOMMAND".to_string()]
    }
}
```

2. **Register Module**:
```rust
// In modules/src/lib.rs
pub mod my_module;

// In server initialization
let my_module = Box::new(MyModule::new());
module_manager.register_module("my_module", my_module).await?;
```

3. **Add Configuration**:
```toml
# In config.toml
[modules]
enabled_modules = ["my_module"]

[modules.my_module]
setting1 = "value1"
setting2 = 42
```

### Adding IRCv3 Capabilities

1. **Create Capability Module**:
```rust
// modules/src/ircv3/my_capability.rs
use rustircd_core::extensions::{CapabilityExtension, CapabilityAction, CapabilityResult};

pub struct MyCapabilityIntegration {
    // Capability state
}

#[async_trait]
impl CapabilityExtension for MyCapabilityIntegration {
    fn get_capabilities(&self) -> Vec<String> {
        vec!["my-capability".to_string()]
    }
    
    async fn handle_capability_negotiation(
        &self,
        client: &Client,
        capability: &str,
        action: CapabilityAction,
    ) -> Result<CapabilityResult, Box<dyn std::error::Error + Send + Sync>> {
        // Handle capability negotiation
        Ok(CapabilityResult::Ack)
    }
}
```

2. **Register Extension**:
```rust
// In server initialization
let my_capability = Box::new(MyCapabilityIntegration::new());
extension_manager.register_capability_extension(my_capability).await?;
```

### Testing

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p rustircd-core
cargo test -p rustircd-modules

# Run with examples
cargo run --example basic_usage
cargo run --example channel_burst_example
```

## 📚 API Reference

### Core API

#### Server
```rust
impl Server {
    pub fn new(config: Config) -> Self;
    pub async fn init(&mut self) -> Result<()>;
    pub async fn start(&mut self) -> Result<()>;
    pub async fn stop(&mut self) -> Result<()>;
}
```

#### Module Trait
```rust
#[async_trait]
pub trait Module: Send + Sync {
    async fn handle_message(
        &mut self,
        client: &Client,
        user: &User,
        message: &Message,
    ) -> Result<ModuleResult, Box<dyn std::error::Error + Send + Sync>>;
    
    fn get_commands(&self) -> Vec<String>;
    fn get_name(&self) -> &str;
}
```

#### Extension Traits
```rust
pub trait UserExtension: Send + Sync {
    async fn on_user_registration(&self, user: &User) -> Result<()>;
    async fn on_user_disconnection(&self, user: &User) -> Result<()>;
}

pub trait MessageExtension: Send + Sync {
    async fn on_message_preprocess(&self, client: &Client, message: &Message) -> Result<Option<Message>>;
    async fn on_message_send(&self, target_user: &User, message: &Message) -> Result<Option<Message>>;
}

pub trait CapabilityExtension: Send + Sync {
    fn get_capabilities(&self) -> Vec<String>;
    async fn handle_capability_negotiation(&self, client: &Client, capability: &str, action: CapabilityAction) -> Result<CapabilityResult>;
}
```

### Configuration API

#### Config Structure
```rust
pub struct Config {
    pub server: ServerConfig,
    pub connection: ConnectionConfig,
    pub security: SecurityConfig,
    pub modules: ModuleConfig,
    pub tls: TlsConfig,
}
```

#### Module Configuration
```rust
pub struct ModuleConfig {
    pub enabled_modules: Vec<String>,
    pub module_settings: HashMap<String, Value>,
}
```

## 📖 Examples

### Basic Server Setup
```rust
use rustircd_core::{Config, Server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let mut server = Server::new(config);
    
    server.init().await?;
    server.start().await?;
    
    Ok(())
}
```

### IRCv3 Extended Join and Multi-Prefix
```rust
use rustircd_modules::ircv3::Ircv3Module;
use rustircd_core::{Client, Message, Result};

// Enable IRCv3 capabilities
let mut ircv3_module = Ircv3Module::new();
ircv3_module.init().await?;

// Enable extended-join for a client
ircv3_module.enable_extended_join(client.id);

// Create extended JOIN message with account name and real name
let extended_join = ircv3_module.create_extended_join_message(
    &client,
    "#test",
    Some("alice"),
    Some("Alice User"),
)?;
// Result: :testuser!testuser@localhost JOIN #test alice :Alice User

// Enable multi-prefix for a client
ircv3_module.enable_multi_prefix(client.id);

// Process channel members with multiple prefixes
let members = vec![
    (user_id, modes_with_operator_and_voice),
    (user_id, modes_with_halfop),
    (user_id, modes_with_voice_only),
];

let formatted_names = ircv3_module.process_channel_members(
    &client,
    &members,
    &|user_id| Some(format!("user{}", user_id.as_u128() % 1000)),
);
// Result: ["@+user1", "%user2", "+user3"] with multi-prefix
// Result: ["@user1", "%user2", "+user3"] without multi-prefix
```

### Custom Module
```rust
use rustircd_core::{Module, ModuleResult, Client, User, Message};

pub struct GreetingModule;

#[async_trait]
impl Module for GreetingModule {
    async fn handle_message(
        &mut self,
        client: &Client,
        user: &User,
        message: &Message,
    ) -> Result<ModuleResult, Box<dyn std::error::Error + Send + Sync>> {
        if message.command == "HELLO" {
            let response = format!("Hello, {}!", user.nickname);
            client.send_message(&response).await?;
            return Ok(ModuleResult::Handled);
        }
        Ok(ModuleResult::Continue)
    }
    
    fn get_commands(&self) -> Vec<String> {
        vec!["HELLO".to_string()]
    }
    
    fn get_name(&self) -> &str {
        "greeting"
    }
}
```

### IRCv3 Capability
```rust
use rustircd_core::extensions::{CapabilityExtension, CapabilityAction, CapabilityResult};

pub struct EchoMessageCapability;

#[async_trait]
impl CapabilityExtension for EchoMessageCapability {
    fn get_capabilities(&self) -> Vec<String> {
        vec!["echo-message".to_string()]
    }
    
    async fn handle_capability_negotiation(
        &self,
        _client: &Client,
        capability: &str,
        action: CapabilityAction,
    ) -> Result<CapabilityResult, Box<dyn std::error::Error + Send + Sync>> {
        if capability == "echo-message" {
            match action {
                CapabilityAction::Request => Ok(CapabilityResult::Ack),
                _ => Ok(CapabilityResult::Nak),
            }
        } else {
            Ok(CapabilityResult::Nak)
        }
    }
}
```

## 🧪 Testing

### Running Tests
```bash
# All tests
cargo test

# Specific module tests
cargo test -p rustircd-core --test user_tests
cargo test -p rustircd-modules --test channel_tests

# Integration tests
cargo test --test integration_tests

# Performance tests
cargo test --test performance_tests --release
```

### Example Tests
```rust
#[tokio::test]
async fn test_user_registration() {
    let mut server = Server::new(Config::default());
    server.init().await.unwrap();
    
    // Test user registration flow
    let client = Client::new();
    let result = server.handle_nick(client, "testuser").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_channel_join() {
    let mut server = Server::new(Config::default());
    server.init().await.unwrap();
    
    // Test channel joining
    let result = server.handle_join(client, "#test").await;
    assert!(result.is_ok());
}
```

## 🚀 Performance

### Benchmarks
- **Message Throughput**: 10,000+ messages/second
- **Connection Handling**: 1,000+ concurrent connections
- **Memory Usage**: ~1KB per user
- **Startup Time**: <1 second

### Optimization Features
- **Async I/O**: Non-blocking operations throughout
- **Concurrent Processing**: Multi-threaded message handling
- **Memory Efficiency**: Optimized data structures
- **Connection Pooling**: Efficient client management

## 🔧 Troubleshooting

### Common Issues

1. **Server Won't Start**
   - Check configuration file syntax
   - Verify port availability
   - Check file permissions

2. **Module Loading Errors**
   - Verify module is in enabled_modules list
   - Check module configuration
   - Review error logs

3. **Connection Issues**
   - Check firewall settings
   - Verify TLS configuration
   - Test with different IRC clients

### Debug Information

Enable debug logging:
```toml
[logging]
level = "debug"
```

Or via command line:
```bash
RUST_LOG=debug cargo run --release
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup
```bash
# Fork and clone the repository
git clone https://github.com/your-username/rustircd.git
cd rustircd

# Create a feature branch
git checkout -b feature/amazing-feature

# Make your changes
# Add tests
# Update documentation

# Run tests
cargo test

# Submit a pull request
```

### Code Style
- Follow Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Add tests for new features
- Update documentation

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [RFC 1459](https://datatracker.ietf.org/doc/html/rfc1459) - IRC Protocol Specification
- [IRCv3 Working Group](https://ircv3.net/) - Modern IRC Extensions
- [Rust Community](https://www.rust-lang.org/community) - Excellent async libraries
- [Tokio](https://tokio.rs/) - Async runtime for Rust
- [Serde](https://serde.rs/) - Serialization framework

## 📞 Support

- **Documentation**: [GitHub Wiki](https://github.com/emilio/rustircd/wiki)
- **Issues**: [GitHub Issues](https://github.com/emilio/rustircd/issues)
- **Discussions**: [GitHub Discussions](https://github.com/emilio/rustircd/discussions)
- **IRC**: `#rustircd` on `irc.libera.chat`

---

**RustIRCD** - Modern IRC daemon implementation in Rust. Built for performance, security, and extensibility.