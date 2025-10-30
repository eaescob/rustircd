//! Rust IRC Daemon - Main binary

use rustircd_core::{Config, Server};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

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
        // Test configuration if requested
        if cli.test_config {
            return validate_config(&cli.config);
        }

        info!("Loading configuration from {:?}", cli.config);
        Config::from_file(&cli.config)?
    } else {
        if cli.test_config {
            eprintln!("❌ Configuration file not found: {:?}", cli.config);
            std::process::exit(1);
        }
        info!("Configuration file not found, using defaults");
        Config::default()
    };
    
    // Validate configuration
    config.validate()?;
    
    // Create and initialize server
    let config_path = cli.config.to_string_lossy().to_string();
    let mut server = Server::new_with_config_path(config, config_path).await;
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

/// Validate configuration file and display detailed results
fn validate_config(config_path: &PathBuf) -> anyhow::Result<()> {
    println!("🔍 Validating configuration file: {:?}", config_path);
    println!();

    // Try to load the configuration
    let config = match Config::from_file(config_path) {
        Ok(config) => {
            println!("✅ Configuration file parsed successfully");
            config
        }
        Err(e) => {
            eprintln!("❌ Failed to parse configuration file");
            eprintln!();
            eprintln!("Error details:");
            eprintln!("{}", e);
            eprintln!();

            // Try to extract line number information from TOML parse errors
            let error_msg = format!("{}", e);
            if error_msg.contains("line") || error_msg.contains("column") {
                eprintln!("💡 Tip: Check the line and column numbers mentioned above in your config file");
            }

            std::process::exit(1);
        }
    };

    println!();
    println!("🔍 Running validation checks...");
    println!();

    // Validate the configuration
    match config.validate() {
        Ok(_) => {
            println!("✅ Configuration validation passed!");
            println!();
            println!("Summary:");
            println!("  Server name: {}", config.server.name);
            println!("  Network name: {}", config.network.name);
            println!("  Ports configured: {}", config.connection.ports.len());
            println!("  TLS enabled: {}", config.security.tls.enabled);

            if !config.modules.enabled_modules.is_empty() {
                println!("  Modules: {}", config.modules.enabled_modules.join(", "));
            }

            if !config.services.services.is_empty() {
                let enabled_services: Vec<_> = config.services.services
                    .iter()
                    .filter(|s| s.enabled)
                    .map(|s| s.name.as_str())
                    .collect();
                if !enabled_services.is_empty() {
                    println!("  Services: {}", enabled_services.join(", "));
                }
            }

            println!();
            println!("✅ Configuration is ready to use!");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Configuration validation failed");
            eprintln!();
            eprintln!("Validation errors:");
            eprintln!("{}", e);
            eprintln!();
            eprintln!("💡 Please fix the errors above and try again");
            std::process::exit(1);
        }
    }
}
