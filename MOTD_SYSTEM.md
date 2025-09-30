# MOTD (Message of the Day) System for RustIRCD

The MOTD system provides configurable welcome messages that are displayed to users when they connect and register with the IRC server.

## Features

- **File-based Configuration**: MOTD content stored in plain text files
- **Automatic Display**: MOTD shown automatically after user registration
- **Manual Command**: Users can request MOTD with `/MOTD` command
- **Graceful Handling**: Proper error messages for missing or empty MOTD files
- **RFC Compliance**: Full RFC 1459 compliance with proper numeric replies
- **Configurable Replies**: All MOTD responses can be customized

## Configuration

### Basic Configuration

Configure the MOTD file path in your server configuration:

```toml
[server]
name = "example.com"
# ... other server settings ...

# MOTD file path (optional) - supports both relative and absolute paths
motd_file = "motd.txt"  # Relative path (resolved from server working directory)
# motd_file = "/etc/rustircd/motd.txt"  # Absolute path (Unix/Linux)
# motd_file = "C:\\Program Files\\RustIRCd\\motd.txt"  # Absolute path (Windows)
```

### MOTD File Format

MOTD files are plain text files with one message per line:

```
Welcome to RustIRCd!
====================

This server features:
• RFC 1459 compliance
• Enhanced security
• Modern features

Have a great time!
```

### Path Handling

The MOTD system supports both relative and absolute file paths:

#### Relative Paths
- Resolved from the server's current working directory
- Examples:
  ```toml
  motd_file = "motd.txt"                    # Same directory as server
  motd_file = "config/messages/motd.txt"    # Subdirectory
  motd_file = "../shared/motd.txt"          # Parent directory
  ```

#### Absolute Paths
- Used as-is without modification
- Platform-specific format:
  ```toml
  # Unix/Linux/macOS
  motd_file = "/etc/rustircd/motd.txt"
  motd_file = "/usr/local/etc/rustircd/motd.txt"
  motd_file = "/opt/rustircd/config/motd.txt"
  
  # Windows
  motd_file = "C:\\Program Files\\RustIRCd\\motd.txt"
  motd_file = "D:\\IRC\\motd.txt"
  ```

