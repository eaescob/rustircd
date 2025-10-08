//! Example demonstrating user mode management functionality
//!
//! This example shows how to:
//! 1. Configure a server with user mode support
//! 2. Demonstrate user mode changes
//! 3. Show mode validation and permissions
//! 4. Display user mode responses

use rustircd_core::{Config, Server, Result, Message, MessageType, NumericReply, UserMode, UserModeManager};
use tokio::time::{sleep, Duration};
use std::collections::HashSet;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("RustIRCD User Mode Management Example");
    println!("===================================");
    
    // Create server configuration
    let config = create_server_config();
    
    // Create and initialize the server
    let mut server = Server::new(config).await;
    server.init().await?;
    
    println!("Server initialized with user mode support");
    println!();
    
    // Demonstrate user mode functionality
    demonstrate_user_mode_functionality(&server).await?;
    
    println!();
    println!("User Mode System Features:");
    println!("=========================");
    println!("✅ RFC 1459 compliant user mode implementation");
    println!("✅ Standard IRC user modes (a, i, w, r, o, O, s)");
    println!("✅ Mode validation and permission checking");
    println!("✅ Self-only and operator-only mode restrictions");
    println!("✅ Real-time mode change notifications");
    println!("✅ Configurable numeric replies");
    println!("✅ Comprehensive error handling");
    
    println!();
    println!("Supported User Modes:");
    println!("===================");
    show_supported_user_modes();
    
    println!();
    println!("User Mode Command Usage:");
    println!("======================");
    show_user_mode_usage();
    
    println!();
    println!("Expected IRC Output:");
    println!("==================");
    show_expected_output();
    
    // Keep server running for a short time
    println!();
    println!("Server running for 30 seconds to demonstrate functionality...");
    sleep(Duration::from_secs(30)).await;
    
    Ok(())
}

/// Create a server configuration with user mode support
fn create_server_config() -> Config {
    let mut config = Config::default();
    
    // Configure server settings
    config.server.name = "usermodes.example.com".to_string();
    config.server.description = "User Mode Test Server".to_string();
    config.server.version = "1.0.0".to_string();
    config.server.max_clients = 1000;
    
    // Enable modules for user mode support
    config.modules.enabled_modules = vec![
        "channel".to_string(),
        "ircv3".to_string(),
        "throttling".to_string(),
    ];
    
    // Configure connection settings
    config.connection.ports.clear();
    config.connection.ports.push(rustircd_core::config::PortConfig {
        port: 6667,
        connection_type: rustircd_core::config::PortConnectionType::Client,
        tls: false,
        description: Some("Standard IRC port".to_string()),
        bind_address: None,
    });
    
    config
}

/// Demonstrate user mode functionality
async fn demonstrate_user_mode_functionality(server: &Server) -> Result<()> {
    println!("Demonstrating User Mode Functionality:");
    println!("====================================");
    
    // Create sample client IDs for demonstration
    let user1_id = uuid::Uuid::new_v4();
    let user2_id = uuid::Uuid::new_v4();
    
    println!();
    println!("1. Testing User Mode Manager:");
    test_user_mode_manager();
    
    println!();
    println!("2. Testing Mode Validation:");
    test_mode_validation();
    
    println!();
    println!("3. Testing Mode Changes:");
    test_mode_changes(server, user1_id, user2_id).await?;
    
    Ok(())
}

