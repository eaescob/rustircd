# Operator Authentication Auditing

This document describes the comprehensive security audit logging implementation for RustIRCd, addressing findings L-004 and M-007 from the security audit report.

## Overview

The audit logging system provides structured, comprehensive logging of security-relevant events including:

- **Authentication Events**: Successful and failed authentication attempts (OPER, SASL, Services)
- **Authorization Events**: Privilege checks and authorization failures
- **Operator Actions**: All actions performed by operators
- **Operator Privilege Changes**: Granting and revoking of operator privileges

## Architecture

### Core Components

#### 1. **AuditEvent** (`core/src/audit.rs`)

A structured event that captures all relevant security information:

```rust
pub struct AuditEvent {
    pub event_type: AuditEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user: Option<String>,
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub hostname: Option<String>,
    pub ip: Option<String>,
    pub method: Option<String>,
    pub command: Option<String>,
    pub target: Option<String>,
    pub required_flag: Option<String>,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub secure: Option<bool>,
    pub metadata: HashMap<String, String>,
}
```

#### 2. **AuditEventType**

Categorizes security events:

- `AuthSuccess` - Successful authentication
- `AuthFailure` - Failed authentication attempt
- `AuthChallenge` - Challenge-response authentication event
- `AuthzSuccess` - Successful authorization check
- `AuthzFailure` - Failed authorization check (security concern)
- `OperAuth` - Successful operator authentication
- `OperAuthFailure` - Failed operator authentication
- `OperAction` - Operator action performed
- `OperPrivilegeGrant` - Operator privileges granted
- `OperPrivilegeRevoke` - Operator privileges revoked

#### 3. **AuditLogger**

Controls audit event logging with configurable levels:

```rust
pub struct AuditLogger {
    enabled: bool,
    min_level: u8,  // 0 = all, 1 = info+, 2 = warn+
}
```

## Event Log Levels

Events are logged at different levels based on security relevance:

### Warning Level (Security Concerns)
- `AuthFailure` - Failed authentication attempts
- `OperAuthFailure` - Failed operator authentication
- `AuthzFailure` - Authorization failures
- `ConnectionThrottle` - Connection throttling triggered
- `ConnectionBan` - Connection banned

### Info Level (Sensitive Operations)
- `OperAuth` - Operator authentication
- `OperAction` - Operator commands executed
- `OperPrivilegeGrant` - Privilege grants
- `OperPrivilegeRevoke` - Privilege revocations
- `ServerSquit` - Server disconnections
- `ConfigChange` - Configuration changes
- `ConfigReload` - Configuration reloads

### Debug Level (All Events)
- `AuthSuccess` - Successful authentications
- `AuthChallenge` - Authentication challenges
- `AuthzSuccess` - Successful authorization checks

## Configuration

### Operator Module Configuration

The operator module includes audit logging configuration in `OperConfig`:

```rust
pub struct OperConfig {
    pub enabled: bool,
    pub require_oper_for_connect: bool,
    pub show_server_details_in_stats: bool,
    pub log_operator_actions: bool,
    pub audit_enabled: bool,           // Enable/disable audit logging
    pub audit_min_level: u8,            // Minimum log level (0-2)
}
```

**Default Configuration:**
```rust
OperConfig {
    enabled: true,
    audit_enabled: true,
    audit_min_level: 0,  // Log all security events
    // ... other fields
}
```

### Configuration Levels

- **Level 0** (All): Logs all security events including successful operations
- **Level 1** (Info+): Logs info and warning level events (recommended for production)
- **Level 2** (Warn+): Logs only warning level events (minimal logging)

## Usage Examples

### Example 1: Operator Authentication Event

**Successful Authentication:**
```
[INFO] Security event
  event=oper_auth
  user="alice"
  user_id="550e8400-e29b-41d4-a716-446655440000"
  username="alice"
  hostname="admin.example.com"
  ip="192.0.2.1"
  method="OPER"
  metadata={"flags": "[GlobalOper, RemoteConnect]", "oper_name": "alice"}
  timestamp="2025-10-27T12:34:56Z"
```

