//! Bot Mode IRCv3 Capability Flow Example
//! 
//! This example demonstrates the complete flow of bot mode registration
//! and how it affects WHOIS commands and message tagging.

use rustircd_core::{User, BotInfo, Message, MessageType, NumericReply};
use chrono::Utc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Bot Mode IRCv3 Capability Flow Demo ===\n");

    // Demonstrate the complete bot mode flow
    demonstrate_bot_registration().await?;
    demonstrate_whois_bot_info().await?;
    demonstrate_message_tagging().await?;
    demonstrate_bot_commands().await?;

    Ok(())
}

async fn demonstrate_bot_registration() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Bot Registration Flow");
    println!("========================");
    
    // Create a regular user
    let mut user = User::new(
        "MyBot".to_string(),
        "mybot".to_string(),
        "My Awesome Bot".to_string(),
        "bot.example.com".to_string(),
        "localhost".to_string(),
    );
    
    println!("Initial user: is_bot = {}", user.is_bot());
    
    // Register as bot
    let bot_info = BotInfo {
        name: "MyAwesomeBot".to_string(),
        description: Some("A helpful IRC bot for the channel".to_string()),
        version: Some("1.0.0".to_string()),
        capabilities: vec![
            "commands".to_string(),
            "weather".to_string(),
            "quotes".to_string(),
        ],
        registered_at: Utc::now(),
    };
    
    user.set_bot_mode(bot_info);
    
    println!("After bot registration:");
    println!("  is_bot = {}", user.is_bot());
    println!("  bot_name = {:?}", user.get_bot_info().map(|b| &b.name));
    println!("  bot_description = {:?}", user.get_bot_info().and_then(|b| &b.description));
    println!("  bot_version = {:?}", user.get_bot_info().and_then(|b| &b.version));
    println!("  bot_capabilities = {:?}", user.get_bot_info().map(|b| &b.capabilities));
    println!("  bot_tag = {:?}", user.get_bot_tag());
    
    Ok(())
}

async fn demonstrate_whois_bot_info() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n2. WHOIS Bot Information");
    println!("========================");
    
    // Create a bot user
    let mut user = User::new(
        "WeatherBot".to_string(),
        "weather".to_string(),
        "Weather Information Bot".to_string(),
        "weather.example.com".to_string(),
        "localhost".to_string(),
    );
    
    let bot_info = BotInfo {
        name: "WeatherBot".to_string(),
        description: Some("Provides weather information for cities worldwide".to_string()),
        version: Some("2.1.0".to_string()),
        capabilities: vec![
            "weather".to_string(),
            "forecast".to_string(),
            "alerts".to_string(),
            "location".to_string(),
        ],
        registered_at: Utc::now(),
    };
    
    user.set_bot_mode(bot_info);
    
    // Simulate WHOIS responses
    println!("WHOIS responses for bot user:");
    
    // Basic user info
    let whois_user = NumericReply::whois_user(
        &user.nick,
        &user.username,
        &user.host,
        &user.realname,
    );
    println!("  {:?}", whois_user);
    
    // Server info
    let whois_server = NumericReply::whois_server(
        &user.nick,
        "localhost",
        "Rust IRC Daemon",
    );
    println!("  {:?}", whois_server);
    
    // Bot information
    if let Some(bot_info) = user.get_bot_info() {
        let whois_bot = NumericReply::whois_bot(
            &user.nick,
            &bot_info.name,
            &bot_info.description.as_deref().unwrap_or("No description"),
        );
        println!("  {:?}", whois_bot);
        
        if let (Some(version), Some(capabilities)) = (&bot_info.version, Some(bot_info.capabilities.join(", "))) {
            let bot_info = NumericReply::bot_info(
                &user.nick,
                version,
                &capabilities,
            );
            println!("  {:?}", bot_info);
        }
    }
    
    // End of WHOIS
    let end_whois = NumericReply::end_of_whois(&user.nick);
    println!("  {:?}", end_whois);
    
    Ok(())
}

