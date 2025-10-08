//! Help Module
//! 
//! Provides comprehensive help system for users and operators.
//! Based on Ratbox's m_help.c module.

use rustircd_core::{
    async_trait, Client, Message, MessageType, Module, ModuleManager,
    NumericReply, Result, User, ModuleNumericManager, ModuleNumericClient, Server,
    module::{ModuleResult, ModuleStatsResponse, ModuleContext},
    define_module_numerics
};
use tracing::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;

/// Macro to create help topics with module name
macro_rules! help_topic {
    ($command:expr, $syntax:expr, $description:expr, $oper_only:expr, $examples:expr, $module:expr) => {
        Self::create_topic($command, $syntax, $description, $oper_only, $examples, $module)
    };
}

/// Help system module that provides command documentation and assistance
pub struct HelpModule {
    /// Help topics for regular users
    user_help: HashMap<String, HelpTopic>,
    /// Help topics for operators
    oper_help: HashMap<String, HelpTopic>,
    /// General help topics
    general_help: HashMap<String, HelpTopic>,
    /// Reference to module manager for dynamic command discovery
    module_manager: Option<Arc<ModuleManager>>,
    /// Dynamic help topics discovered from modules
    dynamic_help: HashMap<String, HelpTopic>,
    /// Module-specific numeric manager
    numeric_manager: ModuleNumericManager,
}

/// A help topic containing command information
#[derive(Debug, Clone)]
pub struct HelpTopic {
    pub command: String,
    pub syntax: String,
    pub description: String,
    pub oper_only: bool,
    pub examples: Vec<String>,
    pub module_name: Option<String>,
}

/// Trait for modules that can provide help information
pub trait HelpProvider {
    /// Get help topics provided by this module
    fn get_help_topics(&self) -> Vec<HelpTopic>;
    
    /// Get help for a specific command
    fn get_command_help(&self, command: &str) -> Option<HelpTopic>;
}

impl HelpModule {
    /// Create a new help module with default topics
    pub fn new() -> Self {
        let mut module = Self {
            user_help: HashMap::new(),
            oper_help: HashMap::new(),
            general_help: HashMap::new(),
            module_manager: None,
            dynamic_help: HashMap::new(),
            numeric_manager: ModuleNumericManager::new(),
        };
        
        module.initialize_help_topics();
        module
    }
    
    /// Create a new help module with module manager reference
    pub fn with_module_manager(module_manager: Arc<ModuleManager>) -> Self {
        let mut module = Self {
            user_help: HashMap::new(),
            oper_help: HashMap::new(),
            general_help: HashMap::new(),
            module_manager: Some(module_manager),
            dynamic_help: HashMap::new(),
            numeric_manager: ModuleNumericManager::new(),
        };
        
        module.initialize_help_topics();
        module
    }
    