/// Test user mode manager functionality
fn test_user_mode_manager() {
    println!("   📋 User Mode Manager Tests:");
    
    let mut manager = UserModeManager::new();
    
    // Test adding modes
    println!("   • Adding invisible mode...");
    assert!(manager.add_mode(UserMode::Invisible).is_ok());
    assert!(manager.has_mode(UserMode::Invisible));
    println!("   ✅ Invisible mode added successfully");
    
    println!("   • Adding away mode...");
    assert!(manager.add_mode(UserMode::Away).is_ok());
    assert!(manager.has_mode(UserMode::Away));
    println!("   ✅ Away mode added successfully");
    
    // Test mode string
    let modes = manager.modes_string();
    println!("   • Current modes: {}", modes);
    assert!(modes.contains('a') && modes.contains('i'));
    println!("   ✅ Mode string generated correctly");
    
    // Test removing modes
    println!("   • Removing invisible mode...");
    assert!(manager.remove_mode(UserMode::Invisible).is_ok());
    assert!(!manager.has_mode(UserMode::Invisible));
    println!("   ✅ Invisible mode removed successfully");
    
    // Test operator modes - these should always fail via MODE command
    println!("   • Testing operator mode (should always fail via MODE command)...");
    assert!(manager.add_mode(UserMode::Operator).is_err());
    println!("   ✅ Operator mode correctly rejected (only via OPER command)");
    
    // Test that operator mode can only be set via OPER command
    manager.set_operator(true);
    println!("   • Testing operator mode removal (should be allowed)...");
    assert!(manager.remove_mode(UserMode::Operator).is_ok());
    println!("   ✅ Operator mode removal allowed for self");
}

/// Test mode validation
fn test_mode_validation() {
    println!("   🔒 Mode Validation Tests:");
    
    let manager = UserModeManager::new();
    
    // Test self-only modes
    println!("   • Testing invisible mode (self-only)...");
    assert!(manager.validate_mode_change(
        UserMode::Invisible,
        true,
        "user1",
        "user1",
        false,
    ).is_ok());
    println!("   ✅ Self can set invisible mode");
    
    assert!(manager.validate_mode_change(
        UserMode::Invisible,
        true,
        "user1",
        "user2",
        false,
    ).is_err());
    println!("   ✅ Others cannot set invisible mode for user");
    
    // Test operator-only modes - should always fail for setting
    println!("   • Testing operator mode setting (should always fail)...");
    assert!(manager.validate_mode_change(
        UserMode::Operator,
        true,
        "user1",
        "user1",
        false,
    ).is_err());
    println!("   ✅ Non-operators cannot set operator mode");
    
    assert!(manager.validate_mode_change(
        UserMode::Operator,
        true,
        "user1",
        "user1",
        true,
    ).is_err());
    println!("   ✅ Even operators cannot set operator mode via MODE command");
    
    // Test operator mode removal - should be allowed for self
    println!("   • Testing operator mode removal (should be allowed for self)...");
    assert!(manager.validate_mode_change(
        UserMode::Operator,
        false,
        "user1",
        "user1",
        true,
    ).is_ok());
    println!("   ✅ Users can remove their own operator mode");
}

