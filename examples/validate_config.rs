//! Standalone configuration validation tool
//!
//! This tool validates RustIRCD configuration files and provides detailed
//! error messages, warnings, and suggestions for improvement.
//!
//! Usage:
//!   cargo run --example validate_config
//!   cargo run --example validate_config -- /path/to/config.toml
//!   cargo run --example validate_config -- --help

use rustircd_core::{Config, ConfigValidator, print_validation_result};
use std::env;
use std::process;

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        print_help();
        return;
    }

    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "config.toml"
    };

    println!("RustIRCD Configuration Validator");
    println!("{}", "=".repeat(80));
    println!("Validating: {}\n", config_path);

    // Load configuration
    let config = match Config::from_file(config_path) {
        Ok(cfg) => {
            println!("✓ Configuration file loaded successfully");
            cfg
        }
        Err(e) => {
            println!("✗ Failed to load configuration file:");
            println!("  Error: {}", e);
            println!("\nPossible issues:");
            println!("  • File does not exist");
            println!("  • Invalid TOML syntax");
            println!("  • Missing required fields");
            println!("  • Incorrect data types");
            println!("\nRun with --help for more information.");
            process::exit(1);
        }
    };

    // Validate configuration
    let validator = ConfigValidator::new(config);
    let result = validator.validate();

    // Print results
    print_validation_result(&result);

    // Exit with appropriate code
    if result.is_valid {
        if result.warnings.is_empty() {
            println!("\n✓ Success! Configuration is perfect and ready to use.");
            process::exit(0);
        } else {
            println!("\n⚠ Configuration is valid but has warnings.");
            println!("  Review the warnings above to improve your configuration.");
            process::exit(0);
        }
    } else {
        println!("\n✗ Configuration has errors that must be fixed before starting the server.");
        println!("  Please review the errors above and correct your configuration.");
        process::exit(1);
    }
}

fn print_help() {
    println!("RustIRCD Configuration Validator");
    println!();
    println!("USAGE:");
    println!("    cargo run --example validate_config [CONFIG_FILE]");
    println!();
    println!("ARGUMENTS:");
    println!("    CONFIG_FILE    Path to configuration file (default: config.toml)");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help     Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    # Validate default config.toml");
    println!("    cargo run --example validate_config");
    println!();
    println!("    # Validate specific configuration file");
    println!("    cargo run --example validate_config -- /path/to/config.toml");
    println!();
    println!("    # Validate example configuration");
    println!("    cargo run --example validate_config -- examples/configs/config.example.toml");
    println!();
    println!("VALIDATION CHECKS:");
    println!("    • Required fields present");
    println!("    • Valid values for all settings");
    println!("    • Cross-references (classes, modules, etc.)");
    println!("    • File paths exist");
    println!("    • Security best practices");
    println!("    • Proper configuration ordering");
    println!("    • No duplicate definitions");
    println!();
    println!("EXIT CODES:");
    println!("    0  Configuration is valid (may have warnings)");
    println!("    1  Configuration has errors or failed to load");
}

