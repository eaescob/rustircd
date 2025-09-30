//! Example demonstrating MOTD path handling for both relative and absolute paths
//! 
//! This example shows how to:
//! 1. Test relative path resolution
//! 2. Test absolute path handling
//! 3. Verify path resolution works correctly
//! 4. Handle path errors gracefully

use rustircd_core::{Config, Server, Result, MotdManager};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCD MOTD Path Test");
    println!("=======================");
    
    // Test path resolution functions
    test_path_resolution().await?;
    
    // Test relative path MOTD loading
    test_relative_path_motd().await?;
    
    // Test absolute path MOTD loading
    test_absolute_path_motd().await?;
    
    // Test server with different path configurations
    test_server_path_configurations().await?;
    
    println!();
    println!("All MOTD path tests completed successfully!");
    println!();
    println!("Path Resolution Summary:");
    println!("=======================");
    println!("✅ Relative paths resolved from server working directory");
    println!("✅ Absolute paths used as-is");
    println!("✅ Cross-platform path detection working");
    println!("✅ Error handling for missing files");
    println!("✅ Proper logging of resolved paths");
    
    Ok(())
}

/// Test the path resolution functions
async fn test_path_resolution() -> Result<()> {
    println!("Testing path resolution functions...");
    
    // Test relative path resolution
    let relative_path = "motd.txt";
    let resolved = MotdManager::resolve_motd_path(relative_path)?;
    println!("  Relative path '{}' resolves to: {}", relative_path, resolved.display());
    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("motd.txt"));
    
    // Test absolute path resolution (Unix-style)
    #[cfg(unix)]
    {
        let absolute_path = "/etc/motd.txt";
        let resolved = MotdManager::resolve_motd_path(absolute_path)?;
        println!("  Absolute path '{}' resolves to: {}", absolute_path, resolved.display());
        assert_eq!(resolved, Path::new("/etc/motd.txt"));
    }
    
    // Test absolute path resolution (Windows-style)
    #[cfg(windows)]
    {
        let absolute_path = "C:\\Windows\\motd.txt";
        let resolved = MotdManager::resolve_motd_path(absolute_path)?;
        println!("  Absolute path '{}' resolves to: {}", absolute_path, resolved.display());
        assert_eq!(resolved, Path::new("C:\\Windows\\motd.txt"));
    }
    
    // Test file existence checking
    let nonexistent = "nonexistent_file_12345.txt";
    assert!(!MotdManager::motd_file_exists(nonexistent));
    println!("  File existence check for '{}': {}", nonexistent, MotdManager::motd_file_exists(nonexistent));
    
    println!("  ✅ Path resolution functions working correctly");
    Ok(())
}

/// Test MOTD loading with relative paths
async fn test_relative_path_motd() -> Result<()> {
    println!("Testing relative path MOTD loading...");
    
    // Create a temporary MOTD file
    let motd_content = "Welcome to Relative Path Test!\nThis is line 2\nThis is line 3";
    fs::write("test_relative_motd.txt", motd_content)?;
    
    // Test loading with relative path
    let mut manager = MotdManager::new();
    let result = manager.load_motd("test_relative_motd.txt").await;
    
    assert!(result.is_ok());
    assert!(manager.is_enabled().await);
    assert_eq!(manager.line_count().await, 3);
    
    println!("  ✅ Relative path MOTD loading successful");
    
    // Clean up
    let _ = fs::remove_file("test_relative_motd.txt");
    
    Ok(())
}

/// Test MOTD loading with absolute paths
async fn test_absolute_path_motd() -> Result<()> {
    println!("Testing absolute path MOTD loading...");
    
    // Create a temporary directory and file
    let temp_dir = TempDir::new()?;
    let motd_file = temp_dir.path().join("test_absolute_motd.txt");
    
    let motd_content = "Welcome to Absolute Path Test!\nThis is line 2\nThis is line 3";
    fs::write(&motd_file, motd_content)?;
    
    // Test loading with absolute path
    let mut manager = MotdManager::new();
    let result = manager.load_motd(motd_file.to_str().unwrap()).await;
    
    assert!(result.is_ok());
    assert!(manager.is_enabled().await);
    assert_eq!(manager.line_count().await, 3);
    
    println!("  ✅ Absolute path MOTD loading successful");
    
    // Clean up (temp_dir will be cleaned up automatically)
    
    Ok(())
}