#### Path Resolution
- The system automatically detects if a path is absolute or relative
- Absolute paths are identified by platform-specific prefixes:
  - Unix/Linux/macOS: Paths starting with `/`
  - Windows: Paths starting with drive letter (e.g., `C:\`) or UNC paths (`\\server\share`)
- All resolved paths are logged for debugging purposes

### Configuration Options

- **`motd_file`**: Path to MOTD file (optional)
  - Set to `null` or omit to disable MOTD
  - **Supports both relative and absolute paths**
  - Relative paths resolved from server working directory
  - Absolute paths used as-is (platform-specific)
  - File is read once at server startup
  - If file doesn't exist, shows "MOTD file is missing" message

## Usage

### Automatic MOTD Display

MOTD is displayed automatically when a user completes registration:

1. User connects and sends `NICK` and `USER` commands
2. Server sends welcome message (RPL_WELCOME)
3. Server automatically sends MOTD (if configured)
4. User sees welcome message followed by MOTD

### Manual MOTD Command

Users can request MOTD at any time using the `/MOTD` command:

```
/motd
```

This displays the same MOTD content as the automatic display.

### Example IRC Session

```
> /connect localhost 6667
> /nick testuser
> /user testuser 0 * :Test User

< :server 001 testuser :Welcome to the Internet Relay Network testuser!testuser@localhost
< :server 375 * :- server.example.com Message of the Day -
< :server 372 * :- Welcome to RustIRCd!
< :server 372 * :- ====================
< :server 372 * :- 
< :server 372 * :- This server features:
< :server 372 * :- • RFC 1459 compliance
< :server 372 * :- • Enhanced security
< :server 372 * :- • Modern features
< :server 372 * :- 
< :server 372 * :- Have a great time!
< :server 376 * :End of /MOTD command.
```

## RFC 1459 Compliance

The MOTD system implements the following RFC 1459 numeric replies:

### RPL_MOTDSTART (375)
```
:server 375 * :- server.example.com Message of the Day -
```

### RPL_MOTD (372)
```
:server 372 * :- Welcome to RustIRCd!
:server 372 * :- ====================
:server 372 * :- 
:server 372 * :- This server features:
```

### RPL_ENDOFMOTD (376)
```
:server 376 * :End of /MOTD command.
```

### ERR_NOMOTD (422)
```
:server 422 * :MOTD file is missing
```

## Error Handling

### Missing MOTD File
- If `motd_file` is not configured or file doesn't exist
- Shows `ERR_NOMOTD (422)` message
- No server errors or crashes

### Empty MOTD File
- If MOTD file exists but is empty
- Shows `ERR_NOMOTD (422)` message
- Treats empty file as missing

### File Read Errors
- If MOTD file exists but cannot be read (permissions, etc.)
- Logs warning message
- Shows `ERR_NOMOTD (422)` to user
- Server continues normally

## Advanced Features

### Configurable Replies

All MOTD replies can be customized in `replies.toml`:

```toml
[replies.375]
code = 375
text = ":- {server} Message of the Day -"
description = "RPL_MOTDSTART - MOTD start"

[replies.372]
code = 372
text = ":- {line}"
description = "RPL_MOTD - MOTD line"

[replies.376]
code = 376
text = ":End of /MOTD command."
description = "RPL_ENDOFMOTD - MOTD end"

[replies.422]
code = 422
text = ":MOTD file is missing"
description = "ERR_NOMOTD - No MOTD file"
```

### Dynamic MOTD

For dynamic MOTD content, you can:

1. **Generate MOTD at startup**: Use a script to create the MOTD file with current information
2. **Include server statistics**: Add uptime, user count, etc.
3. **Conditional content**: Different MOTD for different times or events

Example dynamic MOTD script:
```bash
#!/bin/bash
cat > motd.txt << EOF
Welcome to RustIRCd!
====================

Server Status:
• Uptime: $(uptime -p)
• Users: $(who | wc -l)
• Date: $(date)

Have a great day!
EOF
```

### Multiple MOTD Files

You can create different MOTD files for different purposes:

- `motd.txt` - Standard MOTD
- `motd_holiday.txt` - Holiday messages
- `motd_maintenance.txt` - Maintenance notices

Switch between them by updating the configuration and restarting the server.

## Best Practices

### MOTD Content Guidelines

1. **Keep it concise**: Users will see this every time they connect
2. **Include important information**: Server rules, contact info, features
3. **Use clear formatting**: Lines, sections, and bullet points
4. **Update regularly**: Keep information current and relevant
5. **Test readability**: Ensure it looks good in IRC clients

### File Management

1. **Use version control**: Track MOTD changes
2. **Backup files**: Keep copies of working MOTD files
3. **Test changes**: Verify MOTD displays correctly before deploying
4. **Monitor logs**: Check for MOTD-related warnings or errors

### Performance Considerations

1. **File size**: Keep MOTD files reasonably small (< 50 lines recommended)
2. **Line length**: Avoid extremely long lines (> 400 characters)
3. **Loading**: MOTD is loaded once at startup, not on each connection
4. **Memory**: MOTD content is stored in memory for fast access

## Examples

### Simple MOTD
```
Welcome to RustIRCd!

Have a great time chatting!
```

### Detailed MOTD
```
Welcome to RustIRCd Network!
============================

Server Information:
• Server: RustIRCd v1.0.0
• Network: RustNet
• Uptime: 24/7

Features:
• RFC 1459 compliant IRC protocol
• Enhanced security with connection throttling
• Configurable replies and MOTD system
• IRCv3 capabilities and extensions

Rules:
• Be respectful to other users
• No spam or flooding
• Follow channel rules

Need help? Try /help or contact operators.

Thank you for choosing RustIRCd!
```

### Maintenance MOTD
```
MAINTENANCE NOTICE
==================

The server will be undergoing maintenance on:
• Date: January 15, 2024
• Time: 02:00 - 04:00 UTC
• Duration: ~2 hours

Services affected:
• User connections may be interrupted
• Channels may be temporarily unavailable

We apologize for any inconvenience.

For updates, visit: https://status.rustircd.com
```

## Testing

### Manual Testing

1. **Connect and register**: Verify MOTD displays after welcome message
2. **Use /MOTD command**: Test manual MOTD display
3. **Missing file test**: Remove MOTD file and verify error message
4. **Empty file test**: Create empty MOTD file and verify error message

### Automated Testing

The MOTD system includes comprehensive tests:

```rust
#[tokio::test]
async fn test_motd_loading() {
    // Test MOTD file loading
}

#[tokio::test]
async fn test_motd_messages() {
    // Test MOTD message generation
}

#[tokio::test]
async fn test_no_motd_file() {
    // Test missing MOTD file handling
}
```

## Troubleshooting

### Common Issues

1. **MOTD not displaying**:
   - Check if `motd_file` is configured
   - Verify file exists and is readable
   - Check server logs for path resolution errors
   - Verify path is correct (relative vs absolute)

2. **Empty MOTD showing**:
   - Verify file has content
   - Check for hidden characters or encoding issues
   - Ensure file is not just whitespace

3. **Permission errors**:
   - Check file permissions
   - Ensure server can read the file
   - Verify file path is correct

4. **Path resolution errors**:
   - Check if absolute path exists
   - Verify relative path is correct from server working directory
   - Ensure path separators are correct for your platform
   - Check server logs for resolved path information

5. **Cross-platform path issues**:
   - Use forward slashes `/` for Unix/Linux/macOS
   - Use backslashes `\\` for Windows (or forward slashes work too)
   - Test paths on your target platform

### Debug Information

Enable debug logging to see MOTD loading details:

```toml
[logging]
level = "debug"
```

This will show:
- MOTD file loading attempts
- File read results
- Error messages and warnings

## Future Enhancements

Potential future improvements:

1. **Runtime MOTD reloading**: Reload MOTD without server restart
2. **Multiple MOTD support**: Different MOTD for different user types
3. **Template system**: Dynamic MOTD with variables
4. **MOTD scheduling**: Different MOTD for different times
5. **User-specific MOTD**: Personalized MOTD based on user preferences
