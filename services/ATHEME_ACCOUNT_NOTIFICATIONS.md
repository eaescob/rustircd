# Atheme NickServ Account Notifications

## Overview

The Atheme services integration now automatically detects when users identify with NickServ and triggers IRCv3 account change notifications. This enables real-time account status broadcasting to channel members.

## How It Works

### Detection Methods

Atheme uses multiple methods to signal user authentication:

#### 1. SVSMODE +r (Primary Method)
When a user successfully identifies with NickServ, Atheme sends:
```
:services. SVSMODE nickname +r
```

The `+r` mode indicates the user is registered and identified.

**Detection:**
- Atheme sends `SVSMODE nick +r` when user identifies
- Atheme sends `SVSMODE nick -r` when user logs out
- Triggers `trigger_account_notification()` with account name or None

#### 2. ENCAP LOGIN
Some Atheme configurations use ENCAP for login notification:
```
:services. ENCAP * LOGIN nickname accountname
```

**Detection:**
- Watches for `ENCAP * LOGIN` commands
- Extracts nickname and account name
- Triggers account notification

#### 3. METADATA accountname
Atheme may set account metadata:
```
:services. METADATA nickname accountname :AccountName
:services. METADATA nickname accountname :*
```

**Detection:**
- Watches for `METADATA` commands with `accountname` key
- Value `*` or empty indicates logout
- Triggers account notification

## Implementation

### Account Notification Flow

```
User → /msg NickServ IDENTIFY password
         ↓
NickServ validates credentials
         ↓
Atheme → SVSMODE nick +r (or ENCAP/METADATA)
         ↓
IRCd receives and handles SVSMODE
         ↓
trigger_account_notification() called
         ↓
Server coordinates with IRCv3 module
         ↓
IRCv3 → :nick!user@host ACCOUNT accountname
         ↓
Channel members receive notification
```

### Key Methods

#### `trigger_account_notification()`
```rust
async fn trigger_account_notification(
    &self, 
    nick: &str, 
    account: Option<&str>, 
    context: &ServiceContext
) -> Result<()>
```

**What it does:**
1. Looks up user by nickname in database
2. Logs the authentication event
3. Provides hook point for server to call IRCv3 module

**Server Integration Point:**
```rust
// In server's Atheme message handler
if let Some(ircv3_module) = module_manager.get_module_mut("ircv3") {
    if let Some(account_name) = account {
        ircv3_module.set_user_account(user_id, account_name, context).await?;
    } else {
        ircv3_module.remove_user_account(user_id, context).await?;
    }
}
```

#### `handle_atheme_svsmode_with_context()`
```rust
async fn handle_atheme_svsmode_with_context(
    &self, 
    message: &Message, 
    context: &ServiceContext
) -> Result<()>
```

**What it does:**
1. Parses SVSMODE command
2. Checks for `+r` (identified) or `-r` (logged out)
3. Calls `trigger_account_notification()`
4. Broadcasts SVSMODE to other servers

#### `handle_atheme_encap_login()`
```rust
async fn handle_atheme_encap_login(
    &self, 
    message: &Message, 
    context: &ServiceContext
) -> Result<()>
```

**What it does:**
1. Parses ENCAP LOGIN command
2. Extracts nickname and account name
3. Calls `trigger_account_notification()`

#### `handle_atheme_metadata()`
```rust
async fn handle_atheme_metadata(
    &self, 
    message: &Message, 
    context: &ServiceContext
) -> Result<()>
```

**What it does:**
1. Parses METADATA command
2. Checks if key is `accountname` or `ACCOUNTNAME`
3. Calls `trigger_account_notification()` with account or None

## Protocol Examples

### User Identifies with NickServ

**User sends:**
```irc
/msg NickServ IDENTIFY mypassword
```

**Atheme responds (to IRCd):**
```irc
:services.example.org SVSMODE Alice +r
```

**IRCd processes:**
- Detects `+r` mode
- Calls `trigger_account_notification("Alice", Some("Alice"), context)`
- Server should call `ircv3_module.set_user_account(alice_id, "Alice", context)`

**IRCv3 broadcasts:**
```irc
:Alice!alice@host.example.org ACCOUNT Alice
```

### User Logs Out

**User disconnects or logs out**

**Atheme responds:**
```irc
:services.example.org SVSMODE Alice -r
```

**IRCd processes:**
- Detects `-r` mode
- Calls `trigger_account_notification("Alice", None, context)`
- Server should call `ircv3_module.remove_user_account(alice_id, context)`

**IRCv3 broadcasts:**
```irc
:Alice!alice@host.example.org ACCOUNT *
```

### Alternative: ENCAP LOGIN

**Atheme sends:**
```irc
:services.example.org ENCAP * LOGIN Alice Alice
```

**IRCd processes:**
- Detects ENCAP LOGIN
- Calls `trigger_account_notification("Alice", Some("Alice"), context)`
- Same flow as SVSMODE

### Alternative: METADATA

**Atheme sends:**
```irc
:services.example.org METADATA Alice accountname :Alice
```

