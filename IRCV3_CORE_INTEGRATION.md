# IRCv3 Core Integration

This document explains how IRCv3 capabilities integrate with the core IRC daemon through a clean, extensible architecture.

## 🏗️ **Architecture Overview**

The IRCv3 integration follows a **plugin-based architecture** where:

- **Core remains simple**: Only essential IRC functionality
- **Modules provide features**: IRCv3 capabilities as optional modules
- **Extension system**: Clean hooks for module integration
- **No core modifications**: Modules extend core without changing it

## 🔌 **Extension System**

### **Core Extension Traits**

The core provides several extension points that modules can implement:

#### **1. UserExtension**
```rust
pub trait UserExtension: Send + Sync {
    async fn on_user_registration(&self, user: &User) -> Result<()>;
    async fn on_user_disconnection(&self, user: &User) -> Result<()>;
    async fn on_user_property_change(&self, user: &User, property: &str, old_value: &str, new_value: &str) -> Result<()>;
    async fn on_user_join_channel(&self, user: &User, channel: &str) -> Result<()>;
    async fn on_user_part_channel(&self, user: &User, channel: &str, reason: Option<&str>) -> Result<()>;
    async fn on_user_nick_change(&self, user: &User, old_nick: &str, new_nick: &str) -> Result<()>;
    async fn on_user_away_change(&self, user: &User, away: bool, message: Option<&str>) -> Result<()>;
}
```

#### **2. MessageExtension**
```rust
pub trait MessageExtension: Send + Sync {
    async fn on_message_preprocess(&self, client: &Client, message: &Message) -> Result<Option<Message>>;
    async fn on_message_postprocess(&self, client: &Client, message: &Message, result: &ModuleResult) -> Result<()>;
    async fn on_message_send(&self, target_user: &User, message: &Message) -> Result<Option<Message>>;
    async fn on_message_broadcast(&self, message: &Message, targets: &[Uuid]) -> Result<Option<Message>>;
}
```

#### **3. CapabilityExtension**
```rust
pub trait CapabilityExtension: Send + Sync {
    fn get_capabilities(&self) -> Vec<String>;
    fn supports_capability(&self, capability: &str) -> bool;
    async fn handle_capability_negotiation(&self, client: &Client, capability: &str, action: CapabilityAction) -> Result<CapabilityResult>;
    async fn on_capabilities_enabled(&self, client: &Client, capabilities: &[String]) -> Result<()>;
    async fn on_capabilities_disabled(&self, client: &Client, capabilities: &[String]) -> Result<()>;
}
```

#### **4. MessageTagExtension**
```rust
pub trait MessageTagExtension: Send + Sync {
    async fn process_incoming_tags(&self, client: &Client, tags: &HashMap<String, String>) -> Result<HashMap<String, String>>;
    async fn generate_outgoing_tags(&self, sender: &User, message: &Message) -> Result<HashMap<String, String>>;
    async fn validate_tags(&self, tags: &HashMap<String, String>) -> Result<()>;
}
```

## 🎯 **IRCv3 Capabilities Integration**

### **1. Capability Negotiation**
- **Core Hook**: `CapabilityExtension`
- **Module**: `capability_negotiation.rs`
- **Features**:
  - Advertise supported capabilities
  - Handle CAP LS, REQ, ACK, END commands
  - Track client capabilities
  - Enable/disable features based on capabilities

### **2. Message Tags**
- **Core Hook**: `MessageTagExtension`
- **Module**: `message_tags.rs`
- **Features**:
  - Process incoming message tags
  - Generate outgoing message tags
  - Validate tag format and content
  - Support for standard tags (time, account, bot, away)

### **3. Account Tracking**
- **Core Hook**: `UserExtension`
- **Module**: `account_tracking.rs`
- **Features**:
  - Track user account information
  - Handle account registration/disconnection
  - Provide account metadata in messages
  - Support for account-based permissions

### **4. Away Notification**
- **Core Hook**: `UserExtension`
- **Module**: `away_notification.rs`
- **Features**:
  - Track user away status
  - Notify about away status changes
  - Broadcast away notifications
  - Support for away messages

### **5. Bot Mode**
- **Core Hook**: `UserExtension` + `MessageTagExtension`
- **Module**: `bot_mode.rs`
- **Features**:
  - Bot registration and identification
  - Bot metadata (name, version, capabilities)
  - Bot message tagging
  - WHOIS bot information

### **6. Echo Message**
- **Core Hook**: `MessageExtension`
- **Module**: `echo_message.rs`
- **Features**:
  - Echo back certain message types
  - Message confirmation for clients
  - Support for echo-message capability