    /// Initialize all help topics
    fn initialize_help_topics(&mut self) {
        // General help topics
        self.add_general_topic(Self::create_topic(
            "HELP", 
            "HELP [command]",
            "Shows help information for commands", 
            false, 
            vec!["HELP".to_string(), "HELP JOIN".to_string(), "HELP PRIVMSG".to_string()], 
            "help"
        ));
        
        // User commands
        self.add_user_topic(help_topic!(
            "JOIN",
            "JOIN <channel>[,<channel>...] [<key>[,<key>...]]",
            "Join one or more channels",
            false,
            vec![
                "JOIN #rust".to_string(),
                "JOIN #rust,#programming".to_string(),
                "JOIN #secret secretkey".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "PART",
            "PART <channel>[,<channel>...] [<reason>]",
            "Leave one or more channels",
            false,
            vec![
                "PART #rust".to_string(),
                "PART #rust,#programming".to_string(),
                "PART #rust Leaving for lunch".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "PRIVMSG",
            "PRIVMSG <target>[,<target>...] :<message>",
            "Send a private message to a user or channel",
            false,
            vec![
                "PRIVMSG #rust :Hello everyone!".to_string(),
                "PRIVMSG alice :Hi there!".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "NOTICE",
            "NOTICE <target>[,<target>...] :<message>",
            "Send a notice to a user or channel",
            false,
            vec![
                "NOTICE #rust :This is a notice".to_string(),
                "NOTICE alice :Hello!".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "NICK",
            "NICK <nickname>",
            "Change your nickname",
            false,
            vec![
                "NICK alice".to_string(),
                "NICK alice_new".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "WHOIS",
            "WHOIS <nickname>[,<nickname>...]",
            "Get information about a user",
            false,
            vec![
                "WHOIS alice".to_string(),
                "WHOIS alice,bob".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "WHO",
            "WHO <mask> [<flags>]",
            "List users matching a mask",
            false,
            vec![
                "WHO #rust".to_string(),
                "WHO alice".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "LIST",
            "LIST [<channel>[,<channel>...]] [<server>]",
            "List channels and their topics",
            false,
            vec![
                "LIST".to_string(),
                "LIST #rust".to_string(),
                "LIST #rust,#programming".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "NAMES",
            "NAMES [<channel>[,<channel>...]]",
            "List nicknames in channels",
            false,
            vec![
                "NAMES".to_string(),
                "NAMES #rust".to_string(),
                "NAMES #rust,#programming".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "TOPIC",
            "TOPIC <channel> [<topic>]",
            "Get or set a channel topic",
            false,
            vec![
                "TOPIC #rust".to_string(),
                "TOPIC #rust Welcome to #rust!".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "MODE",
            "MODE <target> [<modes> [<mode-parameters>]]",
            "Get or set modes for a channel or user",
            false,
            vec![
                "MODE #rust".to_string(),
                "MODE #rust +t".to_string(),
                "MODE alice +i".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "INVITE",
            "INVITE <nickname> <channel>",
            "Invite a user to a channel",
            false,
            vec![
                "INVITE alice #rust".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "KICK",
            "KICK <channel> <user> [<reason>]",
            "Remove a user from a channel",
            false,
            vec![
                "KICK #rust alice".to_string(),
                "KICK #rust alice Being disruptive".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "AWAY",
            "AWAY [<reason>]",
            "Set or remove away status",
            false,
            vec![
                "AWAY".to_string(),
                "AWAY Gone to lunch".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "ISON",
            "ISON <nickname>[,<nickname>...]",
            "Check if users are online",
            false,
            vec![
                "ISON alice".to_string(),
                "ISON alice,bob,charlie".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "USERHOST",
            "USERHOST <nickname>[,<nickname>...]",
            "Get user host information",
            false,
            vec![
                "USERHOST alice".to_string(),
                "USERHOST alice,bob".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "PING",
            "PING <server>",
            "Test connection to a server",
            false,
            vec![
                "PING irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "PONG",
            "PONG <server> [<server2>]",
            "Reply to a PING command",
            false,
            vec![
                "PONG irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_user_topic(help_topic!(
            "QUIT",
            "QUIT [<reason>]",
            "Disconnect from the server",
            false,
            vec![
                "QUIT".to_string(),
                "QUIT Goodbye!".to_string(),
            ],
            "core"
        ));
        
        // Operator commands
        self.add_oper_topic(help_topic!(
            "OPER",
            "OPER <name> <password>",
            "Authenticate as an IRC operator",
            true,
            vec![
                "OPER admin secretpass".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "KILL",
            "KILL <nickname> <reason>",
            "Force disconnect a user",
            true,
            vec![
                "KILL alice Spamming".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "WALLOPS",
            "WALLOPS :<message>",
            "Send a message to all operators",
            true,
            vec![
                "WALLOPS :Server maintenance in 10 minutes".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "CONNECT",
            "CONNECT <server> [<port>] [<server>]",
            "Connect to another server",
            true,
            vec![
                "CONNECT irc.example.com".to_string(),
                "CONNECT irc.example.com 6667".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "SQUIT",
            "SQUIT <server> [<reason>]",
            "Disconnect from a server",
            true,
            vec![
                "SQUIT irc.example.com".to_string(),
                "SQUIT irc.example.com Server maintenance".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "STATS",
            "STATS <query> [<server>]",
            "Get server statistics",
            true,
            vec![
                "STATS c".to_string(),
                "STATS l".to_string(),
                "STATS m".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "LINKS",
            "LINKS [<server>]",
            "List server links",
            true,
            vec![
                "LINKS".to_string(),
                "LINKS irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "VERSION",
            "VERSION [<server>]",
            "Get server version information",
            true,
            vec![
                "VERSION".to_string(),
                "VERSION irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "ADMIN",
            "ADMIN [<server>]",
            "Get server administrator information",
            true,
            vec![
                "ADMIN".to_string(),
                "ADMIN irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "INFO",
            "INFO [<server>]",
            "Get server information",
            true,
            vec![
                "INFO".to_string(),
                "INFO irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "LUSERS",
            "LUSERS [<mask> [<server>]]",
            "Get user count statistics",
            true,
            vec![
                "LUSERS".to_string(),
                "LUSERS irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "MOTD",
            "MOTD [<server>]",
            "Get the message of the day",
            true,
            vec![
                "MOTD".to_string(),
                "MOTD irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "USERS",
            "USERS [<server>]",
            "Get user count information",
            true,
            vec![
                "USERS".to_string(),
                "USERS irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "MAP",
            "MAP [<server>]",
            "Get network topology map",
            true,
            vec![
                "MAP".to_string(),
                "MAP irc.example.com".to_string(),
            ],
            "core"
        ));
        
        self.add_oper_topic(help_topic!(
            "TRACE",
            "TRACE [<server>]",
            "Trace connection path to a server",
            true,
            vec![
                "TRACE".to_string(),
                "TRACE irc.example.com".to_string(),
            ],
            "core"
        ));
    }
    
    /// Add a general help topic
    fn add_general_topic(&mut self, topic: HelpTopic) {
        self.general_help.insert(topic.command.clone(), topic);
    }
    
    /// Add a user help topic
    fn add_user_topic(&mut self, topic: HelpTopic) {
        self.user_help.insert(topic.command.clone(), topic);
    }
    
    /// Add an operator help topic
    fn add_oper_topic(&mut self, topic: HelpTopic) {
        self.oper_help.insert(topic.command.clone(), topic);
    }
    
    /// Create a help topic with default module name
    fn create_topic(command: &str, syntax: &str, description: &str, oper_only: bool, examples: Vec<String>, module_name: &str) -> HelpTopic {
        HelpTopic {
            command: command.to_string(),
            syntax: syntax.to_string(),
            description: description.to_string(),
            oper_only,
            examples,
            module_name: Some(module_name.to_string()),
        }
    }
    
    /// Get help for a specific command
    fn get_help(&self, command: &str, is_oper: bool) -> Option<&HelpTopic> {
        // First try general help
        if let Some(topic) = self.general_help.get(command) {
            return Some(topic);
        }
        
        // Then try user help
        if let Some(topic) = self.user_help.get(command) {
            return Some(topic);
        }
        
        // Try dynamic help from modules
        if let Some(topic) = self.dynamic_help.get(command) {
            if !topic.oper_only || is_oper {
                return Some(topic);
            }
        }
        
        // Finally try operator help if user is an operator
        if is_oper {
            if let Some(topic) = self.oper_help.get(command) {
                return Some(topic);
            }
        }
        
        None
    }
    
    /// Discover help topics from loaded modules
    async fn discover_module_help(&mut self) -> Result<()> {
        if let Some(module_manager) = &self.module_manager {
            // Clear existing dynamic help
            self.dynamic_help.clear();
            
            // Get all loaded modules
            let modules = module_manager.get_modules().await;
            let module_count = modules.len();

            for (_module_name, _module) in modules {
                // NOTE: Module trait could be enhanced with as_any() method to support dynamic downcasting to HelpProvider
                // This would allow modules to dynamically register help topics at runtime
                // Current implementation uses static help definitions which works well for all modules
                // Enhancement tracked for future consideration if dynamic help registration is needed
            }

            info!("Discovered {} help topics from {} modules",
                  self.dynamic_help.len(), module_count);
        }
        
        Ok(())
    }
    
    /// Refresh help topics from modules
    pub async fn refresh_module_help(&mut self) -> Result<()> {
        self.discover_module_help().await
    }
    
    /// Get all available commands for a user
    fn get_available_commands(&self, is_oper: bool) -> Vec<&HelpTopic> {
        let mut commands = Vec::new();
        
        // Add general help topics
        commands.extend(self.general_help.values());
        
        // Add user help topics
        commands.extend(self.user_help.values());
        
        // Add dynamic help topics from modules
        for topic in self.dynamic_help.values() {
            if !topic.oper_only || is_oper {
                commands.push(topic);
            }
        }
        
        // Add operator help topics if user is an operator
        if is_oper {
            commands.extend(self.oper_help.values());
        }
        
        commands.sort_by(|a, b| a.command.cmp(&b.command));
        commands
    }
    
    /// Handle HELP command
    async fn handle_help(&self, client: &Client, user: &User, args: &[String]) -> Result<()> {
        let is_oper = user.is_operator();
        
        if args.is_empty() {
            // Show general help
            self.send_general_help(client, is_oper).await?;
        } else if args[0].to_uppercase() == "MODULES" {
            // Show module information
            self.send_module_info(client, user).await?;
        } else {
            let command = &args[0].to_uppercase();
            
            if let Some(topic) = self.get_help(command, is_oper) {
                self.send_command_help(client, topic).await?;
            } else {
                // Command not found, show available commands
                client.send_numeric(NumericReply::ErrHelpNotFound, &[command, "No help available for this command"])?;
                self.send_available_commands(client, is_oper).await?;
            }
        }
        
        Ok(())
    }
    
    /// Send general help information
    async fn send_general_help(&self, client: &Client, is_oper: bool) -> Result<()> {
        client.send_numeric(NumericReply::RplHelpStart, &["HELP", "Help system for Rust IRC Daemon"])?;
        client.send_numeric(NumericReply::RplHelpTxt, &["HELP", "Type HELP <command> for detailed help on a specific command"])?;
        client.send_numeric(NumericReply::RplHelpTxt, &["HELP", "Type HELP MODULES to see loaded modules and their commands"])?;
        client.send_numeric(NumericReply::RplHelpTxt, &["HELP", "Available commands:"])?;
        
        let commands = self.get_available_commands(is_oper);
        for topic in commands {
            let module_info = topic.module_name.as_ref()
                .map(|m| format!(" [{}]", m))
                .unwrap_or_default();
            client.send_numeric(NumericReply::RplHelpTxt, &["HELP", &format!("  {}{} - {}", topic.command, module_info, topic.description)])?;
        }
        
        client.send_numeric(NumericReply::RplHelpTxt, &["HELP", "End of HELP"])?;
        client.send_numeric(NumericReply::RplEndOfHelp, &["HELP", "End of HELP"])?;
        
        Ok(())
    }
    
    /// Send help for a specific command
    async fn send_command_help(&self, client: &Client, topic: &HelpTopic) -> Result<()> {
        client.send_numeric(NumericReply::RplHelpStart, &[&topic.command, &topic.description])?;
        client.send_numeric(NumericReply::RplHelpTxt, &[&topic.command, &format!("Syntax: {}", topic.syntax)])?;
        
        if topic.oper_only {
            client.send_numeric(NumericReply::RplHelpTxt, &[&topic.command, "This command is only available to IRC operators"])?;
        }
        
        if let Some(module_name) = &topic.module_name {
            client.send_numeric(NumericReply::RplHelpTxt, &[&topic.command, &format!("Provided by module: {}", module_name)])?;
        }
        
        if !topic.examples.is_empty() {
            client.send_numeric(NumericReply::RplHelpTxt, &[&topic.command, "Examples:"])?;
            for example in &topic.examples {
                client.send_numeric(NumericReply::RplHelpTxt, &[&topic.command, &format!("  {}", example)])?;
            }
        }
        
        client.send_numeric(NumericReply::RplEndOfHelp, &[&topic.command, "End of HELP"])?;
        
        Ok(())
    }
    
    /// Send list of available commands
    async fn send_available_commands(&self, client: &Client, is_oper: bool) -> Result<()> {
        client.send_numeric(NumericReply::RplHelpTxt, &["HELP", "Available commands:"])?;
        
        let commands = self.get_available_commands(is_oper);
        for topic in commands {
            let module_info = topic.module_name.as_ref()
                .map(|m| format!(" [{}]", m))
                .unwrap_or_default();
            client.send_numeric(NumericReply::RplHelpTxt, &["HELP", &format!("  {}{} - {}", topic.command, module_info, topic.description)])?;
        }
        
        client.send_numeric(NumericReply::RplHelpTxt, &["HELP", "End of available commands"])?;
        
        Ok(())
    }
    
    /// Send a module-specific numeric reply
    fn send_module_numeric(&self, client: &Client, numeric: &str, params: &[&str]) -> Result<()> {
        client.send_module_numeric(&self.numeric_manager, numeric, params)
    }

    /// Send module information
    async fn send_module_info(&self, client: &Client, user: &User) -> Result<()> {
        self.send_module_numeric(client, "RPL_HELPSTART", &["MODULES", "Loaded modules and their commands"])?;
        
        if let Some(module_manager) = &self.module_manager {
            let modules = module_manager.get_modules().await;
            
            for (module_name, module) in modules {
                self.send_module_numeric(client, "RPL_HELPTXT", &["MODULES", &format!("Module: {} - {}", module_name, module.description())])?;
                
                // Get commands from this module
                let module_commands: Vec<&HelpTopic> = self.dynamic_help.values()
                    .filter(|topic| topic.module_name.as_ref().map_or(false, |m| m == &module_name))
                    .collect();
                
                if !module_commands.is_empty() {
                    let command_list: Vec<String> = module_commands
                        .iter()
                        .map(|topic| {
                            let oper_indicator = if topic.oper_only { " (oper)" } else { "" };
                            format!("{}{}", topic.command, oper_indicator)
                        })
                        .collect();
                    
                    self.send_module_numeric(client, "RPL_HELPTXT", &["MODULES", &format!("  Commands: {}", command_list.join(", "))])?;
                } else {
                    self.send_module_numeric(client, "RPL_HELPTXT", &["MODULES", "  No help topics available"])?;
                }
            }
        } else {
            self.send_module_numeric(client, "RPL_HELPTXT", &["MODULES", "Module manager not available"])?;
        }
        
        self.send_module_numeric(client, "RPL_ENDOFHELP", &["MODULES", "End of module information"])?;
        
        Ok(())
    }
}

#[async_trait]
impl Module for HelpModule {
    fn name(&self) -> &str {
        "help"
    }
    
    fn description(&self) -> &str {
        "Provides comprehensive help system for users and operators"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("Help module initialized with {} user commands, {} oper commands, {} general topics",
              self.user_help.len(), self.oper_help.len(), self.general_help.len());
        Ok(())
    }

    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        // Get user from client
        let user = match &client.user {
            Some(u) => u,
            None => return Ok(ModuleResult::NotHandled), // Not registered yet
        };

        match message.command {
            MessageType::Custom(ref cmd) if cmd == "HELP" => {
                self.handle_help(client, user, &message.params).await?;
                Ok(ModuleResult::Handled)
            }
            _ => {
                Ok(ModuleResult::NotHandled)
            }
        }
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        info!("Help module cleaned up");
        Ok(())
    }
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, _user: &User, _context: &ModuleContext) -> Result<()> {
        Ok(())
    }
    
    fn get_capabilities(&self) -> Vec<String> {
        vec!["message_handler".to_string()]
    }
    
    fn supports_capability(&self, capability: &str) -> bool {
        capability == "message_handler"
    }
    
    fn get_numeric_replies(&self) -> Vec<u16> {
        vec![]
    }
    
    fn handles_numeric_reply(&self, _numeric: u16) -> bool {
        false
    }
    
    async fn handle_numeric_reply(&mut self, _numeric: u16, _params: Vec<String>) -> Result<()> {
        Ok(())
    }
    
    async fn handle_stats_query(&mut self, _query: &str, _client_id: uuid::Uuid, _server: Option<&Server>) -> Result<Vec<ModuleStatsResponse>> {
        Ok(vec![])
    }
    
    fn get_stats_queries(&self) -> Vec<String> {
        vec![]
    }
    
    fn register_numerics(&self, manager: &mut ModuleNumericManager) -> Result<()> {
        // Register help-specific numerics
        define_module_numerics!(help, manager, {
            RPL_HELPSTART = 704,
            RPL_HELPTXT = 705,
            RPL_ENDOFHELP = 706,
            RPL_MODULES = 711
        });
        Ok(())
    }
}

impl Default for HelpModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_help_module_creation() {
        let module = HelpModule::new();
        assert!(!module.user_help.is_empty());
        assert!(!module.oper_help.is_empty());
        assert!(!module.general_help.is_empty());
    }
    
    #[test]
    fn test_help_topic_retrieval() {
        let module = HelpModule::new();
        
        // Test user command
        assert!(module.get_help("JOIN", false).is_some());
        assert!(module.get_help("JOIN", true).is_some());
        
        // Test operator command
        assert!(module.get_help("KILL", false).is_none());
        assert!(module.get_help("KILL", true).is_some());
        
        // Test non-existent command
        assert!(module.get_help("NONEXISTENT", false).is_none());
        assert!(module.get_help("NONEXISTENT", true).is_none());
    }
    
    #[test]
    fn test_available_commands() {
        let module = HelpModule::new();
        
        let user_commands = module.get_available_commands(false);
        let oper_commands = module.get_available_commands(true);
        
        assert!(oper_commands.len() > user_commands.len());
        assert!(user_commands.iter().any(|c| c.command == "JOIN"));
        assert!(oper_commands.iter().any(|c| c.command == "KILL"));
    }
}
