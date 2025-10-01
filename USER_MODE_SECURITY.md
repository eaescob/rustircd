# User Mode Security in RustIRCD

This document explains the security model for user mode management in RustIRCD, with particular focus on operator mode restrictions and the proper separation between OPER command authentication and MODE command management.

## Overview

RustIRCD implements a secure user mode management system that prevents unauthorized privilege escalation while allowing users appropriate control over their own modes. The key security principle is that **operator privileges can only be granted through the OPER command authentication process, never through the MODE command**.

## Security Model

### Operator Mode Restrictions

#### +o (Operator Mode) - RESTRICTED
- **Setting**: ❌ **NEVER ALLOWED** via MODE command
- **Removal**: ✅ **ALLOWED** for users to remove their own operator privileges
- **Grant Method**: Only through successful OPER command authentication
- **Security Rationale**: Prevents privilege escalation attacks and ensures proper authentication

#### +O (Local Operator Mode) - RESTRICTED  
- **Setting**: ❌ **NEVER ALLOWED** via MODE command
- **Removal**: ✅ **ALLOWED** for users to remove their own local operator privileges
- **Grant Method**: Only through OPER command with local operator flags
- **Security Rationale**: Same as global operator mode

### Self-Managed Modes

#### +i (Invisible Mode) - SELF-ONLY
- **Setting**: ✅ **ALLOWED** for self only
- **Removal**: ✅ **ALLOWED** for self only
- **Security Rationale**: Privacy control for users

#### +a (Away Mode) - SELF-ONLY
- **Setting**: ✅ **ALLOWED** for self only
- **Removal**: ✅ **ALLOWED** for self only
- **Security Rationale**: Status management for users

#### +w (Wallops Mode) - SELF-ONLY
- **Setting**: ✅ **ALLOWED** for self only
- **Removal**: ✅ **ALLOWED** for self only
- **Security Rationale**: Message preference control

#### +s (Server Notices Mode) - SELF-ONLY
- **Setting**: ✅ **ALLOWED** for self only
- **Removal**: ✅ **ALLOWED** for self only
- **Security Rationale**: Server message preference control

### Operator-Controlled Modes

#### +r (Restricted Mode) - OPERATOR-ONLY
- **Setting**: ✅ **ALLOWED** for operators only
- **Removal**: ✅ **ALLOWED** for operators only
- **Security Rationale**: Administrative control over problematic users

## Implementation Details

### Mode Validation Logic

```rust
// Check if mode can only be set by OPER command (not MODE command)
if adding && mode.oper_only() {
    return Err("Operator mode can only be granted through OPER command".to_string());
}

// Check operator requirements for removal
if !adding && mode.requires_operator() && !requesting_user_is_operator {
    // Exception: Users can always remove their own operator mode
    if !(is_self && (mode == UserMode::Operator || mode == UserMode::LocalOperator)) {
        return Err("Permission denied".to_string());
    }
}
```

### Error Responses

#### ERR_CANTSETOPERATORMODE (503)
```
:server.example.com 503 user1 :Operator mode can only be granted through OPER command
```
- **Triggered**: When user attempts to set +o or +O via MODE command
- **Security Purpose**: Clear indication that operator privileges require proper authentication

#### ERR_USERSDONTMATCH (502)
```
:server.example.com 502 user2 :Cannot change mode for other users
```
- **Triggered**: When user attempts to change self-only modes for others
- **Security Purpose**: Prevents unauthorized mode changes

## Command Usage Examples

### ✅ Allowed Operations

```irc
# View current modes
/MODE user1

# Set invisible mode (self-only)
/MODE user1 +i

# Set away mode (self-only)  
/MODE user1 +a

# Remove operator mode (self-removal allowed)
/MODE user1 -o

# Combined mode changes
/MODE user1 +i-a+w
```

### ❌ Prohibited Operations

