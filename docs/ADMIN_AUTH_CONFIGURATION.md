# RustIRCd Authentication Configuration Guide for Administrators

This guide provides step-by-step instructions for server administrators to configure and enable authentication providers in RustIRCd.

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Configuration File Structure](#configuration-file-structure)
4. [Available Authentication Providers](#available-authentication-providers)
5. [Provider Configuration Examples](#provider-configuration-examples)
6. [SASL Configuration](#sasl-configuration)
7. [Testing Authentication](#testing-authentication)
8. [Troubleshooting](#troubleshooting)
9. [Security Considerations](#security-considerations)
10. [Performance Tuning](#performance-tuning)

## Overview

RustIRCd supports multiple authentication providers that allow users to authenticate using various external services. The authentication system integrates with IRC SASL (Simple Authentication and Security Layer) to provide secure user authentication.

### Key Benefits

- **Multiple Providers**: Support for LDAP, databases, HTTP APIs, file-based auth, and IRC services
- **Fallback Support**: Automatic failover between authentication providers
- **Caching**: Built-in authentication caching for improved performance
- **Security**: Secure credential handling and TLS support
- **Flexibility**: Easy to add new authentication providers

## Quick Start

### 1. Enable SASL Module

First, ensure the SASL module is enabled in your configuration:

```toml
[modules]
sasl = { enabled = true }

[modules.sasl]
mechanisms = ["PLAIN"]
allow_insecure_mechanisms = false
max_failed_attempts = 3
session_timeout_seconds = 300
```

### 2. Configure Authentication Manager

Add authentication configuration to your main config file:

```toml
[auth]
cache_ttl_seconds = 3600
primary_provider = "ldap"
fallback_providers = ["database", "file"]
```

### 3. Configure Your Authentication Provider

Choose and configure one of the available providers (see examples below).

### 4. Restart RustIRCd

Restart your RustIRCd server to apply the new authentication configuration.

## Configuration File Structure

The authentication system uses a hierarchical configuration structure:

```toml
# Main authentication settings
[auth]
cache_ttl_seconds = 3600
primary_provider = "provider_name"
fallback_providers = ["fallback1", "fallback2"]

# SASL module configuration
[modules.sasl]
mechanisms = ["PLAIN"]
allow_insecure_mechanisms = false
max_failed_attempts = 3
session_timeout_seconds = 300

# Individual provider configurations
[auth.ldap]
# LDAP provider settings

[auth.database]
# Database provider settings

[auth.http]
# HTTP API provider settings

[auth.file]
# File-based provider settings

[auth.supabase]
# Supabase provider settings
```

## Available Authentication Providers

### 1. LDAP/Active Directory Provider

Authenticate users against LDAP servers or Active Directory.

**Configuration:**
```toml
[auth.ldap]
enabled = true
server_url = "ldap://ldap.company.com:389"
bind_dn = "cn=admin,dc=company,dc=com"
bind_password = "admin_password"
base_dn = "ou=users,dc=company,dc=com"
username_attribute = "uid"
search_filter = "(objectClass=person)"
use_tls = false
timeout_seconds = 30
max_connections = 10
```

**Example for Active Directory:**
```toml
[auth.ldap]
enabled = true
server_url = "ldaps://dc.company.com:636"
bind_dn = "CN=Service Account,OU=Service Accounts,DC=company,DC=com"
bind_password = "service_account_password"
base_dn = "OU=Users,DC=company,DC=com"
username_attribute = "sAMAccountName"
search_filter = "(&(objectClass=user)(objectCategory=person))"
use_tls = true
timeout_seconds = 30
max_connections = 10
```

### 2. Database Provider

Authenticate users against a database (PostgreSQL, MySQL, SQLite).

**Configuration:**
```toml
[auth.database]
enabled = true
connection_string = "postgresql://username:password@localhost/ircdb"
user_table = "users"
username_column = "username"
password_column = "password_hash"
email_column = "email"
active_column = "is_active"
password_hash_algorithm = "bcrypt"  # bcrypt, scrypt, argon2, plain
connection_pool_size = 10
timeout_seconds = 30
```

**MySQL Example:**
```toml
[auth.database]
enabled = true
connection_string = "mysql://username:password@localhost/ircdb"
user_table = "irc_users"
username_column = "login"
password_column = "password_hash"
email_column = "email_address"
active_column = "active"
password_hash_algorithm = "bcrypt"
connection_pool_size = 10
timeout_seconds = 30
```

**SQLite Example:**
```toml
[auth.database]
enabled = true
connection_string = "sqlite:///var/lib/rustircd/users.db"
user_table = "users"
username_column = "username"
password_column = "password_hash"
email_column = "email"
active_column = "is_active"
password_hash_algorithm = "bcrypt"
connection_pool_size = 5
timeout_seconds = 30
```

### 3. HTTP API Provider

Authenticate users against a REST API.

**Configuration:**
```toml
[auth.http]
enabled = true
base_url = "https://api.yourservice.com"
auth_endpoint = "/authenticate"
method = "POST"
username_field = "username"
password_field = "password"
response_format = "json"
success_field = "success"
user_field = "user"
api_key = "your-api-key"
timeout_seconds = 30
max_connections = 10
```

**Example for Custom API:**
```toml
[auth.http]
enabled = true
base_url = "https://auth.company.com"
auth_endpoint = "/api/v1/auth"
method = "POST"
username_field = "login"
password_field = "passwd"
response_format = "json"
success_field = "authenticated"
user_field = "user_info"
api_key = "Bearer your-jwt-token"
timeout_seconds = 30
max_connections = 10
```

### 4. File-based Provider

Authenticate users against a local file.

**Configuration:**
```toml
[auth.file]
enabled = true
file_path = "/etc/rustircd/users.conf"
format = "ini"  # ini, json, csv
username_field = "username"
password_field = "password_hash"
case_sensitive = true
reload_on_change = true
check_interval_seconds = 60
```

**Example file format (`/etc/rustircd/users.conf`):**
```ini
[user1]
username = alice
password_hash = $2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj4VbJJyL5Kq
email = alice@example.com

[user2]
username = bob
password_hash = $2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj4VbJJyL5Kq
email = bob@example.com
```

### 5. Supabase Provider

Authenticate users against a Supabase project.

**Configuration:**
```toml
[auth.supabase]
enabled = true
project_url = "https://your-project-id.supabase.co"
api_key = "your-supabase-anon-key"
user_table = "irc_users"
username_column = "username"
password_column = "password_hash"
email_column = "email"
use_email_auth = false
timeout_seconds = 30
max_connections = 10
```

**Supabase Table Schema:**
```sql
-- Create table for IRC users
CREATE TABLE irc_users (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  username TEXT UNIQUE NOT NULL,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  is_active BOOLEAN DEFAULT true
);

-- Enable Row Level Security
ALTER TABLE irc_users ENABLE ROW LEVEL SECURITY;

-- Create policy for reading user data
CREATE POLICY "Allow read access for auth" ON irc_users
  FOR SELECT USING (true);
```

### 6. Atheme Services Provider

Authenticate users against Atheme IRC services.

**Configuration:**
```toml
[services.atheme]
enabled = true
hostname = "services.example.com"
port = 7000
service_name = "AuthServ"
password = "services_password"
use_ssl = true
timeout_seconds = 30

[auth.atheme_sasl]
enabled = true
service_provider = "atheme"
mechanisms = ["PLAIN"]
timeout_seconds = 30
```

## SASL Configuration

Configure SASL (Simple Authentication and Security Layer) for IRC client authentication:

```toml
[modules.sasl]
# Enable SASL module
enabled = true

# Supported SASL mechanisms
mechanisms = ["PLAIN"]  # Can include: PLAIN, EXTERNAL, SCRAM-SHA-256

# Security settings
allow_insecure_mechanisms = false  # Require TLS for PLAIN
max_failed_attempts = 3           # Max failed auth attempts per client
session_timeout_seconds = 300     # SASL session timeout

# Advanced settings
require_ssl = true                # Require SSL/TLS for SASL
max_sessions_per_ip = 5          # Limit concurrent SASL sessions
```

### SASL Mechanism Options

- **PLAIN**: Username/password authentication (requires TLS)
- **EXTERNAL**: Certificate-based authentication
- **SCRAM-SHA-256**: Challenge-response authentication (more secure)

## Testing Authentication

### 1. Test Configuration

Use the built-in configuration validator:

```bash
rustircd --validate-config /path/to/config.toml
```

### 2. Test Authentication Providers

Test individual providers:

```bash
# Test LDAP connection
rustircd --test-auth ldap --config /path/to/config.toml

# Test database connection
rustircd --test-auth database --config /path/to/config.toml

# Test HTTP API
rustircd --test-auth http --config /path/to/config.toml
```

### 3. Test with IRC Client

Connect with an IRC client that supports SASL:

```bash
# Using irssi
/network add testnet
/server add testnet irc.example.com 6667
/sasl set testnet PLAIN username password
/connect testnet

# Using weechat
/server add testnet irc.example.com/6667
/set irc.server.testnet.sasl_mechanism plain
/set irc.server.testnet.sasl_username username
/set irc.server.testnet.sasl_password password
/connect testnet
```

## Troubleshooting

### Common Issues

#### 1. Authentication Provider Not Found

**Error:** `Provider 'ldap' not found`

**Solution:** Ensure the provider is properly configured and enabled:

```toml
[auth.ldap]
enabled = true
# ... other settings
```

#### 2. Connection Timeouts

**Error:** `Connection timeout` or `Network error`

**Solutions:**
- Check network connectivity
- Verify server URLs and ports
- Increase timeout settings
- Check firewall rules

#### 3. Invalid Credentials

**Error:** `Authentication failed` or `Invalid password`

**Solutions:**
- Verify username/password combinations
- Check password hash algorithms
- Ensure user accounts are active
- Verify database/API credentials

#### 4. SASL Not Working

**Error:** Client cannot authenticate via SASL

**Solutions:**
- Ensure SASL module is enabled
- Check SASL mechanism support
- Verify TLS/SSL configuration
- Check client SASL configuration

### Debug Logging

Enable debug logging to troubleshoot authentication issues:

```toml
[logging]
level = "debug"
auth_logging = true
sasl_logging = true
```

### Log Files

Check these log locations for authentication issues:

- `/var/log/rustircd/auth.log` - Authentication events
- `/var/log/rustircd/sasl.log` - SASL-specific logs
- `/var/log/rustircd/error.log` - General error logs

## Security Considerations

### 1. Password Security

- Use strong password hashing algorithms (bcrypt, scrypt, argon2)
- Never store plain text passwords
- Implement password complexity requirements
- Consider password expiration policies

### 2. Network Security

- Always use TLS/SSL for authentication
- Restrict network access to authentication servers
- Use VPN or private networks when possible
- Implement rate limiting for authentication attempts

### 3. Configuration Security

- Protect configuration files with proper permissions (600 or 400)
- Use environment variables for sensitive data
- Regularly rotate API keys and passwords
- Monitor authentication logs for suspicious activity

### 4. Access Control

- Implement proper user permissions
- Use principle of least privilege
- Regular audit of user accounts
- Disable unused authentication providers

## Performance Tuning

### 1. Caching Configuration

```toml
[auth]
cache_ttl_seconds = 3600        # Cache authentication results for 1 hour
max_cache_size = 10000          # Maximum number of cached entries
cache_cleanup_interval = 300    # Clean up expired entries every 5 minutes
```

### 2. Connection Pooling

For database and HTTP providers:

```toml
[auth.database]
connection_pool_size = 20       # Increase for high-load environments
connection_timeout = 30         # Connection timeout in seconds
idle_timeout = 300              # Close idle connections after 5 minutes

[auth.http]
max_connections = 50            # Maximum concurrent HTTP connections
connection_timeout = 30         # HTTP request timeout
```

### 3. Load Balancing

For multiple authentication servers:

```toml
[auth.ldap]
servers = [
    "ldap://ldap1.company.com:389",
    "ldap://ldap2.company.com:389",
    "ldap://ldap3.company.com:389"
]
load_balance = "round_robin"    # round_robin, failover, weighted
```

### 4. Monitoring

Enable authentication monitoring:

```toml
[auth.monitoring]
enable_stats = true
stats_interval = 60             # Report stats every minute
alert_on_failures = true        # Alert on authentication failures
failure_threshold = 10          # Alert after 10 failures in interval
```

## Example Complete Configuration

Here's a complete example configuration for a production environment:

```toml
# Main server configuration
[server]
name = "irc.example.com"
description = "Example IRC Server"

# Authentication settings
[auth]
cache_ttl_seconds = 3600
primary_provider = "ldap"
fallback_providers = ["database"]

# SASL configuration
[modules.sasl]
enabled = true
mechanisms = ["PLAIN", "EXTERNAL"]
allow_insecure_mechanisms = false
max_failed_attempts = 3
session_timeout_seconds = 300
require_ssl = true

# LDAP provider (primary)
[auth.ldap]
enabled = true
server_url = "ldaps://ldap.company.com:636"
bind_dn = "CN=IRC Service,OU=Service Accounts,DC=company,DC=com"
bind_password = "${LDAP_BIND_PASSWORD}"
base_dn = "OU=Users,DC=company,DC=com"
username_attribute = "sAMAccountName"
search_filter = "(&(objectClass=user)(objectCategory=person))"
use_tls = true
timeout_seconds = 30
max_connections = 10

# Database provider (fallback)
[auth.database]
enabled = true
connection_string = "${DATABASE_URL}"
user_table = "irc_users"
username_column = "username"
password_column = "password_hash"
email_column = "email"
active_column = "is_active"
password_hash_algorithm = "bcrypt"
connection_pool_size = 10
timeout_seconds = 30

# Logging
[logging]
level = "info"
auth_logging = true
sasl_logging = true
log_file = "/var/log/rustircd/auth.log"

# Monitoring
[auth.monitoring]
enable_stats = true
stats_interval = 60
alert_on_failures = true
failure_threshold = 10
```

This configuration provides a robust, production-ready authentication system with LDAP as the primary provider and database as fallback, complete with monitoring and security features.

## Support

For additional help:

1. Check the logs in `/var/log/rustircd/`
2. Review the developer documentation in `docs/`
3. Test with the provided examples in `examples/`
4. Consult the troubleshooting section above

Remember to always test authentication configuration in a development environment before deploying to production!
