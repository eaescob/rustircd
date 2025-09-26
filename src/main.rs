//! Rust IRC Daemon - Main binary

use rustircd_core::{Config, Server};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, error};

/// Rust IRC Daemon - A modular IRC server implementation
#[derive(Parser)]
#[command(name = "rustircd")]
#[command(about = "A modular IRC daemon implementation in Rust")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Daemon mode (run in background)
    #[arg(short, long)]
    daemon: bool,
    
    /// Test configuration and exit
    #[arg(long)]
    test_config: bool,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a default configuration file
    Config {
        /// Output file path
        #[arg(short, long, default_value = "config.toml")]
        output: PathBuf,
    },
    /// Show server information
    Info,
    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(&cli.log_level)?;
    
    // Handle subcommands
    if let Some(command) = cli.command {
        match command {
            Commands::Config { output } => {
                generate_config(&output)?;
                return Ok(());
            }
            Commands::Info => {
                show_info();
                return Ok(());
            }
            Commands::Version => {
                show_version();
                return Ok(());
            }
        }
    }
    
    // Load configuration
    let config = if cli.config.exists() {
        info!("Loading configuration from {:?}", cli.config);
        Config::from_file(&cli.config)?
    } else {
        info!("Configuration file not found, using defaults");
        Config::default()
    };
    
    // Test configuration if requested
    if cli.test_config {
        config.validate()?;
        info!("Configuration is valid");
        return Ok(());
    }
    
    // Validate configuration
    config.validate()?;
    
    // Create and initialize server
    let mut server = Server::new(config);
    server.init().await?;
    
    // Start server
    info!("Starting Rust IRC Daemon...");
    server.start().await?;
    
    Ok(())
}

/// Initialize logging
fn init_logging(level: &str) -> anyhow::Result<()> {
    let log_level = match level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();
    
    Ok(())
}

/// Generate default configuration file
fn generate_config(output: &PathBuf) -> anyhow::Result<()> {
    let config = Config::default();
    config.to_file(output)?;
    println!("Generated default configuration file: {:?}", output);
    Ok(())
}

/// Show server information
fn show_info() {
    println!("Rust IRC Daemon");
    println!("===============");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Description: {}", env!("CARGO_PKG_DESCRIPTION"));
    println!("Repository: {}", env!("CARGO_PKG_REPOSITORY"));
    println!("License: {}", env!("CARGO_PKG_LICENSE"));
    println!();
    println!("Features:");
    println!("  - Modular architecture");
    println!("  - RFC 1459 compliance");
    println!("  - IRCv3 support");
    println!("  - TLS/SSL support");
    println!("  - Dynamic module loading");
    println!("  - Services framework");
}

/// Show version information
fn show_version() {
    println!("rustircd {}", env!("CARGO_PKG_VERSION"));
}
