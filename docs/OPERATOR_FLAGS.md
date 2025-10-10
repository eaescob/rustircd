# RustIRCd Operator Flags Reference

## Overview

RustIRCd implements a **granular operator permission system** based on flags. Instead of a single "operator" privilege level, administrators can combine specific flags to create operators with precisely the permissions they need. This follows the principle of **least privilege** and allows for fine-grained access control.

## Architecture

The operator system follows a **modular architecture**:

- **Operator Flags** are defined in `core` (`core/src/config.rs`) - used by all commands
- **Flag Definitions** are in `core/src/user.rs` - part of the User struct  
- **OPER Authentication** is handled by the **oper module** (`modules/src/oper.rs`)
- **Flag Checking** is done in core commands (SQUIT, KILL, CONNECT, etc.)

This means:
- ✅ Operator flags are **always available** (core functionality)
- ✅ OPER command requires the **oper module** to be loaded
- ✅ Commands like SQUIT, KILL, CONNECT check flags directly (no module needed)
- ✅ Configuration is in `config.toml` (operators section)

**To enable operator authentication:**
```toml
# config.toml
[modules]
enabled_modules = ["oper", "channel", "ircv3"]  # Include "oper"
```

Without the oper module, users cannot authenticate as operators (OPER command unavailable), but operator flag checking still works for any users who have flags set through other means.

## Available Operator Flags

### 🌐 GlobalOper
**Full operator privileges network-wide**

- **Can KILL any user** on any server in the network
- Can perform operator commands globally
- Highest privilege level for user management
- Required for network-wide moderation

**Use cases:**
- Head administrators
- Network moderators
- Global abuse handlers

**Security note:** Grant sparingly - allows killing any user anywhere.

---

### 🏠 LocalOper
**Operator privileges limited to local server only**

- **Can KILL users** connected to the local server only
- Cannot affect remote servers or users
- Good for server-specific moderation
- Safer than GlobalOper for most staff

**Use cases:**
- Server administrators
- Local moderators
- Server-specific support staff

**Security note:** Safer than GlobalOper - limited blast radius.

---

### 🔗 RemoteConnect
**Can use /CONNECT to link remote servers**

- Can establish **server-to-server connections** from anywhere
- Can connect servers that are not directly linked
- Required for network topology management
- Allows connecting third-party servers to the network

**Use cases:**
- Hub administrators
- Network topology managers
- Senior staff managing network structure

**Security note:** Powerful - can reshape network topology.

---

### 🏠 LocalConnect
**Can use /CONNECT to link servers from local server**

- Limited to connecting **from this server only**
- Cannot manage remote server connections
- Cannot force remote servers to connect to others
- Useful for leaf server administrators

**Use cases:**
- Local server administrators
- Operators who need to reconnect their own servers
- Limited server management roles

**Security note:** Safer than RemoteConnect - limited scope.

---

### 👑 Administrator
**Enhanced administrative privileges**

- **Can see secret channels** in /WHOIS (+s channels)
- Access to additional server information
- Enhanced visibility for troubleshooting
- Does NOT grant KILL or SQUIT privileges

**Use cases:**
- Senior staff members
- Support staff needing enhanced visibility
- Services administrators

**Security note:** Information only - no destructive powers.

---

### 🕵️ Spy
**Notified when users /WHOIS them**

- Receives alerts when someone queries their information
- Privacy/security feature for high-profile operators
- Helps detect potential targeting or surveillance
- Does NOT grant any additional privileges

**Use cases:**
- High-profile operators
- Security-conscious administrators
- Network owners

**Security note:** Information only - no additional privileges.

---

### ⚠️ Squit
**Can use /SQUIT to disconnect servers**

- Can **forcibly remove servers** from the network
- Causes network splits and disconnections
- Powerful and potentially destructive
- Required for emergency network management

**Use cases:**
- Senior network administrators
- Hub administrators
- Emergency response operators

**Security note:** DANGEROUS - can split the entire network. Grant with extreme caution.

---

## Common Flag Combinations

### Full Network Administrator
```toml
flags = ["GlobalOper", "RemoteConnect", "Administrator", "Squit", "Spy"]
```
**Permissions:**
- ✓ Kill any user network-wide
- ✓ Connect/disconnect servers anywhere
- ✓ See all channels in WHOIS
- ✓ SQUIT servers
- ✓ Receive WHOIS notifications

