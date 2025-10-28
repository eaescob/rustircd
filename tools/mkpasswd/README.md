# mkpasswd - RustIRCD Password Hashing Utility

A command-line tool for generating secure Argon2id password hashes for use in RustIRCD operator authentication.

## Overview

This utility generates password hashes using the **Argon2id** algorithm, which is recommended by security experts for password storage. Argon2id was the winner of the Password Hashing Competition in 2015 and provides strong protection against:

- Rainbow table attacks
- Dictionary attacks
- Brute force attacks
- GPU-accelerated cracking
- Side-channel attacks

Each generated hash includes:
- Random salt (unique per password)
- Algorithm parameters
- The hash itself

All in a single, portable string format.

## Installation

Build the tool from the RustIRCD repository:

```bash
cd tools/mkpasswd
cargo build --release
```

The binary will be available at `target/release/mkpasswd`.

Alternatively, install it to your cargo bin directory:

```bash
cargo install --path .
```

## Usage

### Interactive Mode (Recommended)

Simply run the tool and enter your password when prompted:

```bash
./mkpasswd
```

The password will not be echoed to the screen for security.

### Command-line Mode (Not Recommended)

You can provide the password directly as an argument, but this is **not recommended** because the password will be visible in your shell history and process list:

```bash
./mkpasswd --password "MySecretPassword"
```

### Stdin Mode (For Scripting)

Read password from stdin, useful for automation:

```bash
echo "MySecretPassword" | ./mkpasswd --stdin
```

Or from a file:

```bash
cat password.txt | ./mkpasswd --stdin
```

## Example Output

```
=== Argon2 Password Hash ===
$argon2id$v=19$m=19456,t=2,p=1$Xq3Y8Z9K2L5M7N0P1Q4R3S$vZO8F7E6D5C4B3A2Z1Y0X9W8V7U6T5S4R3Q2P1O0N9M

=== Usage ===
Copy the hash above into your RustIRCD configuration file:
Example: password = "$argon2id$v=19$m=19456,t=2,p=1$Xq3Y8Z9K2L5M7N0P1Q4R3S$vZO8F7E6D5C4B3A2Z1Y0X9W8V7U6T5S4R3Q2P1O0N9M"

The hash format is: $argon2id$v=19$m=...$...$...
This includes the algorithm, parameters, salt, and hash all in one string.
```

## Configuration

Copy the generated hash into your RustIRCD configuration file:

```toml
[[operator]]
name = "admin"
username = "admin"
host = "admin@example.com"
password = "$argon2id$v=19$m=19456,t=2,p=1$..."
flags = ["GlobalKill", "LocalKill", "Rehash", "Die", "ServerLink", "Squit"]
```

## Security Considerations

1. **Never share password hashes** - While hashes are much more secure than plaintext passwords, they should still be treated as sensitive data.

2. **Use strong passwords** - The tool will warn if your password is shorter than 8 characters. Consider using passwords of 12+ characters with mixed case, numbers, and symbols.

3. **Keep your config file secure** - Ensure your RustIRCD configuration file has appropriate permissions:
   ```bash
   chmod 600 config.toml
   ```

4. **Avoid command-line mode** - When possible, use the interactive prompt to avoid password exposure in shell history.

## Algorithm Details

### Argon2id Parameters

The tool uses Argon2's default parameters, which provide a good balance of security and performance:

- **Memory cost (m)**: 19 MiB (19,456 KiB)
- **Time cost (t)**: 2 iterations
- **Parallelism (p)**: 1 thread
- **Salt**: 128-bit random salt (16 bytes)
- **Hash length**: 256 bits (32 bytes)

These parameters are designed to make attacks expensive while keeping verification fast enough for authentication purposes (typically < 100ms).

### Why Argon2id?

- **Memory-hard**: Requires significant RAM, making GPU attacks expensive
- **Time-hard**: Multiple iterations increase computation time
- **Hybrid**: Combines data-dependent (Argon2d) and data-independent (Argon2i) approaches
- **Standardized**: Recommended by OWASP, NIST, and security experts worldwide

## Migrating from SHA-256

If you're upgrading from an older RustIRCD version that used SHA-256:

1. Generate new Argon2 hashes for all operator passwords
2. Update your configuration file with the new hashes
3. Notify operators to use their existing passwords (the passwords haven't changed, only the hashing algorithm)

RustIRCD will automatically detect the hash format and use the appropriate verification method.

## Troubleshooting

### Build Failures

If you encounter build errors, ensure you have:
- Rust 1.70 or later
- Standard build tools (gcc/clang, etc.)

### Permission Denied

If you get permission errors when running the tool:
```bash
chmod +x ./mkpasswd
```

## License

MIT License - Same as RustIRCD
