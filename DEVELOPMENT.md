# RustIRCd Development Guide

## 🚀 **Quick Start**

### Prerequisites
- Rust 1.70+ with `cargo`
- Git for version control

### Setup on New Machine
```bash
# Clone the repository
git clone <your-repo-url> rustircd
cd rustircd

# Install dependencies
cargo check

# Run tests
cargo test

# Build the project
cargo build
```

## 📁 **Project Structure**

```
rustircd/
├── core/                   # Core IRC daemon functionality
│   ├── src/
│   │   ├── lib.rs         # Main library exports
│   │   ├── server.rs      # Main server implementation
│   │   ├── extensions.rs  # IRCv3 extension framework
│   │   ├── database.rs    # In-memory database
│   │   ├── broadcast.rs   # Message broadcasting
│   │   └── ...
│   └── Cargo.toml
├── modules/                # Loadable modules
│   ├── src/
│   │   ├── channel.rs     # Channel operations
│   │   ├── ircv3/         # IRCv3 capabilities
│   │   └── ...
│   └── Cargo.toml
├── services/               # Services framework
│   └── ...
├── examples/               # Example implementations
├── docs/                   # Documentation
└── config_example.toml     # Example configuration
```

## 🔧 **Configuration**

### Multi-Port Configuration
The daemon supports listening on multiple ports simultaneously with different configurations:

```toml
[connection]
bind_address = "0.0.0.0"
connection_timeout = 60
ping_timeout = 300

# Configure multiple ports
[[connection.ports]]
port = 6667
connection_type = "Client"
tls = false
description = "Standard IRC port"

[[connection.ports]]
port = 6697
connection_type = "Client"
tls = true
description = "Secure IRC port"

[[connection.ports]]
port = 6668
connection_type = "Server"
tls = false
description = "Server connections"
```

#### Port Configuration Options:
- `port`: Port number to listen on
- `connection_type`: `Client`, `Server`, or `Both`
- `tls`: Enable/disable TLS for this port
- `description`: Optional description for logging

### Configuration Validation
The daemon validates configuration on startup:
- Ensures at least one port is configured
- Checks for duplicate ports
- Validates TLS settings for TLS-enabled ports
- Verifies certificate files exist when TLS is enabled

### Configurable Replies
The daemon supports customizable IRC numeric replies through a `replies.toml` file:

```toml
[server]
name = "MyIRCd"
version = "1.0.0"
description = "My Custom IRC Daemon"

[replies.001]
code = 001
text = "Welcome to {server_name}, {nick}! You are now connected! 🚀"
description = "RPL_WELCOME - Custom welcome message"

[replies.433]
code = 433
text = "{nick} :That nickname is already taken! Try {nick}_ or {nick}2"
description = "ERR_NICKNAMEINUSE - Helpful nickname suggestion"
```

#### Available Placeholders
- **Server**: `{server_name}`, `{server_version}`, `{server_description}`
- **User**: `{nick}`, `{user}`, `{host}`, `{realname}`, `{target}`
- **Channel**: `{channel}`, `{topic}`, `{reason}`, `{count}`, `{info}`
- **Custom**: `{param0}`, `{param1}`, etc.

#### Implementation Details
- Replies are loaded automatically if `replies.toml` exists
- Falls back to RFC 1459 defaults for missing replies
- Template processing with placeholder substitution
- Full coverage of all 100+ numeric replies

## 🔧 **Development Workflow**

### Building
```bash
# Check all crates
cargo check

# Build specific crate
cargo build -p rustircd-core
cargo build -p rustircd-modules

# Build everything
cargo build
```

### Testing
```bash
# Test all crates
cargo test

# Test specific crate
cargo test -p rustircd-core
```

### Running
```bash
# Run with example config
cargo run -- --config config_example.toml

# Run with debug logging
RUST_LOG=debug cargo run
```

## 📋 **Current Status**

### ✅ **Completed Features**
- [x] Modular architecture with core/modules separation
- [x] IRCv3 extension framework with hooks into core
- [x] Bot mode capability with WHOIS integration
- [x] In-memory database for users, servers, channels
- [x] Efficient broadcasting system with priority queues
- [x] Network-wide query system for distributed IRC
- [x] Extension traits for User, Message, Capability, MessageTag
- [x] Configurable numeric replies system with TOML configuration
- [x] Complete documentation and examples

### 🚧 **In Progress**
- [ ] Fix remaining 11 compilation errors (mostly connection.rs)
- [ ] Complete channel module implementation
- [ ] Implement remaining IRCv3 capabilities

### 📅 **Next Steps**
1. Fix compilation issues
2. Complete core IRC commands (PRIVMSG, NOTICE, etc.)
3. Implement server-to-server connections with TLS
4. Add DNS and ident lookup functionality
5. Implement SASL authentication

## 🏗️ **Architecture Notes**

### Module System
- **Core**: Minimal, handles networking and basic IRC protocol
- **Modules**: Optional features loaded via configuration
- **Extensions**: IRCv3 capabilities with clean hooks into core
- **Services**: Separate framework for network services

### Key Design Principles
- **Modularity**: Features are optional and loadable
- **Extensibility**: Easy to add new capabilities without core changes
- **Performance**: Efficient message routing and broadcasting
- **Standards Compliance**: RFC 1459 + IRCv3 specifications

## 🔌 **Adding New Features**

### Adding a New IRCv3 Capability
1. Create capability file in `modules/src/ircv3/`
2. Implement extension traits in `core_integration.rs`
3. Register with ExtensionManager
4. Update capability list

### Adding a New Module
1. Create module file in `modules/src/`
2. Implement `Module` trait
3. Add to module loading configuration
4. Define module-specific numeric replies

## 🐛 **Known Issues**

### Compilation Errors (11 remaining)
- Connection trait issues in `core/src/connection.rs`
- Format string issue in `core/src/user.rs` (keeps reverting)
- Some async trait bound issues

### TODO Items
- Complete channel command implementations
- Add proper error handling throughout
- Implement TLS support
- Add configuration validation
