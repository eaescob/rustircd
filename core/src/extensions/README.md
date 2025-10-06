# Core Extensions

This directory contains core extensions inspired by Solanum IRCd. Each extension is implemented in its own file for better modularity and maintainability, following Solanum's architectural pattern.

## Extension Files

### `identify_msg.rs`
- **Purpose**: Adds account information to messages
- **Similar to**: Solanum's identify-msg extension
- **Features**:
  - Adds `account` tag to messages from identified users
  - Adds `identify-msg` tag to indicate identification status
  - Validates message tag formats

### `account_tracking.rs`
- **Purpose**: Tracks user account information and identification status
- **Similar to**: Solanum's account-tracking extension
- **Features**:
  - Tracks account names and identification status
  - Manages account lifecycle (registration, disconnection)
  - Provides account information lookup

### `server_time.rs`
- **Purpose**: Provides server time information via message tags
- **Similar to**: Solanum's server-time extension
- **Features**:
  - Adds `time` tag to messages with server timestamp
  - Validates time tag format (RFC3339)
  - Configurable for all messages or specific types

### `batch.rs`
- **Purpose**: Handles message batching for efficient network communication
- **Similar to**: Solanum's batch extension
- **Features**:
  - Manages active batch sessions
  - Handles batch start/end commands
  - Processes batched messages efficiently

## Usage

Extensions are managed through the `CoreExtensionManager`:

```rust
use rustircd_core::extensions::CoreExtensionManager;

// Create extension manager
let core_extensions = CoreExtensionManager::new("services.example.org".to_string());

// Initialize all extensions
core_extensions.initialize().await?;

// Access specific extensions
let account_tracking = core_extensions.get_account_tracking();
let identify_msg = core_extensions.get_identify_message();
```

## Adding New Extensions

To add a new extension:

1. Create a new file in this directory (e.g., `new_extension.rs`)
2. Implement the appropriate extension traits:
   - `UserExtension` for user-related hooks
   - `MessageExtension` for message processing
   - `MessageTagExtension` for message tag handling
   - `CapabilityExtension` for capability negotiation
3. Add the module to `mod.rs`
4. Export the extension type in `mod.rs`
5. Add it to `CoreExtensionManager` if it should be auto-loaded

## Extension Traits

### UserExtension
Hooks for user-related events:
- `on_user_registration()`
- `on_user_disconnection()`
- `on_user_property_change()`
- `on_user_join_channel()`
- `on_user_part_channel()`
- `on_user_nick_change()`
- `on_user_away_change()`

### MessageExtension
Hooks for message processing:
- `on_message_preprocess()`
- `on_message_postprocess()`
- `on_message_send()`
- `on_message_broadcast()`

### MessageTagExtension
Hooks for message tag handling:
- `process_incoming_tags()`
- `generate_outgoing_tags()`
- `validate_tags()`

### CapabilityExtension
Hooks for capability negotiation:
- `get_capabilities()`
- `supports_capability()`
- `handle_capability_negotiation()`
- `on_capabilities_enabled()`
- `on_capabilities_disabled()`

## Configuration

Extensions can be configured through the main configuration file or through the extension registry system. Each extension should define its own configuration structure and provide sensible defaults.

## Testing

Each extension should include comprehensive tests. Test files should be placed in the same directory with a `_test.rs` suffix (e.g., `identify_msg_test.rs`).

## Dependencies

Extensions should minimize dependencies and only use what's necessary. Common dependencies include:
- `async_trait` for async trait implementations
- `tokio` for async primitives
- `uuid` for unique identifiers
- `chrono` for time handling
- `std::collections` for data structures

## Security Considerations

Extensions should:
- Validate all input data
- Not expose sensitive information
- Use secure defaults
- Log security-relevant events
- Follow the principle of least privilege
