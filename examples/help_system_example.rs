//! Help System Example
//! 
//! Demonstrates the enhanced help system with dynamic module discovery.

use rustircd_core::{Client, User, ModuleManager};
use rustircd_modules::{HelpModule, MonitorModule, KnockModule, BanManagementModule, HelpProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Rust IRC Daemon - Enhanced Help System Example");
    println!("================================================");
    
    // Create a module manager
    let mut module_manager = ModuleManager::new();
    
    // Create modules
    let help_module = HelpModule::new();
    let monitor_module = MonitorModule::new();
    let knock_module = KnockModule::new();
    let ban_module = BanManagementModule::new();
    
    // Register modules
    module_manager.register_module("help", Box::new(help_module)).await?;
    module_manager.register_module("monitor", Box::new(monitor_module)).await?;
    module_manager.register_module("knock", Box::new(knock_module)).await?;
    module_manager.register_module("ban_management", Box::new(ban_module)).await?;
    
    // Initialize all modules
    module_manager.initialize_all().await?;
    
    // Get the help module to demonstrate dynamic discovery
    if let Some(help_module) = module_manager.get_module("help").await {
        if let Some(help_provider) = help_module.as_any().downcast_ref::<HelpModule>() {
            println!("\n📚 Available Commands (with module information):");
            println!("=============================================");
            
            // Get all available commands for a regular user
            let user_commands = help_provider.get_available_commands(false);
            for topic in user_commands {
                let module_info = topic.module_name.as_ref()
                    .map(|m| format!(" [{}]", m))
                    .unwrap_or_default();
                println!("  {}{} - {}", topic.command, module_info, topic.description);
            }
            
            println!("\n🔧 Available Commands for Operators:");
            println!("===================================");
            
            // Get all available commands for an operator
            let oper_commands = help_provider.get_available_commands(true);
            for topic in oper_commands {
                let module_info = topic.module_name.as_ref()
                    .map(|m| format!(" [{}]", m))
                    .unwrap_or_default();
                let oper_indicator = if topic.oper_only { " (oper)" } else { "" };
                println!("  {}{}{} - {}", topic.command, module_info, oper_indicator, topic.description);
            }
            
            println!("\n📋 Module Information:");
            println!("=====================");
            
            // Show module information
            let modules = module_manager.get_modules().await;
            for (module_name, module) in modules {
                println!("  {} - {}", module_name, module.description());
                
                // Get commands from this module
                if let Some(help_provider) = module.as_any().downcast_ref::<dyn HelpProvider>() {
                    let topics = help_provider.get_help_topics();
                    if !topics.is_empty() {
                        let command_list: Vec<String> = topics
                            .iter()
                            .map(|topic| {
                                let oper_indicator = if topic.oper_only { " (oper)" } else { "" };
                                format!("{}{}", topic.command, oper_indicator)
                            })
                            .collect();
                        println!("    Commands: {}", command_list.join(", "));
                    } else {
                        println!("    No help topics available");
                    }
                }
            }
        }
    }
    
    println!("\n✨ Enhanced Help System Features:");
    println!("=================================");
    println!("  • Dynamic command discovery from loaded modules");
    println!("  • Module information display with HELP MODULES");
    println!("  • Automatic help topic generation from modules");
    println!("  • Operator vs user command filtering");
    println!("  • Real-time module loading/unloading support");
    println!("  • Comprehensive command documentation");
    
    println!("\n🎯 Usage Examples:");
    println!("==================");
    println!("  HELP                    - Show all available commands");
    println!("  HELP MODULES            - Show loaded modules and their commands");
    println!("  HELP MONITOR            - Show detailed help for MONITOR command");
    println!("  HELP GLINE              - Show detailed help for GLINE command");
    println!("  HELP KNOCK              - Show detailed help for KNOCK command");
    
    Ok(())
}