**Failed Authentication:**
```
[WARN] Security event
  event=oper_auth_failure
  user="mallory"
  username="mallory"
  ip="203.0.113.10"
  method="OPER"
  error="Invalid credentials"
  reason="Operator authentication failed - invalid username, password, or hostmask"
  timestamp="2025-10-27T12:35:12Z"
```

### Example 2: Authorization Failure

**Insufficient Privileges:**
```
[WARN] Security event
  event=authz_failure
  user="bob"
  user_id="550e8400-e29b-41d4-a716-446655440001"
  username="bob"
  hostname="user.example.com"
  command="SQUIT"
  required_flag="Squit"
  error="Insufficient privileges"
  reason="User attempted SQUIT without required privileges"
  timestamp="2025-10-27T12:36:30Z"
```

### Example 3: Operator Action

**SQUIT Command:**
```
[INFO] Security event
  event=oper_action
  user="alice"
  user_id="550e8400-e29b-41d4-a716-446655440000"
  username="alice"
  hostname="admin.example.com"
  command="SQUIT"
  target="hub.example.com"
  reason="Network maintenance"
  timestamp="2025-10-27T12:37:00Z"
```

### Example 4: Authentication via Services

**Successful SASL Authentication:**
```
[INFO] Security event
  event=auth_success
  user="charlie"
  user_id="550e8400-e29b-41d4-a716-446655440002"
  ip="192.0.2.50"
  hostname="client.example.com"
  method="Atheme"
  secure=true
  metadata={"provider": "Atheme", "fallback": "false"}
  timestamp="2025-10-27T12:38:15Z"
```

## Implementation Details

### Operator Module Integration

The operator module (`modules/src/oper.rs`) has been enhanced with:

1. **Authentication Logging** in `handle_oper()`:
   - Logs successful operator authentication with flags
   - Logs failed authentication with error details
   - Captures IP, hostname, and username for forensics

2. **Authorization Checking** via `check_operator_action()`:
   - Logs successful authorization checks (debug level)
   - Logs authorization failures with required flag information (warning level)
   - Available for all operator privilege checks

3. **Operator Action Logging** via `log_operator_action()`:
   - Logs all operator actions with structured data
   - Includes command, target, and reason information

4. **Privilege Change Logging**:
   - `log_privilege_grant()` logs when privileges are granted
   - `revoke_operator_privileges()` logs when privileges are revoked

### Authentication Manager Integration

The authentication manager (`core/src/auth.rs`) logs:

1. **Primary Provider Authentication**:
   - Success: Logs with provider name and metadata
   - Failure: Logs with error details
   - Challenge: Logs challenge-response events

2. **Fallback Provider Authentication**:
   - Same as primary with "fallback: true" metadata
   - Helps identify authentication provider chains

3. **Final Failure**:
   - Logs when all providers fail
   - Captures complete authentication attempt context

## Security Benefits

This implementation addresses security audit findings:

### L-004: Insufficient Logging of Security Events
✅ **Resolved**: Comprehensive structured logging of all security-relevant events

### M-007: Operator Flag Privilege Escalation Potential
✅ **Addressed**: Complete audit trail of operator privilege grants, revocations, and usage

### Additional Benefits

1. **Forensic Analysis**: All events include timestamp, user identity, IP, hostname
2. **Intrusion Detection**: Failed authentication patterns can be detected
3. **Compliance**: Structured logs support compliance requirements
4. **Debugging**: Detailed context helps troubleshoot authentication issues
5. **Accountability**: Complete audit trail of operator actions

## Log Analysis

### Detecting Brute Force Attacks

Search for multiple `oper_auth_failure` events from the same IP:

```bash
grep "oper_auth_failure" logs/rustircd.log | grep "ip=\"192.0.2.1\"" | wc -l
```