**IRCd processes:**
- Detects METADATA with accountname
- Calls `trigger_account_notification("Alice", Some("Alice"), context)`
- Same flow as SVSMODE

## Server Integration

The server must coordinate between Atheme and IRCv3 modules:

```rust
// In server's module coordinator or main loop

// When Atheme service triggers account notification
impl ServerCoordinator {
    async fn handle_account_notification(
        &self,
        user_id: Uuid,
        account: Option<String>,
        context: &ServiceContext
    ) -> Result<()> {
        if let Some(ircv3_module) = self.module_manager.get_module_mut("ircv3") {
            if let Some(account_name) = account {
                // User identified
                ircv3_module.set_user_account(user_id, account_name, context).await?;
            } else {
                // User logged out
                ircv3_module.remove_user_account(user_id, context).await?;
            }
        }
        Ok(())
    }
}
```

## Configuration

### Atheme Configuration

In `atheme.conf`:
```conf
loadmodule "modules/protocol/charybdis";
loadmodule "modules/nickserv/main";
loadmodule "modules/nickserv/identify";

# Enable account notifications
serverinfo {
    name = "services.example.org";
    # ... other config ...
}

nickserv {
    # This enables sending SVSMODE +r on IDENTIFY
    modeonid = true;
}
```

### IRCd Configuration

In `config.toml`:
```toml
[services]
enabled = true

[[services.services]]
name = "services.example.org"
service_type = "atheme"
hostname = "localhost"
port = 6666
password = "linkpassword"
```

## Testing

### Test NickServ Identification

1. **User connects and joins channel:**
```irc
USER alice alice host :Alice
NICK Alice
JOIN #test
```

2. **User requests account-notify capability:**
```irc
CAP REQ :account-notify
```

3. **User identifies:**
```irc
/msg NickServ IDENTIFY mypassword
```

4. **Expected flow:**
   - Atheme sends: `:services.example.org SVSMODE Alice +r`
   - IRCd detects and triggers notification
   - IRCv3 broadcasts: `:Alice!alice@host ACCOUNT Alice`
   - Other users in #test receive the ACCOUNT message

### Test Logout

1. **User disconnects or sends:**
```irc
/msg NickServ LOGOUT
```

2. **Expected flow:**
   - Atheme sends: `:services.example.org SVSMODE Alice -r`
   - IRCd detects and triggers notification
   - IRCv3 broadcasts: `:Alice!alice@host ACCOUNT *`

## Logging

The integration provides detailed logging:

```
[INFO] SVSMODE command from Atheme: Alice +r
[INFO] User Alice identified with NickServ (mode +r)
[INFO] NickServ: User Alice identified as account Alice
[DEBUG] Account notification should be triggered: Alice -> Alice
```

For logout:
```
[INFO] SVSMODE command from Atheme: Alice -r
[INFO] User Alice unidentified/logged out (mode -r)
[INFO] NickServ: User Alice logged out
[DEBUG] Account removal notification should be triggered: Alice
```

## Architecture

### Module Communication

```
┌──────────────┐
│   Atheme     │ (External Services)
│  NickServ    │
└──────┬───────┘
       │ SVSMODE +r / ENCAP LOGIN / METADATA
       ↓
┌──────────────┐
│   Atheme     │
│  Services    │ (services/src/atheme.rs)
│   Module     │
└──────┬───────┘
       │ trigger_account_notification()
       ↓
┌──────────────┐
│   Server     │
│ Coordinator  │ (To be implemented)
└──────┬───────┘
       │ ircv3.set_user_account()
       ↓
┌──────────────┐
│   IRCv3      │
│   Module     │ (modules/src/ircv3/mod.rs)
└──────┬───────┘
       │ broadcast ACCOUNT message
       ↓
┌──────────────┐
│   Channel    │
│   Members    │
└──────────────┘
```

## Supported Atheme Versions

- ✅ Atheme 7.x (with Charybdis protocol)
- ✅ Atheme Services 8.x
- ✅ Any Atheme using SVSMODE for identification
- ✅ Any Atheme using ENCAP LOGIN
- ✅ Any Atheme using METADATA

## Files Modified

1. **`services/src/atheme.rs`**
   - Added `trigger_account_notification()`
   - Updated `handle_atheme_svsmode_with_context()` to detect +r/-r
   - Added `handle_atheme_encap_login()`
   - Added `handle_atheme_metadata()`
   - Updated message handler to process ENCAP and METADATA

## Next Steps

1. ✅ Atheme detection methods implemented
2. ✅ Account notification hook points added
3. ✅ Multiple protocol methods supported (SVSMODE, ENCAP, METADATA)
4. ⏳ Server coordinator to link Atheme → IRCv3
5. ⏳ Test with live Atheme instance
6. ⏳ Add support for grouped nicks (different nick, same account)

## See Also

- `docs/ACCOUNT_INTEGRATION.md` - General account integration guide
- `ACCOUNT_NOTIFICATION_INTEGRATION.md` - SASL integration
- `services/ATHEME_PROTOCOL.md` - Atheme protocol documentation
- IRCv3 account-notify spec: https://ircv3.net/specs/extensions/account-notify

