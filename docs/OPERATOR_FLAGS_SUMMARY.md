# Operator Flags Enhancement Summary

## What Was Improved

The RustIRCd operator system already had **granular permissions** implemented in the code, but the configuration files lacked comprehensive documentation explaining how to use them. This update provides complete documentation and examples.

## Changes Made

### 1. Enhanced Configuration Files

#### `config.toml` (Main Configuration)
- ✅ Added inline documentation for all 7 operator flags
- ✅ Added quick reference with flag descriptions
- ✅ Provided 4 ready-to-use operator templates
- ✅ Added examples for different operator types

#### `examples/configs/config.example.toml` (Comprehensive Template)
- ✅ Complete operator flags reference section
- ✅ 6 detailed operator templates with use cases:
  1. Network Administrator (Full Privileges)
  2. Local Server Administrator
  3. Hub Administrator (Server Linking)
  4. Global Moderator (User Management)
  5. Local Moderator (Server-Specific)
  6. Services Administrator (Information Access)
- ✅ Security recommendations section
- ✅ Permission matrix for each template
- ✅ Hostmask security guidelines
- ✅ Password hash generation instructions

#### `examples/configs/squit_operator.toml` (SQUIT Testing)
- ✅ 5 operator examples demonstrating SQUIT permissions
- ✅ Detailed flag explanations
- ✅ Testing instructions for each operator type
- ✅ Expected behavior documentation

### 2. New Documentation

#### `docs/OPERATOR_FLAGS.md` (Complete Reference)
A comprehensive 400+ line reference guide including:
- ✅ Detailed description of all 7 operator flags
- ✅ Use cases for each flag
- ✅ Security notes and warnings
- ✅ 6 common flag combination templates
- ✅ Command permission matrix
- ✅ Security best practices
- ✅ Testing procedures
- ✅ Troubleshooting guide
- ✅ Migration guide from traditional IRC

## Available Operator Flags

### 1. **GlobalOper** - Network-Wide User Management
- Kill any user on any server
- Full operator privileges globally
- For: Network administrators, global moderators

### 2. **LocalOper** - Local Server User Management  
- Kill users on local server only
- Limited to one server
- For: Server administrators, local moderators

### 3. **RemoteConnect** - Remote Server Linking
- Connect servers from anywhere
- Manage network topology remotely
- For: Hub administrators, senior staff

### 4. **LocalConnect** - Local Server Linking
- Connect servers from this server only
- Limited topology management
- For: Server administrators

### 5. **Administrator** - Enhanced Information Access
- See secret channels in WHOIS
- Enhanced server information
- No destructive powers
- For: Senior staff, support personnel

### 6. **Spy** - WHOIS Notifications
- Notified when users WHOIS them
- Privacy/security feature
- No additional privileges
- For: High-profile operators

### 7. **Squit** - Server Disconnection
- Disconnect servers from network
- Powerful and potentially destructive
- For: Senior administrators only

## Operator Templates Available

### Template 1: Full Network Administrator
```toml
flags = ["GlobalOper", "RemoteConnect", "Administrator", "Squit", "Spy"]
```
**Everything:** Kill globally, manage servers, see all, disconnect servers

### Template 2: Local Server Administrator
```toml
flags = ["LocalOper", "LocalConnect", "Administrator"]
```
**Server Management:** Local kills, local connects, enhanced info

### Template 3: Hub Administrator
```toml
flags = ["LocalOper", "RemoteConnect", "Squit"]
```
**Topology Management:** Server linking and emergency splits

### Template 4: Global Moderator
```toml
flags = ["GlobalOper"]
```
**User Management Only:** Kill globally, no server management

### Template 5: Local Moderator
```toml
flags = ["LocalOper"]
```
**Local Only:** Kill on local server only

### Template 6: Services Administrator
```toml
flags = ["Administrator", "Spy"]
```
**Information Only:** See all, no destructive powers

## Security Features