```irc
# Try to set operator mode (ALWAYS FAILS)
/MODE user1 +o
# Response: :server.example.com 503 user1 :Operator mode can only be granted through OPER command

# Try to set operator mode for others (ALWAYS FAILS)
/MODE user2 +o
# Response: :server.example.com 503 user2 :Operator mode can only be granted through OPER command

# Try to set self-only modes for others
/MODE user2 +i
# Response: :server.example.com 502 user2 :Cannot change mode for other users
```

## Security Benefits

### 1. **Prevents Privilege Escalation**
- Users cannot gain operator privileges through MODE command
- Operator status requires proper authentication via OPER command
- Eliminates potential security vulnerabilities

### 2. **Maintains Authentication Integrity**
- Operator privileges are tied to proper password authentication
- Hostmask validation ensures legitimate operator access
- No bypass of authentication mechanisms

### 3. **Allows Self-Management**
- Users can remove their own operator privileges if desired
- Users maintain control over privacy and status modes
- Reduces administrative overhead for basic user preferences

### 4. **Clear Error Messages**
- Specific error codes for different violation types
- Clear indication of why operations are denied
- Helps users understand proper command usage

## Integration with OPER Command

The OPER command is the only legitimate way to grant operator privileges:

```irc
# Proper operator authentication
/OPER admin secretpassword
# Response: :server.example.com 381 user1 :You are now an IRC operator
# Result: +o mode is automatically set

# User can later remove operator privileges if desired
/MODE user1 -o
# Response: :user1 MODE user1 :-o
# Result: +o mode is removed
```

## Configuration

### Numeric Reply Configuration

All error messages can be customized in `replies.toml`:

```toml
[replies.503]
code = 503
text = ":Operator mode can only be granted through OPER command"
description = "ERR_CANTSETOPERATORMODE - Can't set operator mode"

[replies.502]
code = 502
text = ":Cannot change mode for other users"
description = "ERR_USERSDONTMATCH - Users don't match"
```

## Testing

### Security Test Cases

1. **Operator Mode Setting Prevention**
   - Test: `/MODE user1 +o`
   - Expected: Error 503
   - Purpose: Verify operator mode cannot be set via MODE

2. **Self-Only Mode Protection**
   - Test: `/MODE user2 +i` (where user2 ≠ requesting user)
   - Expected: Error 502
   - Purpose: Verify self-only modes cannot be set for others

3. **Operator Mode Removal**
   - Test: `/MODE user1 -o` (where user1 has +o and is self)
   - Expected: Success
   - Purpose: Verify users can remove their own operator privileges

4. **Cross-User Operator Mode Removal**
   - Test: `/MODE user2 -o` (where user2 ≠ requesting user)
   - Expected: Error 502
   - Purpose: Verify operator mode removal restricted to self

## Best Practices

### For Server Administrators

1. **Configure Operator Hostmasks Carefully**
   - Use specific hostmasks to prevent unauthorized access
   - Regularly review operator configurations
   - Monitor operator privilege usage

2. **Monitor Mode Changes**
   - Log all mode change attempts
   - Alert on repeated failed operator mode attempts
   - Track operator privilege removals

3. **Educate Users**
   - Explain that operator privileges require OPER command
   - Clarify which modes are self-manageable
   - Provide clear documentation

### For Users

1. **Use Proper Authentication**
   - Always use OPER command to gain operator privileges
   - Never attempt to set +o via MODE command
   - Understand that operator privileges require server configuration

2. **Manage Privacy Modes**
   - Use +i for invisibility when desired
   - Use +a to indicate away status
   - Use +w to receive wallop messages

3. **Self-Management**
   - Remove operator privileges with -o if no longer needed
   - Manage own privacy and status modes appropriately
   - Understand mode restrictions and limitations

## Conclusion

The user mode security model in RustIRCD provides a robust framework for privilege management while maintaining security integrity. By restricting operator mode setting to the OPER command authentication process, the system prevents privilege escalation attacks while still allowing users appropriate control over their own modes and the ability to remove their own operator privileges when desired.

This security model ensures that operator privileges are always tied to proper authentication and server configuration, maintaining the integrity of the IRC server's administrative structure.
