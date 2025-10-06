//! Solanum Extensions Example
//! 
//! This example demonstrates the IP cloaking and OPME extensions
//! based on Solanum's 4.0c implementation.

use rustircd_core::{Config, CoreExtensionManager, User, Message, Client, IpCloakConfigBuilder};
use rustircd_modules::{OpmeModule, OpmeConfigBuilder};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Rust IRC Daemon - Solanum Extensions Example");
    println!("=============================================");
    
    // Load configuration
    let config = Config::from_file("examples/configs/services.toml")?;
    
    // Initialize core extensions
    let core_extensions = CoreExtensionManager::new("services.example.org".to_string());
    core_extensions.initialize().await?;
    
    println!("✓ Core extensions initialized");
    println!();
    
    // Demonstrate IP Cloaking Extension
    println!("IP Cloaking Extension Demo:");
    println!("===========================");
    
    let ip_cloak = core_extensions.get_ip_cloak();
    
    // Test IPv4 cloaking
    let ipv4 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
    let cloaked_ipv4 = ip_cloak.cloak_ip(ipv4).await?;
    println!("IPv4: {} -> {}", ipv4, cloaked_ipv4);
    
    // Test IPv6 cloaking
    let ipv6 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1));
    let cloaked_ipv6 = ip_cloak.cloak_ip(ipv6).await?;
    println!("IPv6: {} -> {}", ipv6, cloaked_ipv6);
    
    // Test localhost preservation
    let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let cloaked_localhost = ip_cloak.cloak_ip(localhost).await?;
    println!("Localhost: {} -> {} (should be preserved)", localhost, cloaked_localhost);
    
    // Test private IP preservation
    let private_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    let cloaked_private = ip_cloak.cloak_ip(private_ip).await?;
    println!("Private IP: {} -> {} (should be preserved)", private_ip, cloaked_private);
    
    // Test hostname preservation
    let should_preserve = ip_cloak.should_preserve_hostname("user.example.com");
    println!("Should preserve 'user.example.com': {}", should_preserve);
    
    let should_not_preserve = ip_cloak.should_preserve_hostname("user.evil.com");
    println!("Should preserve 'user.evil.com': {}", should_not_preserve);
    
    // Get cloaking statistics
    let stats = ip_cloak.get_statistics().await;
    println!("Cloaking stats: {:?}", stats);
    
    println!();
    
    // Demonstrate OPME Extension
    println!("OPME Extension Demo:");
    println!("===================");
    
    let opme_config = OpmeConfigBuilder::new()
        .enabled(true)
        .require_oper(true)
        .log_usage(true)
        .notify_channel(true)
        .rate_limit(true, 5, 300) // 5 uses per 5 minutes
        .build();
    
    let opme_module = OpmeModule::new(opme_config);
    
    // Create a test user with operator privileges
    let mut user = User::new(
        "testoper".to_string(),
        "testoper".to_string(),
        "192.168.1.100".to_string(),
        "Test Operator".to_string(),
        "test.ircd.org".to_string(),
    );
    
    // Grant operator privileges
    use rustircd_core::config::OperatorFlag;
    let mut operator_flags = std::collections::HashSet::new();
    operator_flags.insert(OperatorFlag::GlobalOper);
    user.grant_operator_privileges(operator_flags);
    
    println!("Test user: {}", user.nick);
    println!("Is operator: {}", user.is_operator());
    
    // Create a mock client
    let mut client = Client::new(Uuid::new_v4());
    client.user = Some(user.clone());
    
    // Test OPME command
    let opme_message = Message {
        prefix: Some("testoper!testoper@192.168.1.100".to_string()),
        command: "OPME".to_string(),
        params: vec!["#test".to_string()],
    };
    
    println!("Testing OPME command for channel #test...");
    opme_module.handle_opme(&client, &opme_message, &config).await?;
    
    // Test OPME without channel (should use first channel user is in)
    user.channels.insert("#general".to_string());
    client.user = Some(user.clone());
    
    let opme_message_no_channel = Message {
        prefix: Some("testoper!testoper@192.168.1.100".to_string()),
        command: "OPME".to_string(),
        params: vec![],
    };
    
    println!("Testing OPME command without channel (should use #general)...");
    opme_module.handle_opme(&client, &opme_message_no_channel, &config).await?;
    
    // Test OPME with invalid channel
    let opme_message_invalid = Message {
        prefix: Some("testoper!testoper@192.168.1.100".to_string()),
        command: "OPME".to_string(),
        params: vec!["invalid_channel".to_string()],
    };
    
    println!("Testing OPME command with invalid channel...");
    opme_module.handle_opme(&client, &opme_message_invalid, &config).await?;
    
    // Test OPME with non-operator user
    let mut non_oper_user = User::new(
        "regularuser".to_string(),
        "regularuser".to_string(),
        "192.168.1.101".to_string(),
        "Regular User".to_string(),
        "test.ircd.org".to_string(),
    );
    
    let mut non_oper_client = Client::new(Uuid::new_v4());
    non_oper_client.user = Some(non_oper_user);
    
    let opme_message_non_oper = Message {
        prefix: Some("regularuser!regularuser@192.168.1.101".to_string()),
        command: "OPME".to_string(),
        params: vec!["#test".to_string()],
    };
    
    println!("Testing OPME command with non-operator user...");
    opme_module.handle_opme(&non_oper_client, &opme_message_non_oper, &config).await?;
    
    // Get OPME statistics
    let opme_stats = opme_module.get_statistics().await;
    println!("OPME stats: {:?}", opme_stats);
    
    println!();
    
    // Demonstrate configuration builders
    println!("Configuration Builders Demo:");
    println!("============================");
    
    // IP Cloaking configuration
    let ip_cloak_config = IpCloakConfigBuilder::new()
        .enabled(true)
        .secret_key("my_secret_key_123".to_string())
        .suffix(".myircd.org".to_string())
        .ipv4_cidr(16)
        .ipv6_cidr(32)
        .mac_bits(64)
        .preserve_pattern("*.myircd.org".to_string())
        .build();
    
    println!("IP Cloaking config: {:?}", ip_cloak_config);
    
    // OPME configuration
    let opme_config = OpmeConfigBuilder::new()
        .enabled(true)
        .require_oper(false) // Allow non-operators
        .log_usage(true)
        .notify_channel(false) // Don't notify channel
        .rate_limit(true, 10, 600) // 10 uses per 10 minutes
        .build();
    
    println!("OPME config: {:?}", opme_config);
    
    println!();
    
    // Show extension structure
    println!("Extension Structure:");
    println!("===================");
    println!("core/src/extensions/");
    println!("├── mod.rs                 # Module definitions and manager");
    println!("├── identify_msg.rs        # Identify message extension");
    println!("├── account_tracking.rs    # Account tracking extension");
    println!("├── server_time.rs         # Server time extension");
    println!("├── batch.rs               # Batch extension");
    println!("├── ip_cloak_v2.rs         # IP cloaking extension (Solanum 4.0c style)");
    println!("└── README.md              # Documentation");
    println!();
    println!("modules/src/");
    println!("├── opme.rs                # OPME module (Solanum m_opme.c style)");
    println!("├── oper.rs                # Operator module");
    println!("├── sasl.rs                # SASL module");
    println!("└── ...                    # Other modules");
    println!();
    
    println!("Features implemented:");
    println!("- IP Cloaking with CIDR masking and SHA3 MAC");
    println!("- OPME command for channel operator privileges");
    println!("- Configuration builders for easy setup");
    println!("- Rate limiting and security checks");
    println!("- Comprehensive logging and statistics");
    println!("- Solanum-compatible architecture");
    
    println!("\nExample completed successfully!");
    
    Ok(())
}