### 1. Least Privilege by Default
- Each flag grants specific permissions only
- Combine only flags actually needed
- No "super user" with unlimited power

### 2. Hostmask Restrictions
- Operators can be restricted by host/IP
- Patterns: `*@*.admin.example.com`
- CIDR notation: `admin@192.168.1.0/24`

### 3. Granular Access Control
- Separate user management from server management
- Separate local from global operations
- Separate information access from destructive powers

### 4. Audit Trail
- All operator actions logged
- Flag combinations visible in config
- Easy to track who has what permissions

## Example Usage

### Creating a Global Moderator
```toml
[[network.operators]]
nickname = "moderator"
password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
hostmask = "*@*.staff.example.com"
flags = ["GlobalOper"]  # Can only kill users
enabled = true
```

**Can do:** `/KILL username :Reason` (globally)
**Cannot do:** `/SQUIT`, `/CONNECT` (no server management)

### Creating a Hub Administrator
```toml
[[network.operators]]
nickname = "hubadmin"
password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
hostmask = "*@*.hubs.example.com"
flags = ["LocalOper", "RemoteConnect", "Squit"]  # Server topology only
enabled = true
```

**Can do:** `/CONNECT`, `/SQUIT`, `/KILL` (local only)
**Cannot do:** `/KILL` remote users (no GlobalOper)

### Creating a Local Server Admin
```toml
[[network.operators]]
nickname = "serveradmin"
password_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
hostmask = "*@localhost"
flags = ["LocalOper", "LocalConnect", "Administrator"]
enabled = true
```

**Can do:** `/KILL` local, `/CONNECT` local, see secret channels
**Cannot do:** `/KILL` remote, `/SQUIT`, `/CONNECT` remote

## Testing Your Configuration

1. **Start the server:**
   ```bash
   cargo run --release
   ```

2. **Connect and authenticate:**
   ```
   NICK testuser
   USER testuser 0 * :Test
   OPER nickname password
   ```

3. **Test permissions:**
   ```
   KILL username :Test              (Tests GlobalOper/LocalOper)
   SQUIT server.com :Test          (Tests Squit)
   CONNECT hub.example.com 6668    (Tests RemoteConnect/LocalConnect)
   WHOIS someuser                  (Tests Administrator)
   ```

4. **Expected results:**
   - With correct flags: Command succeeds
   - Without correct flags: `ERR_NOPRIVILEGES (481)`

## Files to Reference

1. **`config.toml`** - Quick reference and examples
2. **`examples/configs/config.example.toml`** - Comprehensive templates
3. **`examples/configs/squit_operator.toml`** - SQUIT testing examples
4. **`docs/OPERATOR_FLAGS.md`** - Complete reference guide
5. **`docs/OPERATOR_FLAGS_SUMMARY.md`** - This file

## Best Practices

### ✅ DO:
- Use restrictive hostmasks (`*@*.admin.example.com`)
- Grant only flags actually needed
- Use LocalOper instead of GlobalOper when possible
- Audit operator list regularly
- Use strong passwords with SHA256 hashing

### ❌ DON'T:
- Use `*@*` hostmasks in production
- Grant GlobalOper to everyone
- Give Squit flag without careful consideration
- Store plain text passwords
- Grant RemoteConnect and Squit without Oper flags

## Quick Reference

| Flag | Grants Permission To |
|------|---------------------|
| GlobalOper | KILL users globally |
| LocalOper | KILL users locally |
| RemoteConnect | CONNECT servers remotely |
| LocalConnect | CONNECT servers locally |
| Administrator | See secret channels |
| Spy | Receive WHOIS notifications |
| Squit | Disconnect servers (SQUIT) |

## Conclusion

RustIRCd's operator system is now fully documented with:
- 7 granular permission flags
- 6 ready-to-use templates
- Comprehensive security guidelines
- Complete testing procedures
- Migration guidance

This allows IRC network administrators to implement **least-privilege security** while maintaining operational flexibility.

