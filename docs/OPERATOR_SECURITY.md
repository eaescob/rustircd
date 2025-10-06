# Operator Security Implementation

This document explains how operator privileges are secured in the Rust IRC Daemon to prevent unauthorized access.

## Security Layers

### 1. User Mode Protection

The `+o` (operator) mode is protected at multiple levels:

#### User Mode Definition
```rust
pub enum UserMode {
    Operator,        // 'o' - Global operator
    LocalOperator,   // 'O' - Local operator
    // ... other modes
}
```

#### Mode Validation
- `oper_only()`: Returns `true` for operator modes, preventing them from being set via MODE command
- `requires_operator()`: Returns `true` for operator modes, requiring operator privileges to modify

### 2. MODE Command Protection

The MODE command handler includes multiple security checks:

```rust
// Special case: Operator mode can only be granted through OPER command
if adding && mode.oper_only() {
    return Err("Operator mode can only be granted through OPER command".into());
}
```

This prevents clients from using `MODE user +o` to grant themselves operator privileges.

### 3. User Object Protection

The `User` struct has protected methods for mode manipulation:

#### Public Methods (Client-facing)
```rust
pub fn add_mode(&mut self, mode: char) {
    // Prevent clients from setting operator mode directly
    if mode == 'o' {
        tracing::warn!("Attempted to set operator mode 'o' directly");
        return;
    }
    self.modes.insert(mode);
}
```

#### Internal Methods (Server-only)
```rust
pub fn add_mode_internal(&mut self, mode: char) {
    // Bypasses security checks for internal server use
    self.modes.insert(mode);
}

pub fn grant_operator_privileges(&mut self, flags: HashSet<OperatorFlag>) {
    // Only way to legitimately set operator mode
    self.set_operator_flags(flags);
    // Automatically sets +o mode via add_mode_internal
}
```

### 4. OPER Command Authentication

The OPER command is the only legitimate way to gain operator privileges:

```rust
// In OperModule::handle_oper()
if let Some(operator_config) = config.authenticate_operator(oper_name, password, user, host) {
    // Grant operator privileges (this will set the +o mode securely)
    if let Some(mut user) = client.user.as_ref() {
        user.grant_operator_privileges(operator_flags.clone());
    }
}
```

## Security Flow

1. **Client attempts MODE +o**: Blocked by `oper_only()` check
2. **Client attempts direct mode manipulation**: Blocked by `add_mode()` protection
3. **Client uses OPER command**: 
   - Authenticates against configured operators
   - Verifies password and hostmask
   - Only then grants privileges via `grant_operator_privileges()`

## Configuration

Operators are defined in the configuration file:

```toml
[network]
operators = [
    { 
        nickname = "admin", 
        password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8", 
        hostmask = "*@*", 
        flags = ["GlobalOper", "Administrator"], 
        enabled = true 
    }
]
```

## Operator Flags

The system supports granular operator privileges:

```rust
pub enum OperatorFlag {
    GlobalOper,      // Can operate on any server
    LocalOper,       // Can only operate on local server
    RemoteConnect,   // Can use CONNECT command
    LocalConnect,    // Can use local CONNECT
    Administrator,   // Administrator privileges
    Spy,            // Spy privileges
    Squit,          // Can use SQUIT command
}
```

## Monitoring and Logging

All operator actions are logged:

```rust
if self.config.log_operator_actions {
    tracing::info!("User {} authenticated as operator with flags: {:?}", 
        oper_name, operator_flags);
}
```

## Testing Security

To verify the security implementation:

1. **Test MODE command**: Try `MODE user +o` - should be rejected
2. **Test OPER command**: Use valid credentials - should work
3. **Test invalid OPER**: Use wrong password - should fail
4. **Test hostmask**: Use wrong hostmask - should fail

## Migration from Core

When moving operator functionality from core to modules:

1. **Keep security checks in core**: Mode validation remains in core
2. **Move authentication to module**: OPER command handling moves to oper module
3. **Use secure methods**: Always use `grant_operator_privileges()` instead of direct mode manipulation
4. **Maintain logging**: Ensure all operator actions are logged

This multi-layered approach ensures that operator privileges can only be gained through proper authentication and cannot be bypassed by client manipulation.
