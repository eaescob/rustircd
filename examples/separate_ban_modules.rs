//! Example: Using Separate Ban Modules
//! 
//! This example demonstrates how to use the new separate ban modules
//! instead of the deprecated ban_management module.

use rustircd_core::{Client, User, Message, MessageType, Result};
use rustircd_modules::{
    GlineModule, GlineConfig,
    KlineModule, KlineConfig,
    DlineModule, DlineConfig,
    XlineModule, XlineConfig,
    HelpModule,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Separate Ban Modules Example ===");
    
    // Create separate ban modules with custom configurations
    let gline_config = GlineConfig {
        max_duration: 86400 * 7, // 7 days
        allow_permanent_bans: true,
        require_operator: true,
        auto_cleanup_expired: true,
    };
    let mut gline_module = GlineModule::with_config(gline_config);
    
    let kline_config = KlineConfig {
        max_duration: 86400 * 30, // 30 days
        allow_permanent_bans: true,
        require_operator: true,
        auto_cleanup_expired: true,
    };
    let mut kline_module = KlineModule::with_config(kline_config);
    
    let dline_config = DlineConfig {
        max_duration: 86400 * 30, // 30 days
        allow_permanent_bans: true,
        require_operator: true,
        auto_cleanup_expired: true,
    };
    let mut dline_module = DlineModule::with_config(dline_config);
    
    let xline_config = XlineConfig {
        max_duration: 86400 * 30, // 30 days
        allow_permanent_bans: true,
        require_operator: true,
        auto_cleanup_expired: true,
    };
    let mut xline_module = XlineModule::with_config(xline_config);
    
    // Initialize modules
    gline_module.init().await?;
    kline_module.init().await?;
    dline_module.init().await?;
    xline_module.init().await?;
    
    println!("All ban modules initialized successfully!");
    
    // Create a help module to demonstrate help integration
    let mut help_module = HelpModule::new();
    help_module.init().await?;
    
    println!("\n=== Help Integration Demo ===");
    
    // Show help topics for each ban module
    println!("GLINE module help topics:");
    for topic in gline_module.get_help_topics() {
        println!("  {} - {} (oper: {})", 
                topic.command, 
                topic.description, 
                topic.oper_only);
    }
    
    println!("\nKLINE module help topics:");
    for topic in kline_module.get_help_topics() {
        println!("  {} - {} (oper: {})", 
                topic.command, 
                topic.description, 
                topic.oper_only);
    }
    
    println!("\nDLINE module help topics:");
    for topic in dline_module.get_help_topics() {
        println!("  {} - {} (oper: {})", 
                topic.command, 
                topic.description, 
                topic.oper_only);
    }
    
    println!("\nXLINE module help topics:");
    for topic in xline_module.get_help_topics() {
        println!("  {} - {} (oper: {})", 
                topic.command, 
                topic.description, 
                topic.oper_only);
    }
    
    println!("\n=== Module Information ===");
    println!("GLINE module: {} v{}", 
            gline_module.description(), 
            gline_module.version());
    println!("KLINE module: {} v{}", 
            kline_module.description(), 
            kline_module.version());
    println!("DLINE module: {} v{}", 
            dline_module.description(), 
            dline_module.version());
    println!("XLINE module: {} v{}", 
            xline_module.description(), 
            xline_module.version());
    
    println!("\n=== Benefits of Separate Modules ===");
    println!("1. Each ban type has its own focused module");
    println!("2. Independent configuration for each ban type");
    println!("3. Cleaner help system integration");
    println!("4. Better maintainability and testing");
    println!("5. Easier to enable/disable specific ban types");
    println!("6. Reduced module complexity");
    
    // Cleanup
    gline_module.cleanup().await?;
    kline_module.cleanup().await?;
    dline_module.cleanup().await?;
    xline_module.cleanup().await?;
    help_module.cleanup().await?;
    
    println!("\nExample completed successfully!");
    Ok(())
}