async fn demonstrate_message_tagging() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. Message Tagging with Bot Mode");
    println!("=================================");
    
    // Create a bot user
    let mut user = User::new(
        "QuoteBot".to_string(),
        "quotes".to_string(),
        "Quote Bot".to_string(),
        "quotes.example.com".to_string(),
        "localhost".to_string(),
    );
    
    let bot_info = BotInfo {
        name: "QuoteBot".to_string(),
        description: Some("Shares inspirational quotes".to_string()),
        version: Some("1.5.0".to_string()),
        capabilities: vec!["quotes".to_string()],
        registered_at: Utc::now(),
    };
    
    user.set_bot_mode(bot_info);
    
    // Create a message from the bot
    let message = Message::new(
        MessageType::PrivMsg,
        vec!["#general".to_string(), "Here's a quote: 'Be yourself; everyone else is already taken.'".to_string()],
    );
    
    println!("Regular message: {:?}", message);
    
    // Add bot tag to message
    if let Some(bot_tag) = user.get_bot_tag() {
        println!("Bot tag: {}", bot_tag);
        println!("Tagged message would include: @+bot={}", bot_tag);
    }
    
    // Simulate message with tags
    let tagged_message = format!("@+bot={} :{}!{}@{} PRIVMSG #general :Here's a quote: 'Be yourself; everyone else is already taken.'", 
        user.get_bot_tag().unwrap_or_default(),
        user.nick,
        user.username,
        user.host
    );
    
    println!("Tagged message: {}", tagged_message);
    
    Ok(())
}

async fn demonstrate_bot_commands() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n4. Bot Commands and Capabilities");
    println!("=================================");
    
    // Create different types of bots
    let bots = vec![
        ("ModBot", "Moderation bot", "1.0.0", vec!["kick", "ban", "mute", "warn"]),
        ("MusicBot", "Music streaming bot", "2.3.1", vec!["play", "pause", "skip", "queue"]),
        ("GameBot", "Gaming bot", "1.2.0", vec!["trivia", "hangman", "quiz", "scores"]),
    ];
    
    for (name, description, version, capabilities) in bots {
        let mut user = User::new(
            name.to_string(),
            name.to_lowercase(),
            description.to_string(),
            format!("{}.example.com", name.to_lowercase()),
            "localhost".to_string(),
        );
        
        let bot_info = BotInfo {
            name: name.to_string(),
            description: Some(description.to_string()),
            version: Some(version.to_string()),
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
            registered_at: Utc::now(),
        };
        
        user.set_bot_mode(bot_info);
        
        println!("Bot: {}", name);
        println!("  Description: {}", description);
        println!("  Version: {}", version);
        println!("  Capabilities: {}", capabilities.join(", "));
        println!("  Bot tag: {:?}", user.get_bot_tag());
        println!();
    }
    
    Ok(())
}

// Helper function to demonstrate the complete flow
fn demonstrate_complete_flow() {
    println!("\n=== Complete Bot Mode Flow ===");
    println!();
    println!("1. Client connects to server");
    println!("2. Client negotiates capabilities: CAP LS");
    println!("3. Server advertises: CAP * LS :bot-mode");
    println!("4. Client requests: CAP REQ :bot-mode");
    println!("5. Server confirms: CAP * ACK :bot-mode");
    println!("6. Client registers as bot (via custom command)");
    println!("7. Server stores bot information in user record");
    println!("8. Bot sends messages with +bot tags");
    println!("9. WHOIS shows bot information");
    println!("10. Other clients can identify bots via tags");
    println!();
    println!("Key IRCv3 Bot Mode Features:");
    println!("- Bot registration and identification");
    println!("- Message tagging with +bot");
    println!("- WHOIS bot information");
    println!("- Capability negotiation");
    println!("- Bot metadata (name, version, capabilities)");
}