### Monitoring Operator Actions

Filter for operator actions:

```bash
grep "oper_action" logs/rustircd.log
```

### Finding Authorization Failures

Identify users attempting privileged operations:

```bash
grep "authz_failure" logs/rustircd.log
```

## Best Practices

### Production Deployment

1. **Enable Audit Logging**: Set `audit_enabled: true` (default)
2. **Set Appropriate Level**: Use `audit_min_level: 1` for production (info+)
3. **Monitor Logs**: Set up alerting for `authz_failure` and `oper_auth_failure`
4. **Rotate Logs**: Implement log rotation to manage disk space
5. **Secure Log Storage**: Protect logs from unauthorized access

### Development

1. **Full Logging**: Use `audit_min_level: 0` to see all events
2. **Test Coverage**: Verify audit events in security tests
3. **Review Events**: Ensure sensitive operations generate audit events

## Integration with Log Aggregation

The structured log format is compatible with:

- **Elasticsearch/Logstash**: Parse JSON-formatted structured logs
- **Splunk**: Index structured fields for search and alerting
- **Datadog**: Send logs with structured metadata
- **CloudWatch**: Use structured logging for filtering

Example Logstash configuration:

```ruby
filter {
  if [logger_name] =~ /rustircd/ {
    grok {
      match => { "message" => "event=%{WORD:event_type}" }
    }
    if [event_type] in ["authz_failure", "oper_auth_failure"] {
      mutate {
        add_tag => ["security_alert"]
      }
    }
  }
}
```

## Future Enhancements

Potential improvements:

1. **Log Shipping**: Direct integration with log aggregation services
2. **Alert Rules**: Built-in alerting for suspicious patterns
3. **Rate Limiting**: Track authentication failure rates per IP
4. **Geo-Location**: Add geographic information for IP addresses
5. **Session Tracking**: Correlate events within user sessions
6. **Export Formats**: Support for JSON, CEF, or other standard formats

## Testing

The audit module includes comprehensive tests:

```bash
# Run audit module tests
cargo test --lib -p rustircd-core audit

# Run operator module tests
cargo test --lib -p rustircd-modules oper
```

## API Reference

### Creating Audit Events

```rust
use rustircd_core::audit::{AuditEvent, AuditEventType, AuditLogger};

// Create an audit event
let event = AuditEvent::new(AuditEventType::OperAuth)
    .with_user("alice")
    .with_user_id(user.id)
    .with_ip("192.0.2.1")
    .with_method("OPER")
    .with_metadata("flags", "GlobalOper");

// Log the event
audit_logger.log(&event);
```

### Checking Operator Actions with Auditing

```rust
use rustircd_modules::oper::{OperModule, OperatorAction};

// Check and log operator action
let can_perform = oper_module.check_operator_action(
    &user,
    OperatorAction::Squit,
    "SQUIT"
);

if can_perform {
    // Perform the action
    // Success will be logged automatically
} else {
    // Authorization failure logged automatically
    return Err(Error::NoPrivileges("..."));
}
```

## Compliance Mapping

This implementation supports various compliance requirements:

| Requirement | Coverage |
|------------|----------|
| **PCI DSS 10.2** | User authentication logging ✅ |
| **PCI DSS 10.3** | User identification in logs ✅ |
| **SOC 2 CC6.1** | Logical access logging ✅ |
| **HIPAA § 164.312(b)** | Audit controls ✅ |
| **GDPR Art. 32** | Security monitoring ✅ |
| **ISO 27001 A.12.4.1** | Event logging ✅ |

## Conclusion

The operator authentication auditing implementation provides comprehensive, structured security logging that:

- ✅ Meets security audit recommendations
- ✅ Provides forensic capabilities
- ✅ Supports compliance requirements
- ✅ Enables real-time security monitoring
- ✅ Maintains backward compatibility

For questions or issues, please refer to the main project documentation or open an issue on GitHub.
