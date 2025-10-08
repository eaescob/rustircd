# Account Integration Guide

## Overview

When users authenticate via SASL or services (like NickServ), the system should trigger account change notifications if the IRCv3 account-notify module is enabled. This ensures all channel members who have the capability enabled receive ACCOUNT messages when users log in.

## Integration Points

### 1. SASL Authentication

When a user successfully authenticates via SASL:

**File**: `modules/src/sasl.rs`
**Method**: `complete_authentication()`

```rust
// After SASL authentication succeeds
async fn complete_authentication(&self, client: &Client, mechanism: &dyn SaslMechanism, context: &ModuleContext) -> Result<()> {
    match mechanism.complete(client).await {
        Ok(auth_data) => {
            let account_name = auth_data.username.clone();
            
            // ... update session ...
            
            // Trigger account change notification
            self.set_user_account(client.id, &account_name, context).await?;
        }
        // ... error handling ...
    }
}
```

### 2. Services Integration (NickServ/ChanServ)

When a user identifies with NickServ or another authentication service:

**Location**: Server or Services Module

```rust
// When user sends: /msg NickServ IDENTIFY password
// Or: /msg NickServ IDENTIFY account password

async fn handle_identify(&self, user_id: Uuid, account_name: String, context: &ModuleContext) -> Result<()> {
    // 1. Validate credentials (implementation specific)
    
    // 2. Trigger IRCv3 account notification
    // This should be coordinated at the server level
    if let Some(ircv3_module) = module_manager.get_module_mut("ircv3") {
        ircv3_module.set_user_account(user_id, account_name, context).await?;
    }
    
    Ok(())
}
```

### 3. Server-Level Module Coordination

The server should coordinate between modules to enable account notifications:

**Recommended Implementation**:

```rust
// In the server's main message handling loop or module coordinator

// After SASL authentication
if let Some(sasl_module) = module_manager.get_module("sasl") {
    if let Some(account_name) = sasl_module.get_authenticated_account(user_id).await {
        // Trigger IRCv3 account notification
        if let Some(ircv3_module) = module_manager.get_module_mut("ircv3") {
            ircv3_module.set_user_account(user_id, account_name, context).await?;
        }
    }
}

// After services identification
if let Some(services_module) = module_manager.get_module("services") {
    // Hook for when services identify a user
    // services_module should expose a callback or method to get newly identified users
}
```

## IRCv3 Module API

The IRCv3 module provides these methods for account management:

### Set Account

```rust
pub async fn set_user_account(
    &mut self, 
    user_id: uuid::Uuid, 
    account_name: String, 
    context: &ModuleContext
) -> Result<()>
```

**What it does:**
1. Updates account tracking with the new account
2. Broadcasts ACCOUNT messages to all channel members
3. Only sends to users who have `account-notify` capability enabled

**Example ACCOUNT message broadcasted:**
```
:nick!user@host ACCOUNT AccountName
```

### Remove Account

```rust
pub async fn remove_user_account(
    &mut self,
    user_id: uuid::Uuid,
    context: &ModuleContext
) -> Result<Option<String>>
```

**What it does:**
1. Removes account from tracking
2. Broadcasts ACCOUNT * (logout) to channel members

**Example ACCOUNT message broadcasted:**
```
:nick!user@host ACCOUNT *
```

### Get Account

```rust
pub fn get_user_account(&self, user_id: &uuid::Uuid) -> Option<&String>
```

Returns the current account name for a user, if set.

## Account Flow

### Successful Authentication Flow

1. User connects and registers
2. User authenticates via SASL or services
3. Authentication module validates credentials
4. Authentication module calls `set_user_account()`
5. IRCv3 module updates account tracking
6. IRCv3 module broadcasts ACCOUNT message to all channel members
7. Only users with `account-notify` capability receive the message

### User Logout/Disconnect Flow

1. User disconnects or explicitly logs out
2. Server calls `remove_user_account()`
3. IRCv3 module broadcasts `ACCOUNT *` to channel members
4. Account tracking is cleaned up

## Implementation Checklist

- [x] IRCv3 account_tracking module with broadcasting
- [x] SASL module authentication hook point
- [x] IRCv3 helper methods for account management
- [ ] Services module identification hook point
- [ ] Server-level module coordination
- [ ] ACCOUNT message capability filtering
- [ ] Account persistence (optional)

## Testing

### Test SASL Authentication

```
# Client sends
CAP REQ :account-notify
AUTHENTICATE PLAIN
AUTHENTICATE base64(username\0username\0password)

# Server should broadcast to channel members:
:user!user@host ACCOUNT username
```

### Test Services Identification

```
# Client sends
/msg NickServ IDENTIFY password

# Server should broadcast to channel members:
:user!user@host ACCOUNT username
```

### Test Account Logout

```
# Client disconnects or logs out
# Server should broadcast to channel members:
:user!user@host ACCOUNT *
```

## Notes

- Account names should be case-insensitive
- Account tracking is in-memory by default (can be persisted)
- Only users with `account-notify` capability receive ACCOUNT messages
- The `*` in `ACCOUNT *` indicates no account (logged out)
- Extended join uses account information if available

## See Also

- `docs/IRCV3_MODULE_INTEGRATION.md` - Full IRCv3 integration guide
- `modules/src/ircv3/account_tracking.rs` - Account tracking implementation
- `modules/src/sasl.rs` - SASL authentication module
- IRCv3 specification: https://ircv3.net/specs/extensions/account-notify