### **7. Server Time**
- **Core Hook**: `MessageTagExtension`
- **Module**: `server_time.rs`
- **Features**:
  - Add server-time tags to messages
  - Timestamp validation
  - Support for server-time capability

### **8. Batch Processing**
- **Core Hook**: `MessageExtension`
- **Module**: `batch.rs`
- **Features**:
  - Handle batch message processing
  - Group related messages
  - Support for batch capability

### **9. User Properties**
- **Core Hook**: `UserExtension`
- **Module**: `user_properties.rs`
- **Features**:
  - Track user property changes
  - Support for chghost capability
  - User metadata management

### **10. Channel Rename**
- **Core Hook**: `ChannelExtension`
- **Module**: `channel_rename.rs`
- **Features**:
  - Handle channel renaming
  - Update channel references
  - Support for channel-rename capability

## 🔄 **Integration Flow**

### **1. Server Initialization**
```rust
// Create server with extension manager
let server = Server::new(config);

// Register IRCv3 extensions
server.register_ircv3_extensions().await?;
```

### **2. Client Connection**
```rust
// Client connects and negotiates capabilities
Client → Server: CAP LS
Server → Client: CAP * LS :message-tags server-time bot-mode ...

Client → Server: CAP REQ :message-tags server-time
Server → Client: CAP * ACK :message-tags server-time

Client → Server: CAP END
```

### **3. Message Processing**
```rust
// Message received
let message = receive_message();

// Preprocessing hooks
let processed_message = extension_manager.on_message_preprocess(client, &message).await?;

// Process message tags
let tags = extension_manager.process_incoming_tags(client, &message.tags).await?;

// Send message
extension_manager.on_message_send(target_user, &processed_message).await?;

// Postprocessing hooks
extension_manager.on_message_postprocess(client, &message, &result).await?;
```

### **4. User Operations**
```rust
// User registration
extension_manager.on_user_registration(&user).await?;

// User property change
extension_manager.on_user_property_change(&user, "away", "false", "true").await?;

// User disconnection
extension_manager.on_user_disconnection(&user).await?;
```

## 🛠️ **Adding New IRCv3 Capabilities**

### **Step 1: Create Capability Module**
```rust
// modules/src/ircv3/new_capability.rs
pub struct NewCapabilityIntegration {
    // Capability state
}

impl UserExtension for NewCapabilityIntegration {
    // Implement required methods
}
```

### **Step 2: Register Extension**
```rust
// In server.rs
self.extension_manager.register_user_extension(
    Box::new(NewCapabilityIntegration::new())
).await?;
```

### **Step 3: Add Capability Support**
```rust
// In capability_negotiation.rs
capabilities.insert("new-capability".to_string());
```

## 📊 **Benefits of This Architecture**

### **1. Core Simplicity**
- Core remains focused on essential IRC functionality
- No IRCv3-specific code in core
- Easy to understand and maintain

### **2. Modularity**
- Each IRCv3 capability is a separate module
- Capabilities can be enabled/disabled independently
- Easy to add new capabilities

### **3. Extensibility**
- Clean extension points for integration
- No core modifications needed for new features
- Future-proof architecture

### **4. Performance**
- Extensions only loaded when needed
- Minimal overhead when capabilities disabled
- Efficient message processing

### **5. Testing**
- Each capability can be tested independently
- Core functionality isolated from extensions
- Easy to mock and test

## 🔧 **Configuration**

### **Enable/Disable Capabilities**
```toml
# config.toml
[modules]
enabled_modules = ["ircv3"]

[ircv3]
enabled_capabilities = [
    "message-tags",
    "server-time", 
    "bot-mode",
    "away-notify",
    "account-tag"
]
```

### **Capability-Specific Settings**
```toml
[ircv3.message_tags]
validate_tags = true
add_server_time = true

[ircv3.bot_mode]
require_registration = true
max_bot_name_length = 50

[ircv3.account_tracking]
track_disconnections = true
history_retention_days = 30
```

## 🎯 **Key Features**

- **Clean Architecture**: Core and modules are clearly separated
- **Extensible**: Easy to add new IRCv3 capabilities
- **Configurable**: Capabilities can be enabled/disabled
- **Performant**: Minimal overhead when capabilities disabled
- **Testable**: Each component can be tested independently
- **Future-Proof**: Architecture supports new IRCv3 specifications

This integration system ensures that the core IRC daemon remains simple and focused while providing a powerful, extensible platform for IRCv3 capabilities and future enhancements.
