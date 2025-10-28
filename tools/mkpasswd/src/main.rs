use anyhow::{Context, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use clap::Parser;
use rand::rngs::OsRng;

/// RustIRCD password hashing utility using Argon2
///
/// This tool generates secure password hashes using the Argon2id algorithm
/// for use in RustIRCD operator authentication configuration.
#[derive(Parser, Debug)]
#[command(
    name = "mkpasswd",
    version,
    about = "Generate Argon2 password hashes for RustIRCD operator authentication",
    long_about = "This utility generates secure password hashes using the Argon2id \
                  algorithm. The resulting hash can be used in RustIRCD's configuration \
                  file for operator authentication.\n\n\
                  By default, the tool prompts for a password securely (without echoing). \
                  Alternatively, you can provide a password via command-line argument \
                  (not recommended for security reasons) or pipe it via stdin."
)]
struct Cli {
    /// Password to hash (not recommended - use interactive prompt instead)
    #[arg(short, long, conflicts_with = "stdin")]
    password: Option<String>,

    /// Read password from stdin (useful for scripting)
    #[arg(short, long)]
    stdin: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Get password from appropriate source
    let password = if let Some(pwd) = cli.password {
        eprintln!("Warning: Providing passwords via command-line arguments is insecure.");
        eprintln!("Consider using the interactive prompt or stdin instead.\n");
        pwd
    } else if cli.stdin {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read password from stdin")?;
        buffer.trim().to_string()
    } else {
        // Interactive prompt (secure, doesn't echo password)
        eprintln!("Enter password: ");
        rpassword::read_password().context("Failed to read password")?
    };

    // Validate password
    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    if password.len() < 8 {
        eprintln!("Warning: Password is less than 8 characters. Consider using a stronger password.\n");
    }

    // Generate salt using cryptographically secure random number generator
    let salt = SaltString::generate(&mut OsRng);

    // Use Argon2id with default parameters (recommended)
    let argon2 = Argon2::default();

    // Hash the password
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .context("Failed to hash password")?
        .to_string();

    // Output the hash
    println!("\n=== Argon2 Password Hash ===");
    println!("{}", password_hash);
    println!("\n=== Usage ===");
    println!("Copy the hash above into your RustIRCD configuration file:");
    println!("Example: password = \"{}\"", password_hash);
    println!("\nThe hash format is: $argon2id$v=19$m=...$...$...");
    println!("This includes the algorithm, parameters, salt, and hash all in one string.\n");

    Ok(())
}