**Use for:** Head network administrators with complete control.

---

### Local Server Administrator
```toml
flags = ["LocalOper", "LocalConnect", "Administrator"]
```
**Permissions:**
- ✓ Kill users on local server only
- ✓ Connect servers from this server
- ✓ See all channels in WHOIS
- ✗ Cannot kill remote users
- ✗ Cannot SQUIT servers
- ✗ Cannot manage remote server connections

**Use for:** Server administrators managing a single server.

---

### Hub Administrator (Server Topology)
```toml
flags = ["LocalOper", "RemoteConnect", "Squit"]
```
**Permissions:**
- ✓ Connect/disconnect servers anywhere
- ✓ SQUIT servers (emergency splits)
- ✓ Kill local users
- ✗ Cannot kill remote users
- ✗ No enhanced administrative info

**Use for:** Operators who manage server topology and network structure.

---

### Global Moderator (User Management)
```toml
flags = ["GlobalOper"]
```
**Permissions:**
- ✓ Kill any user network-wide
- ✗ Cannot manage servers
- ✗ No enhanced administrative info
- ✗ Cannot SQUIT

**Use for:** Network moderators who enforce rules network-wide.

---

### Local Moderator (Server-Specific)
```toml
flags = ["LocalOper"]
```
**Permissions:**
- ✓ Kill users on local server only
- ✗ Cannot kill remote users
- ✗ Cannot manage servers
- ✗ No enhanced privileges

**Use for:** Moderators limited to one server.

---

### Services Administrator (Information Access)
```toml
flags = ["Administrator", "Spy"]
```
**Permissions:**
- ✓ See all channels in WHOIS
- ✓ Receive WHOIS notifications
- ✗ Cannot kill users
- ✗ Cannot manage servers

**Use for:** Operators who manage services integration and need visibility but no destructive powers.

---

## Configuration Examples

### Example 1: Complete Network Administrator
```toml
[[network.operators]]
nickname = "netadmin"
password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
hostmask = "*@*.admin.example.com"
flags = ["GlobalOper", "RemoteConnect", "Administrator", "Squit", "Spy"]
enabled = true
```

### Example 2: Local Server Admin
```toml
[[network.operators]]
nickname = "serveradmin"
password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
hostmask = "*@localhost"
flags = ["LocalOper", "LocalConnect", "Administrator"]
enabled = true
```

### Example 3: Hub Administrator
```toml
[[network.operators]]
nickname = "hubadmin"
password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
hostmask = "*@*.hubs.example.com"
flags = ["LocalOper", "RemoteConnect", "Squit"]
enabled = true
```

### Example 4: Global Moderator
```toml
[[network.operators]]
nickname = "globalmod"
password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
hostmask = "*@*.staff.example.com"
flags = ["GlobalOper"]
enabled = true
```

### Example 5: Local Moderator
```toml
[[network.operators]]
nickname = "localmod"
password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
hostmask = "*@*.example.com"
flags = ["LocalOper"]
enabled = true
```

---

## Security Best Practices

### 1. Use Restrictive Hostmasks
Always use the most restrictive hostmask possible:

**Good:**
```toml
hostmask = "admin@192.168.1.100"           # Specific IP
hostmask = "admin@*.admin.example.com"     # Specific subdomain
hostmask = "*@staff.example.com"           # Specific host
```

**Bad:**
```toml
hostmask = "*@*"                           # Anyone from anywhere!
```

### 2. Follow Least Privilege Principle
Only grant the flags actually needed:

**Good:**
- Local moderator → `["LocalOper"]`
- Server admin → `["LocalOper", "LocalConnect", "Administrator"]`
- Network admin → All flags as needed

**Bad:**
- Giving `["GlobalOper", "Squit"]` to everyone
- Granting `RemoteConnect` and `Squit` without `Oper` flags

### 3. Dangerous Flag Combinations
Be careful with these:

```toml
flags = ["Squit"]                          # Can split network but cannot reconnect!
flags = ["RemoteConnect", "Squit"]         # Can manage topology but not users
flags = ["GlobalOper", "Squit"]            # Very powerful - use with caution
```

