# Rust IRC Daemon (rustircd)

A modular IRC daemon implementation in Rust based on RFC 1459 and IRCv3 specifications.

## Features

- **Modular Architecture**: Core functionality is minimal, with features loaded via modules
- **RFC 1459 Compliance**: Implements the core IRC protocol as specified in RFC 1459
- **IRCv3 Support**: Supports IRCv3 extensions including capability negotiation, SASL, and more
- **TLS/SSL Support**: Secure connections with TLS encryption
- **Dynamic Module Loading**: Load and unload modules at runtime
- **Services Framework**: Extensible framework for network-specific services
- **Configurable Replies**: Customize IRC numeric replies with placeholders and personalization
- **High Performance**: Built with async Rust for excellent performance

## Architecture

The daemon is split into three main components:

- **Core** (`core/`): Essential IRC functionality including message parsing, client management, and basic commands
- **Modules** (`modules/`): Optional features like channel operations, IRCv3 support, and additional commands
- **Services** (`services/`): Network-specific services and bots

## Core Commands

The core implements the following RFC 1459 command categories:

- Connection registration commands (PASS, NICK, USER, etc.)
- Server queries and commands (VERSION, STATS, LINKS, etc.)
- Sending messages (PRIVMSG, NOTICE)
- User-based queries (WHO, WHOIS, WHOWAS)
- Miscellaneous messages (PING, PONG, QUIT, etc.)
- Server-to-server connections with TLS encryption
- Nickname/user tracking and management
- Client connection handling
- DNS and ident lookups
- Configuration file handling
- IRC Operator messages
- Super server (u-lined) support

**Note**: Channel operations are handled by the channel module and are not available without it loaded.

## Modules

### Channel Module
- Channel operations (JOIN, PART, MODE, TOPIC, etc.)
- Channel management and tracking
- Channel modes and permissions
- Channel-specific error and reply messages
- **Required for channel support** - without this module, the daemon has no channel functionality

### IRCv3 Module
- Capability negotiation
- SASL authentication (3.1 and 3.2)
- Message tags
- Account tracking
- Away notifications
- Batch messages
- Bot mode registration
- Channel renaming
- User property changes

### Optional Commands Module
- AWAY command
- REHASH command
- SUMMON command
- ISON command
- OPERWALL command
- WALLOPS command
- USERHOST command
- USERS command

## Services Framework

The services framework allows for network-specific functionality:

- Custom service commands
- Bot implementations
- Network-specific protocols
- Database integration
- External API integration

## Super Servers (U-lined)

The daemon supports super servers (u-lined servers) which have elevated privileges:

- Configured in the `[network.super_servers]` section
- Can perform administrative operations
- Messages from super servers are handled with special privileges
- Useful for services, bots, and administrative tools

## Configurable Replies

RustIRCd supports customizable IRC numeric replies, allowing server administrators to personalize messages while maintaining RFC 1459 compliance:

### Features
- **Template System**: Use placeholders like `{nick}`, `{server_name}`, `{channel}` for dynamic content
- **Complete Coverage**: All 100+ RFC 1459 numeric replies can be customized
- **Fallback Safety**: Gracefully falls back to defaults for missing replies
- **Easy Configuration**: Simple TOML format with comprehensive examples

### Quick Start
1. Create a `replies.toml` file in your server directory
2. Customize any numeric reply:
```toml
[replies.001]
code = 001
text = "Welcome to {server_name}, {nick}! You are now connected! 🚀"
description = "RPL_WELCOME - Custom welcome message"
```
3. Restart the server to load custom replies

### Available Placeholders
- **Server**: `{server_name}`, `{server_version}`, `{server_description}`
- **User**: `{nick}`, `{user}`, `{host}`, `{realname}`, `{target}`
- **Channel**: `{channel}`, `{topic}`, `{reason}`, `{count}`, `{info}`
- **Custom**: `{param0}`, `{param1}`, etc.

See `CONFIGURABLE_REPLIES.md` for complete documentation and examples.

## Installation

1. Clone the repository:
```bash
git clone https://github.com/emilio/rustircd.git
cd rustircd
```

2. Build the project:
```bash
cargo build --release
```

3. Generate a configuration file:
```bash
cargo run -- config
```