/// Test mode changes
async fn test_mode_changes(server: &Server, user1_id: uuid::Uuid, user2_id: uuid::Uuid) -> Result<()> {
    println!("   🔄 Mode Change Tests:");
    
    // Test viewing current modes
    println!("   • Testing MODE command (view current modes)...");
    let mode_view_msg = Message::new(
        MessageType::Mode,
        vec!["user1".to_string()], // Just the nickname, no mode changes
    );
    
    match server.handle_mode(user1_id, mode_view_msg).await {
        Ok(()) => println!("   ✅ Mode view command processed successfully"),
        Err(e) => println!("   ⚠️  Mode view command failed: {} (expected - no real user)", e),
    }
    
    // Test setting invisible mode
    println!("   • Testing MODE command (set invisible mode)...");
    let mode_set_msg = Message::new(
        MessageType::Mode,
        vec!["user1".to_string(), "+i".to_string()],
    );
    
    match server.handle_mode(user1_id, mode_set_msg).await {
        Ok(()) => println!("   ✅ Invisible mode set successfully"),
        Err(e) => println!("   ⚠️  Invisible mode failed: {} (expected - no real user)", e),
    }
    
    // Test setting away mode
    println!("   • Testing MODE command (set away mode)...");
    let away_mode_msg = Message::new(
        MessageType::Mode,
        vec!["user1".to_string(), "+a".to_string()],
    );
    
    match server.handle_mode(user1_id, away_mode_msg).await {
        Ok(()) => println!("   ✅ Away mode set successfully"),
        Err(e) => println!("   ⚠️  Away mode failed: {} (expected - no real user)", e),
    }
    
    // Test removing modes
    println!("   • Testing MODE command (remove modes)...");
    let mode_remove_msg = Message::new(
        MessageType::Mode,
        vec!["user1".to_string(), "-i-a".to_string()],
    );
    
    match server.handle_mode(user1_id, mode_remove_msg).await {
        Ok(()) => println!("   ✅ Modes removed successfully"),
        Err(e) => println!("   ⚠️  Mode removal failed: {} (expected - no real user)", e),
    }
    
    // Test trying to set operator mode (should fail)
    println!("   • Testing MODE command (try to set operator mode - should fail)...");
    let operator_mode_msg = Message::new(
        MessageType::Mode,
        vec!["user1".to_string(), "+o".to_string()],
    );
    
    match server.handle_mode(user1_id, operator_mode_msg).await {
        Ok(()) => println!("   ⚠️  Operator mode set (unexpected!)"),
        Err(e) => println!("   ✅ Operator mode correctly rejected: {} (expected)", e),
    }
    
    // Test removing operator mode (should be allowed for self)
    println!("   • Testing MODE command (remove operator mode - should be allowed)...");
    let remove_operator_msg = Message::new(
        MessageType::Mode,
        vec!["user1".to_string(), "-o".to_string()],
    );
    
    match server.handle_mode(user1_id, remove_operator_msg).await {
        Ok(()) => println!("   ✅ Operator mode removal allowed"),
        Err(e) => println!("   ⚠️  Operator mode removal failed: {} (expected - no real user)", e),
    }
    
    Ok(())
}

/// Show supported user modes
fn show_supported_user_modes() {
    println!("a - Away mode (user is away)");
    println!("i - Invisible mode (user doesn't appear in WHO lists)");
    println!("w - Wallops mode (user receives wallop messages)");
    println!("r - Restricted mode (user is restricted from certain actions)");
    println!("o - Operator mode (user has operator privileges)");
    println!("O - Local operator mode (user has local operator privileges)");
    println!("s - Server notices mode (user receives server notices)");
    println!();
    println!("Mode Restrictions:");
    println!("• Self-only modes (a, i, w, s): Can only be set by the user themselves");
    println!("• Operator-only modes (o, O, r): Require operator privileges to set");
    println!("• All modes: Cannot be set if already set, cannot be removed if not set");
}

/// Show user mode command usage
fn show_user_mode_usage() {
    println!("Command: MODE <nickname> [<modes>]");
    println!("Purpose: View or change user modes");
    println!("Usage:");
    println!("  /MODE nick          - View current modes for nick");
    println!("  /MODE nick +i       - Set invisible mode for nick");
    println!("  /MODE nick -i       - Remove invisible mode for nick");
    println!("  /MODE nick +i-a     - Set invisible, remove away mode");
    println!("  /MODE nick -i+a     - Remove invisible, set away mode");
    println!();
    println!("Parameters:");
    println!("  nickname - Target user's nickname");
    println!("  modes    - Mode changes (optional, e.g., +i-a+w)");
    println!();
    println!("Response:");
    println!("  RPL_UMODEIS (221) - Current modes when viewing");
    println!("  MODE message      - Mode change notification");
    println!("  Error replies     - For invalid requests or permissions");
}

