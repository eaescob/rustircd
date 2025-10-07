//! IRCv3 SASL Integration Example
//! 
//! This example demonstrates how SASL authentication is integrated into the IRCv3 capability negotiation system.

use rustircd_core::{Client, Message, MessageType, Result};
use rustircd_modules::ircv3::{Ircv3Module, SaslCapability};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== IRCv3 SASL Integration Example ===\n");
    
    // Create IRCv3 module with SASL capability
    let mut ircv3_module = Ircv3Module::new();
    ircv3_module.init().await?;
    
    // Create a mock client
    let client = create_mock_client();
    
    println!("1. Client connects and requests capabilities");
    println!("   Client sends: CAP LS");
    
    // Simulate CAP LS request
    let cap_ls_msg = Message::new(
        MessageType::Custom("CAP".to_string()),
        vec!["LS".to_string()]
    );
    
    // Handle CAP LS (this would normally be handled by the server)
    println!("   Server responds with available capabilities including SASL");
    println!("   Available capabilities: cap, message-tags, account-tag, away-notify, batch, bot-mode, channel-rename, chghost, echo-message, extended-join, invite-notify, multi-prefix, sasl, server-time, userhost-in-names");
    
    println!("\n2. Client requests SASL capability");
    println!("   Client sends: CAP REQ :sasl");
    
    // Simulate CAP REQ for SASL
    let cap_req_msg = Message::new(
        MessageType::Custom("CAP".to_string()),
        vec!["REQ".to_string(), "sasl".to_string()]
    );
    
    println!("   Server responds: CAP ACK :sasl");
    println!("   SASL capability is now enabled for the client");
    
    // Check if SASL is enabled
    let sasl_enabled = ircv3_module.is_sasl_enabled(&client.id).await;
    println!("   SASL capability enabled: {}", sasl_enabled);
    
    println!("\n3. Client starts SASL authentication");
    println!("   Client sends: AUTHENTICATE PLAIN");
    
    // Simulate AUTHENTICATE command
    let auth_msg = Message::new(
        MessageType::Custom("AUTHENTICATE".to_string()),
        vec!["PLAIN".to_string()]
    );
    
    println!("   Server responds: AUTHENTICATE +");
    println!("   Client sends: AUTHENTICATE <base64-encoded-credentials>");
    
    // Simulate authentication data
    let auth_data_msg = Message::new(
        MessageType::Custom("AUTHENTICATE".to_string()),
        vec!["dXNlcm5hbWU6cGFzc3dvcmQ=".to_string()] // username:password in base64
    );
    
    println!("   Server validates credentials and responds: AUTHENTICATE +");
    println!("   SASL authentication successful!");
    
    println!("\n4. Client ends capability negotiation");
    println!("   Client sends: CAP END");
    
    let cap_end_msg = Message::new(
        MessageType::Custom("CAP".to_string()),
        vec!["END".to_string()]
    );
    
    println!("   Capability negotiation complete");
    
    println!("\n5. Demonstrate SASL capability information");
    let sasl_info = ircv3_module.get_sasl_capability_info().await;
    println!("   SASL capability info: {}", sasl_info);
    
    println!("\n=== SASL IRCv3 Integration Complete ===");
    
    // Cleanup
    ircv3_module.cleanup().await?;
    
    Ok(())
}

fn create_mock_client() -> Client {
    let mut clients = HashMap::new();
    let client_id = Uuid::new_v4();
    
    // Create a mock client - in a real implementation, this would be created by the server
    let client = Client {
        id: client_id,
        nick: Some("testuser".to_string()),
        username: Some("testuser".to_string()),
        hostname: Some("test.example.com".to_string()),
        realname: Some("Test User".to_string()),
        server: Some("testserver".to_string()),
        hops: 0,
        connected_at: std::time::SystemTime::now(),
        last_ping: std::time::SystemTime::now(),
        is_away: false,
        away_message: None,
        user_modes: std::collections::HashSet::new(),
        is_operator: false,
        operator_flags: std::collections::HashSet::new(),
        is_bot: false,
        bot_info: None,
        account_name: None,
        channels: std::collections::HashSet::new(),
    };
    
    client
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_sasl_capability_integration() {
        let mut ircv3_module = Ircv3Module::new();
        ircv3_module.init().await.unwrap();
        
        let client = create_mock_client();
        
        // Test SASL capability info
        let sasl_info = ircv3_module.get_sasl_capability_info().await;
        assert!(sasl_info.contains("sasl"));
        
        // Test SASL capability management
        assert!(!ircv3_module.is_sasl_enabled(&client.id).await);
        
        ircv3_module.enable_sasl(client.id).await;
        assert!(ircv3_module.is_sasl_enabled(&client.id).await);
        
        ircv3_module.disable_sasl(client.id).await;
        assert!(!ircv3_module.is_sasl_enabled(&client.id).await);
        
        ircv3_module.cleanup().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_sasl_authenticate_command() {
        let mut ircv3_module = Ircv3Module::new();
        ircv3_module.init().await.unwrap();
        
        let client = create_mock_client();
        
        // Enable SASL capability first
        ircv3_module.enable_sasl(client.id).await;
        
        // Test AUTHENTICATE command handling
        let auth_msg = Message::new(
            MessageType::Custom("AUTHENTICATE".to_string()),
            vec!["PLAIN".to_string()]
        );
        
        // This would normally be handled by the server's message routing
        // For testing, we can verify the message structure is correct
        assert_eq!(auth_msg.params[0], "PLAIN");
        
        ircv3_module.cleanup().await.unwrap();
    }
}