### 4. Generate Secure Password Hashes
Never use plain text passwords:

```bash
# SHA256 hash (supported)
echo -n "your_secure_password" | sha256sum

# Use long, random passwords
openssl rand -base64 32 | sha256sum
```

### 5. Audit Operator Actions
Enable comprehensive logging for:
- `/OPER` - Who authenticates
- `/KILL` - Who kills whom
- `/SQUIT` - Who disconnects servers
- `/CONNECT` - Who establishes server links

### 6. Regular Audits
- Review operator list quarterly
- Remove unused operator accounts
- Update hostmasks when staff IP ranges change
- Rotate passwords periodically

---

## Command Permission Matrix

| Command | GlobalOper | LocalOper | RemoteConnect | LocalConnect | Admin | Spy | Squit |
|---------|-----------|-----------|---------------|--------------|-------|-----|-------|
| `/KILL` (local) | ✓ | ✓ | - | - | - | - | - |
| `/KILL` (remote) | ✓ | ✗ | - | - | - | - | - |
| `/CONNECT` (remote) | - | - | ✓ | ✗ | - | - | - |
| `/CONNECT` (local) | - | - | ✓ | ✓ | - | - | - |
| `/SQUIT` | - | - | - | - | - | - | ✓ |
| `/WHOIS` (see secret) | - | - | - | - | ✓ | - | - |
| WHOIS notification | - | - | - | - | - | ✓ | - |

**Legend:**
- ✓ = Permission granted
- ✗ = Permission denied (attempted action fails)
- \- = Not applicable

---

## Testing Operator Permissions

To test different permission levels:

```bash
# 1. Connect to server
telnet localhost 6667

# 2. Register
NICK testuser
USER testuser 0 * :Test User

# 3. Authenticate as operator
OPER nickname password

# 4. Test various commands
KILL someuser :Abuse                    # Test kill permission
SQUIT remote.server.com :Maintenance    # Test squit permission
CONNECT hub.example.com 6668            # Test connect permission
WHOIS someuser                          # Test info visibility
```

Expected results based on flags:
- **GlobalOper**: All KILL commands succeed
- **LocalOper**: Only local KILL succeeds, remote fails
- **Squit**: SQUIT succeeds
- **No Squit**: SQUIT fails with ERR_NOPRIVILEGES
- **Administrator**: See secret channels in WHOIS
- **Spy**: Receive notification when someone does /WHOIS on you

---

## Troubleshooting

### "ERR_NOPRIVILEGES" when trying to SQUIT
**Cause:** Operator doesn't have `Squit` flag.

**Solution:** Add `"Squit"` to the operator's flags in config.toml:
```toml
flags = ["GlobalOper", "Squit"]
```

### Operator can KILL locally but not remotely
**Cause:** Operator has `LocalOper` instead of `GlobalOper`.

**Solution:** Change `"LocalOper"` to `"GlobalOper"` if global kill is needed.

### Cannot see secret channels in WHOIS
**Cause:** Operator doesn't have `Administrator` flag.

**Solution:** Add `"Administrator"` to the operator's flags.

### CONNECT command fails
**Cause:** Operator doesn't have `RemoteConnect` or `LocalConnect` flag.

**Solution:** Add appropriate connect flag to operator's configuration.

---

## Migration from Traditional IRC Operators

Traditional IRC servers typically have a single "operator" privilege level. Here's how to migrate:

### Traditional Single-Level Operators
```toml
# Old style: Everyone is the same
flags = ["GlobalOper"]
```

### Modern Granular Permissions
```toml
# Network admin
flags = ["GlobalOper", "RemoteConnect", "Administrator", "Squit"]

# Server admin
flags = ["LocalOper", "LocalConnect", "Administrator"]

# Moderator
flags = ["LocalOper"]
```

---

## Conclusion

RustIRCd's granular operator flag system provides fine-grained control over operator permissions. By combining different flags, you can create operators with precisely the privileges they need, following security best practices and the principle of least privilege.

For more examples, see:
- `config.toml` - Main configuration with operator examples
- `examples/configs/config.example.toml` - Comprehensive templates
- `examples/configs/squit_operator.toml` - SQUIT permission examples
- `examples/operator_security_test.rs` - Operator testing examples