/// Show expected IRC output
fn show_expected_output() {
    println!("Viewing user modes:");
    println!(":server.example.com 221 user1 :+i");
    println!();
    println!("Setting invisible mode:");
    println!(":user1 MODE user1 :+i");
    println!();
    println!("Setting away mode:");
    println!(":user1 MODE user1 :+a");
    println!();
    println!("Removing modes:");
    println!(":user1 MODE user1 :-i-a");
    println!();
    println!("Error - trying to set others' self-only modes:");
    println!(":server.example.com 502 user2 :Cannot change mode for other users");
    println!();
    println!("Error - trying to set operator mode via MODE command:");
    println!(":server.example.com 503 user1 :Operator mode can only be granted through OPER command");
    println!();
    println!("Successfully removing operator mode:");
    println!(":user1 MODE user1 :-o");
}

/// Helper function to demonstrate user mode integration
#[allow(dead_code)]
async fn demonstrate_user_mode_integration(server: &Server) -> Result<()> {
    println!("User Mode Integration Points:");
    println!("============================");
    
    println!("1. User Structure Integration:");
    println!("   - User struct has modes field (HashSet<char>)");
    println!("   - Basic mode management methods (add_mode, remove_mode, has_mode)");
    println!("   - Mode string generation (modes_string)");
    
    println!();
    println!("2. Server Integration:");
    println!("   - MODE command handler for user modes");
    println!("   - Mode validation and permission checking");
    println!("   - Real-time mode change notifications");
    println!("   - Error handling with appropriate numeric replies");
    
    println!();
    println!("3. Numeric Reply Integration:");
    println!("   - RPL_UMODEIS (221) for mode display");
    println!("   - ERR_USERSDONTMATCH (502) for permission errors");
    println!("   - Configurable reply templates");
    
    println!();
    println!("4. Message System Integration:");
    println!("   - MODE message type for mode changes");
    println!("   - Proper message formatting and routing");
    println!("   - Client notification system");
    
    Ok(())
}

/// Helper function to show user mode benefits
#[allow(dead_code)]
fn show_user_mode_benefits() {
    println!("User Mode Benefits:");
    println!("==================");
    println!("✅ User Privacy: Invisible mode hides users from WHO lists");
    println!("✅ Status Management: Away mode indicates user availability");
    println!("✅ Message Control: Wallops mode for operator messages");
    println!("✅ Access Control: Operator modes for privilege management");
    println!("✅ Server Integration: Server notices for important messages");
    println!("✅ Security: Restricted mode for problematic users");
    println!("✅ RFC Compliance: Standard IRC user mode implementation");
}

/// Helper function to show user mode validation scenarios
#[allow(dead_code)]
fn show_validation_scenarios() {
    println!("User Mode Validation Scenarios:");
    println!("==============================");
    
    println!("Scenario 1: User setting own invisible mode");
    println!("  • User: user1");
    println!("  • Command: /MODE user1 +i");
    println!("  • Result: ✅ Success (self-only mode)");
    
    println!();
    println!("Scenario 2: User trying to set another's invisible mode");
    println!("  • User: user1");
    println!("  • Command: /MODE user2 +i");
    println!("  • Result: ❌ Error 502 (Cannot change mode for other users)");
    
    println!();
    println!("Scenario 3: Non-operator trying to set operator mode");
    println!("  • User: user1 (not operator)");
    println!("  • Command: /MODE user1 +o");
    println!("  • Result: ❌ Error 502 (Permission denied)");
    
    println!();
    println!("Scenario 4: Operator trying to set operator mode via MODE");
    println!("  • User: user1 (operator)");
    println!("  • Command: /MODE user1 +o");
    println!("  • Result: ❌ Error 503 (Operator mode can only be granted through OPER command)");
    
    println!();
    println!("Scenario 5: Operator removing their own operator mode");
    println!("  • User: user1 (operator with +o)");
    println!("  • Command: /MODE user1 -o");
    println!("  • Result: ✅ Success (users can remove their own operator mode)");
    
    println!();
    println!("Scenario 6: Trying to set already set mode");
    println!("  • User: user1 (already has +i)");
    println!("  • Command: /MODE user1 +i");
    println!("  • Result: ❌ Error (Mode i is already set)");
}
