# Configurable IRC Numeric Replies

RustIRCd supports configurable IRC numeric replies, allowing server administrators to customize the messages sent to clients while maintaining RFC 1459 compliance.

## Overview

The configurable replies system allows you to:
- Customize any IRC numeric reply message
- Use placeholders for dynamic content
- Maintain RFC compliance while adding personality
- Fall back to sensible defaults for unconfigured replies

## Configuration

### Basic Setup

1. Create a `replies.toml` file in your server directory (same directory as `config.toml`)
2. Define your custom replies using the TOML format
3. Restart the server to load the new replies

### File Structure

**Note**: Server information (name, version, description, admin details) is now configured in the main `config.toml` file under the `[server]` section. The `replies.toml` file only contains the custom reply templates.

```toml
# replies.toml - Only contains custom reply templates
[replies.001]
code = 001
text = "Welcome to {server_name}, {nick}! You are now connected!"
description = "RPL_WELCOME - Custom welcome message"

[replies.401]
code = 401
text = "{nick} :Sorry, I couldn't find that nick/channel! 🤔"
description = "ERR_NOSUCHNICK - Custom error with emoji"
```

Server information is configured in `config.toml`:
```toml
# config.toml - Main server configuration
[server]
name = "MyIRCd"
version = "1.0.0"
description = "My Custom IRC Daemon"
created = "2025-01-01"
admin_email = "admin@example.com"
admin_location1 = "My IRC Network"
admin_location2 = "https://myircd.net"
```

## Placeholders

The following placeholders are available in reply templates:

### Server Information
- `{server_name}` - Server name
- `{server_version}` - Server version
- `{server_description}` - Server description
- `{server_created}` - Server creation date
- `{admin_email}` - Administrator email
- `{admin_location1}` - Admin location line 1
- `{admin_location2}` - Admin location line 2

### User Information
- `{nick}` - User nickname
- `{user}` - Username
- `{host}` - Hostname
- `{realname}` - Real name
- `{target}` - Target user/channel

### Channel Information
- `{channel}` - Channel name
- `{topic}` - Channel topic
- `{reason}` - Reason for action
- `{count}` - Count/number
- `{info}` - Additional information

### Custom Parameters
- `{param0}`, `{param1}`, etc. - Custom parameters passed to the reply

## Example Customizations

### Welcome Message
```toml
[replies.001]
code = 001
text = "Welcome to {server_name}, {nick}! You are now connected to the best IRC network! 🚀"
```

### WHOIS Information
```toml
[replies.311]
code = 311
text = "{nick} {user} {host} * :{realname} (Last seen: {last_seen})"
```

### Error Messages with Help
```toml
[replies.403]
code = 403
text = "{channel} :That channel doesn't exist! Want to create it? Use /join {channel}"
```

### Operator Messages
```toml
[replies.381]
code = 381
text = ":Congratulations! You are now a super awesome IRC operator! 🎉"
```

## Default Behavior

- If no `replies.toml` file exists, the server uses built-in RFC 1459 compliant defaults
- If a reply is not defined in the custom file, the default is used
- The system gracefully handles missing placeholders by leaving them as-is

## Server Information

Server information (name, version, description, admin details) is now configured in the main `config.toml` file under the `[server]` section. This information is automatically used in reply templates and helps maintain a cohesive user experience across your IRC daemon.

## Best Practices

1. **Test Your Replies**: Always test custom replies to ensure they work correctly
2. **Maintain RFC Compliance**: While you can customize messages, ensure they still convey the correct information
3. **Use Placeholders**: Leverage placeholders for dynamic content instead of hardcoded values
4. **Keep It Professional**: While personality is good, maintain professionalism for error messages
5. **Document Changes**: Keep track of customizations for future maintenance

## Troubleshooting

### Common Issues

1. **Replies Not Loading**: Check that `replies.toml` is in the correct directory and has valid TOML syntax
2. **Placeholders Not Working**: Ensure placeholders are spelled correctly and use curly braces `{}`
3. **Server Won't Start**: Check the server logs for TOML parsing errors

### Debugging

Enable debug logging to see which replies are being loaded:
```bash
rustircd --log-level debug
```

## Examples

See `examples/custom_replies.toml` for a comprehensive example with various customizations including:
- Personalized welcome messages
- Helpful error messages with suggestions
- Emoji-enhanced operator messages
- Custom WHOIS information
- Channel-specific messages

## Technical Details

- Replies are loaded at server startup
- Custom replies override defaults for the same numeric code
- The system uses TOML for configuration (same as main config)
- All placeholders are case-sensitive
- Empty or malformed reply configurations fall back to defaults
