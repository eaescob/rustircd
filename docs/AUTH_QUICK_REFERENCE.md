# RustIRCd Authentication Quick Reference

## Enable Authentication (5 Steps)

### 1. Enable SASL Module
```toml
[modules.sasl]
enabled = true
mechanisms = ["PLAIN"]
```

### 2. Configure Auth Manager
```toml
[auth]
primary_provider = "ldap"  # or database, http, file, supabase
```

### 3. Configure Your Provider
```toml
# Choose one provider below
```

### 4. Restart RustIRCd
```bash
systemctl restart rustircd
```

### 5. Test with IRC Client
```
/sasl set network PLAIN username password
```

---

## Provider Configurations

### LDAP/Active Directory
```toml
[auth.ldap]
enabled = true
server_url = "ldaps://ldap.company.com:636"
bind_dn = "CN=Service,OU=Accounts,DC=company,DC=com"
bind_password = "password"
base_dn = "OU=Users,DC=company,DC=com"
username_attribute = "sAMAccountName"
use_tls = true
```

### Database (PostgreSQL/MySQL/SQLite)
```toml
[auth.database]
enabled = true
connection_string = "postgresql://user:pass@localhost/ircdb"
user_table = "users"
username_column = "username"
password_column = "password_hash"
password_hash_algorithm = "bcrypt"
```

### HTTP API
```toml
[auth.http]
enabled = true
base_url = "https://api.yourservice.com"
auth_endpoint = "/auth"
method = "POST"
username_field = "username"
password_field = "password"
api_key = "your-api-key"
```

### File-based
```toml
[auth.file]
enabled = true
file_path = "/etc/rustircd/users.conf"
format = "ini"
username_field = "username"
password_field = "password_hash"
```

### Supabase
```toml
[auth.supabase]
enabled = true
project_url = "https://project.supabase.co"
api_key = "your-anon-key"
user_table = "irc_users"
username_column = "username"
password_column = "password_hash"
```

---

## Common Commands

### Test Configuration
```bash
rustircd --validate-config /etc/rustircd/config.toml
```

### Test Provider
```bash
rustircd --test-auth ldap --config /etc/rustircd/config.toml
```

### Check Logs
```bash
tail -f /var/log/rustircd/auth.log
tail -f /var/log/rustircd/sasl.log
```

### IRC Client SASL Setup

#### Irssi
```
/network add mynet
/server add mynet irc.example.com 6667
/sasl set mynet PLAIN username password
/connect mynet
```

#### WeeChat
```
/server add mynet irc.example.com/6667
/set irc.server.mynet.sasl_mechanism plain
/set irc.server.mynet.sasl_username username
/set irc.server.mynet.sasl_password password
/connect mynet
```

#### HexChat
1. Network Settings → Edit Networks
2. Add server: `irc.example.com/6667`
3. Enable SASL
4. Set username/password

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Provider not found" | Check `enabled = true` in provider config |
| "Connection timeout" | Check server URL, port, firewall |
| "Authentication failed" | Verify username/password, check logs |
| "SASL not working" | Ensure TLS enabled, check mechanism support |

### Enable Debug Logging
```toml
[logging]
level = "debug"
auth_logging = true
sasl_logging = true
```

### Common Log Locations
- `/var/log/rustircd/auth.log` - Authentication events
- `/var/log/rustircd/sasl.log` - SASL-specific logs  
- `/var/log/rustircd/error.log` - General errors

---

## Security Checklist

- [ ] Use TLS/SSL for all connections
- [ ] Strong password hashing (bcrypt/scrypt/argon2)
- [ ] Secure configuration file permissions (600)
- [ ] Environment variables for passwords
- [ ] Regular password rotation
- [ ] Monitor authentication logs
- [ ] Rate limiting enabled
- [ ] Firewall rules configured

---

## Performance Tuning

### High-Load Configuration
```toml
[auth]
cache_ttl_seconds = 7200        # 2 hour cache
max_cache_size = 50000

[auth.database]
connection_pool_size = 50       # More connections
timeout_seconds = 60            # Longer timeout

[auth.http]
max_connections = 100           # More HTTP connections
```

### Multiple Providers with Fallback
```toml
[auth]
primary_provider = "ldap"
fallback_providers = ["database", "file"]
```

---

## Example Complete Config

```toml
[modules.sasl]
enabled = true
mechanisms = ["PLAIN"]
allow_insecure_mechanisms = false

[auth]
primary_provider = "ldap"
cache_ttl_seconds = 3600

[auth.ldap]
enabled = true
server_url = "ldaps://ldap.company.com:636"
bind_dn = "CN=IRC,OU=Services,DC=company,DC=com"
bind_password = "${LDAP_PASSWORD}"
base_dn = "OU=Users,DC=company,DC=com"
username_attribute = "sAMAccountName"
use_tls = true
timeout_seconds = 30

[logging]
level = "info"
auth_logging = true
```

---

**Need more help?** See the full documentation in `docs/ADMIN_AUTH_CONFIGURATION.md`