4. Edit the configuration file as needed:
```bash
nano config.toml
```

5. Run the daemon:
```bash
cargo run --release
```

## Configuration

The daemon uses TOML configuration files. A default configuration is generated with:

```bash
cargo run -- config
```

### Configuration Files

- **`config.toml`**: Main server configuration
- **`replies.toml`**: Optional custom numeric replies (auto-loaded if present)

### Multi-Port Configuration

The daemon supports listening on multiple ports simultaneously, each with different configurations:

- **Port Types**: Each port can be configured for `Client`, `Server`, or `Both` connection types
- **TLS Support**: Individual ports can have TLS enabled or disabled
- **Flexible Setup**: Mix and match secure and non-secure ports as needed
- **Connection Limits**: Global limits apply across all ports

#### Port Configuration Options

- `port`: The port number to listen on
- `connection_type`: Type of connections allowed (`Client`, `Server`, or `Both`)
- `tls`: Whether to use TLS encryption for this port
- `description`: Optional description for documentation purposes

### Basic Configuration

```toml
[server]
name = "localhost"
description = "Rust IRC Daemon"
version = "0.1.0"
max_clients = 1000
max_channels_per_client = 10

[connection]
bind_address = "0.0.0.0"
connection_timeout = 60
ping_timeout = 300
max_connections_per_ip = 5
max_connections_per_host = 10

# Configure multiple ports with different connection types and TLS settings
[[connection.ports]]
port = 6667
connection_type = "Client"
tls = false
description = "Standard IRC port"

[[connection.ports]]
port = 6668
connection_type = "Server"
tls = false
description = "Server-to-server connections"

[[connection.ports]]
port = 6697
connection_type = "Client"
tls = true
description = "Secure IRC port"

[[connection.ports]]
port = 6698
connection_type = "Server"
tls = true
description = "Secure server-to-server connections"

[security]
require_client_password = false
enable_ident = true
enable_dns = true

[security.tls]
enabled = false
cert_file = "cert.pem"
key_file = "key.pem"
```

## Usage

### Starting the Server

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

### Command Line Options

- `--config, -c`: Configuration file path (default: config.toml)
- `--log-level, -l`: Log level (trace, debug, info, warn, error)
- `--daemon, -d`: Run in background
- `--test-config`: Test configuration and exit

### Subcommands

- `config`: Generate default configuration file
- `info`: Show server information
- `version`: Show version information

## Development

### Project Structure

```
rustircd/
├── core/                 # Core IRC functionality
│   ├── src/
│   │   ├── lib.rs       # Core library
│   │   ├── client.rs    # Client management
│   │   ├── server.rs    # Server implementation
│   │   ├── message.rs   # Message parsing
│   │   ├── user.rs      # User management
│   │   ├── channel.rs   # Channel management
│   │   ├── config.rs    # Configuration
│   │   ├── module.rs    # Module system
│   │   └── utils.rs     # Utilities
│   └── Cargo.toml
├── modules/              # Optional modules
│   ├── src/
│   │   ├── lib.rs
│   │   ├── channel.rs   # Channel operations
│   │   ├── ircv3.rs     # IRCv3 support
│   │   └── optional.rs  # Optional commands
│   └── Cargo.toml
├── services/             # Services framework
│   ├── src/
│   │   ├── lib.rs
│   │   ├── framework.rs # Service framework
│   │   └── example.rs   # Example service
│   └── Cargo.toml
├── src/
│   └── main.rs          # Main binary
├── Cargo.toml           # Workspace configuration
└── README.md
```

### Adding a New Module

1. Create a new module file in `modules/src/`
2. Implement the `Module` trait
3. Add the module to `modules/src/lib.rs`
4. Register the module in the server

### Adding a New Service

1. Create a new service file in `services/src/`
2. Implement the `Service` trait
3. Add the service to `services/src/lib.rs`
4. Register the service in the server

## Testing

Run the test suite:

```bash
cargo test
```

Run tests for a specific component:

```bash
cargo test -p rustircd-core
cargo test -p rustircd-modules
cargo test -p rustircd-services
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run the test suite
6. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- RFC 1459 for the IRC protocol specification
- IRCv3 working group for modern IRC extensions
- The Rust community for excellent async libraries