/// Test server configurations with different path types
async fn test_server_path_configurations() -> Result<()> {
    println!("Testing server path configurations...");
    
    // Create a test MOTD file
    let motd_content = "Welcome to Path Test Server!\nThis server tests path handling.";
    fs::write("server_test_motd.txt", motd_content)?;
    
    // Test 1: Relative path configuration
    println!("  Testing relative path configuration...");
    let mut config = Config::default();
    config.server.name = "path-test.example.com".to_string();
    config.server.motd_file = Some("server_test_motd.txt".to_string());
    config.connection.ports.clear();
    
    let mut server = Server::new(config);
    let result = server.init().await;
    assert!(result.is_ok());
    println!("    ✅ Relative path server configuration successful");
    
    // Test 2: Absolute path configuration
    println!("  Testing absolute path configuration...");
    let temp_dir = TempDir::new()?;
    let absolute_motd_file = temp_dir.path().join("absolute_server_motd.txt");
    fs::write(&absolute_motd_file, motd_content)?;
    
    let mut config2 = Config::default();
    config2.server.name = "absolute-path-test.example.com".to_string();
    config2.server.motd_file = Some(absolute_motd_file.to_str().unwrap().to_string());
    config2.connection.ports.clear();
    
    let mut server2 = Server::new(config2);
    let result2 = server2.init().await;
    assert!(result2.is_ok());
    println!("    ✅ Absolute path server configuration successful");
    
    // Test 3: No MOTD configuration
    println!("  Testing no MOTD configuration...");
    let mut config3 = Config::default();
    config3.server.name = "no-motd-test.example.com".to_string();
    config3.server.motd_file = None;
    config3.connection.ports.clear();
    
    let mut server3 = Server::new(config3);
    let result3 = server3.init().await;
    assert!(result3.is_ok());
    println!("    ✅ No MOTD server configuration successful");
    
    // Clean up
    let _ = fs::remove_file("server_test_motd.txt");
    
    println!("  ✅ All server path configurations working correctly");
    
    Ok(())
}

/// Helper function to demonstrate path resolution examples
#[allow(dead_code)]
fn show_path_examples() {
    println!("MOTD Path Examples:");
    println!("==================");
    println!();
    println!("Relative Paths (resolved from server working directory):");
    println!("  motd_file = \"motd.txt\"");
    println!("  motd_file = \"config/messages/motd.txt\"");
    println!("  motd_file = \"data/text/welcome.txt\"");
    println!("  motd_file = \"../shared/motd.txt\"");
    println!();
    println!("Absolute Paths (used as-is):");
    println!();
    println!("Unix/Linux/macOS:");
    println!("  motd_file = \"/etc/rustircd/motd.txt\"");
    println!("  motd_file = \"/usr/local/etc/rustircd/motd.txt\"");
    println!("  motd_file = \"/opt/rustircd/config/motd.txt\"");
    println!("  motd_file = \"/home/user/irc/motd.txt\"");
    println!();
    println!("Windows:");
    println!("  motd_file = \"C:\\\\Program Files\\\\RustIRCd\\\\motd.txt\"");
    println!("  motd_file = \"D:\\\\IRC\\\\motd.txt\"");
    println!("  motd_file = \"C:\\\\Users\\\\Username\\\\Documents\\\\motd.txt\"");
    println!("  motd_file = \"\\\\server\\\\share\\\\motd.txt\"");
    println!();
    println!("Path Resolution Behavior:");
    println!("  • Relative paths: resolved from current working directory");
    println!("  • Absolute paths: used exactly as specified");
    println!("  • All resolved paths logged for debugging");
    println!("  • Cross-platform path detection automatic");
}
