//! IRC message parsing and handling
//! 
//! This module implements the IRC message format as defined in RFC 1459.

use serde::{Deserialize, Serialize};
use std::fmt;

/// IRC message prefix (server or user)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Prefix {
    /// Server name
    Server(String),
    /// User prefix (nick!user@host)
    User {
        nick: String,
        user: String,
        host: String,
    },
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Prefix::Server(name) => write!(f, "{}", name),
            Prefix::User { nick, user, host } => write!(f, "{}!{}@{}", nick, user, host),
        }
    }
}

/// IRC message types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    // Connection registration
    Password,
    Nick,
    User,
    Server,
    Oper,
    Quit,
    ServerQuit,
    
    // Channel operations
    Join,
    Part,
    Mode,
    Topic,
    Names,
    List,
    Invite,
    Kick,
    
    // Server queries
    Version,
    Stats,
    Links,
    Time,
    Connect,
    Trace,
    Admin,
    Info,
    Motd,
    
    // Messaging
    PrivMsg,
    Notice,
    Wallops,
    
    // User queries
    Who,
    Whois,
    Whowas,
    
    // Miscellaneous
    Kill,
    Ping,
    Pong,
    Error,
    Away,
    Ison,
    Userhost,
    Lusers,
    
    // Server-to-server specific
    ServerBurst,
    UserBurst,
    ChannelBurst,
    ServerPing,
    ServerPong,
    
    // IRCv3 extensions
    Cap,
    Authenticate,
    
    // Custom/unknown
    Custom(String),
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MessageType::Password => "PASS",
            MessageType::Nick => "NICK",
            MessageType::User => "USER",
            MessageType::Server => "SERVER",
            MessageType::Oper => "OPER",
            MessageType::Quit => "QUIT",
            MessageType::ServerQuit => "SQUIT",
            MessageType::Join => "JOIN",
            MessageType::Part => "PART",
            MessageType::Mode => "MODE",
            MessageType::Topic => "TOPIC",
            MessageType::Names => "NAMES",
            MessageType::List => "LIST",
            MessageType::Invite => "INVITE",
            MessageType::Kick => "KICK",
            MessageType::Version => "VERSION",
            MessageType::Stats => "STATS",
            MessageType::Links => "LINKS",
            MessageType::Time => "TIME",
            MessageType::Connect => "CONNECT",
            MessageType::Trace => "TRACE",
            MessageType::Admin => "ADMIN",
            MessageType::Info => "INFO",
            MessageType::Motd => "MOTD",
            MessageType::PrivMsg => "PRIVMSG",
            MessageType::Notice => "NOTICE",
            MessageType::Wallops => "WALLOPS",
            MessageType::Who => "WHO",
            MessageType::Whois => "WHOIS",
            MessageType::Whowas => "WHOWAS",
            MessageType::Kill => "KILL",
            MessageType::Ping => "PING",
            MessageType::Pong => "PONG",
            MessageType::Error => "ERROR",
            MessageType::Away => "AWAY",
            MessageType::Ison => "ISON",
            MessageType::Userhost => "USERHOST",
            MessageType::Lusers => "LUSERS",
            MessageType::ServerBurst => "BURST",
            MessageType::UserBurst => "UBURST",
            MessageType::ChannelBurst => "CBURST",
            MessageType::ServerPing => "PING",
            MessageType::ServerPong => "PONG",
            MessageType::Cap => "CAP",
            MessageType::Authenticate => "AUTHENTICATE",
            MessageType::Custom(cmd) => cmd,
        };
        write!(f, "{}", s)
    }
}

impl From<&str> for MessageType {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "PASS" => MessageType::Password,
            "NICK" => MessageType::Nick,
            "USER" => MessageType::User,
            "SERVER" => MessageType::Server,
            "OPER" => MessageType::Oper,
            "QUIT" => MessageType::Quit,
            "SQUIT" => MessageType::ServerQuit,
            "JOIN" => MessageType::Join,
            "PART" => MessageType::Part,
            "MODE" => MessageType::Mode,
            "TOPIC" => MessageType::Topic,
            "NAMES" => MessageType::Names,
            "LIST" => MessageType::List,
            "INVITE" => MessageType::Invite,
            "KICK" => MessageType::Kick,
            "VERSION" => MessageType::Version,
            "STATS" => MessageType::Stats,
            "LINKS" => MessageType::Links,
            "TIME" => MessageType::Time,
            "CONNECT" => MessageType::Connect,
            "TRACE" => MessageType::Trace,
            "ADMIN" => MessageType::Admin,
            "INFO" => MessageType::Info,
            "MOTD" => MessageType::Motd,
            "PRIVMSG" => MessageType::PrivMsg,
            "NOTICE" => MessageType::Notice,
            "WALLOPS" => MessageType::Wallops,
            "WHO" => MessageType::Who,
            "WHOIS" => MessageType::Whois,
            "WHOWAS" => MessageType::Whowas,
            "KILL" => MessageType::Kill,
            "PING" => MessageType::Ping,
            "PONG" => MessageType::Pong,
            "ERROR" => MessageType::Error,
            "AWAY" => MessageType::Away,
            "ISON" => MessageType::Ison,
            "USERHOST" => MessageType::Userhost,
            "LUSERS" => MessageType::Lusers,
            "BURST" => MessageType::ServerBurst,
            "UBURST" => MessageType::UserBurst,
            "CBURST" => MessageType::ChannelBurst,
            "CAP" => MessageType::Cap,
            "AUTHENTICATE" => MessageType::Authenticate,
            _ => MessageType::Custom(s.to_string()),
        }
    }
}

/// IRC message as defined in RFC 1459
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Optional prefix (server or user)
    pub prefix: Option<Prefix>,
    /// Message command/type
    pub command: MessageType,
    /// Message parameters
    pub params: Vec<String>,
}

impl Message {
    /// Create a new message
    pub fn new(command: MessageType, params: Vec<String>) -> Self {
        Self {
            prefix: None,
            command,
            params,
        }
    }
    
    /// Create a new message with prefix
    pub fn with_prefix(prefix: Prefix, command: MessageType, params: Vec<String>) -> Self {
        Self {
            prefix: Some(prefix),
            command,
            params,
        }
    }
    
    /// Parse an IRC message from a string
    pub fn parse(input: &str) -> crate::Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Err(crate::Error::MessageParse("Empty message".to_string()));
        }
        
        let parts = input.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            return Err(crate::Error::MessageParse("No command found".to_string()));
        }
        
        let (prefix, command_str) = if parts[0].starts_with(':') {
            let prefix_str = &parts[0][1..];
            let prefix = if prefix_str.contains('!') {
                // User prefix: nick!user@host
                let parts: Vec<&str> = prefix_str.split('!').collect();
                if parts.len() != 2 {
                    return Err(crate::Error::MessageParse("Invalid user prefix format".to_string()));
                }
                let user_host: Vec<&str> = parts[1].split('@').collect();
                if user_host.len() != 2 {
                    return Err(crate::Error::MessageParse("Invalid user prefix format".to_string()));
                }
                Prefix::User {
                    nick: parts[0].to_string(),
                    user: user_host[0].to_string(),
                    host: user_host[1].to_string(),
                }
            } else {
                // Server prefix
                Prefix::Server(prefix_str.to_string())
            };
            (Some(prefix), parts[1])
        } else {
            (None, parts[0])
        };
        
        let command = MessageType::from(command_str);
        let params = if parts.len() > 1 {
            let start_idx = if prefix.is_some() { 2 } else { 1 };
            let mut params = Vec::new();
            
            for (i, part) in parts.iter().enumerate().skip(start_idx) {
                if part.starts_with(':') {
                    // Last parameter can contain spaces
                    let last_param = &parts[start_idx + i..].join(" ");
                    params.push(last_param[1..].to_string());
                    break;
                } else {
                    params.push(part.to_string());
                }
            }
            params
        } else {
            Vec::new()
        };
        
        Ok(Message {
            prefix,
            command,
            params,
        })
    }
    
    /// Serialize message to string
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        
        if let Some(ref prefix) = self.prefix {
            result.push(':');
            result.push_str(&prefix.to_string());
            result.push(' ');
        }
        
        result.push_str(&self.command.to_string());
        
        for (i, param) in self.params.iter().enumerate() {
            result.push(' ');
            if i == self.params.len() - 1 && param.contains(' ') {
                result.push(':');
            }
            result.push_str(param);
        }
        
        result.push_str("\r\n");
        result
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string().trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_message() {
        let msg = Message::parse("NICK alice").unwrap();
        assert_eq!(msg.command, MessageType::Nick);
        assert_eq!(msg.params, vec!["alice"]);
        assert!(msg.prefix.is_none());
    }
    
    #[test]
    fn test_parse_message_with_prefix() {
        let msg = Message::parse(":alice!user@host PRIVMSG #channel :Hello world").unwrap();
        match msg.prefix {
            Some(Prefix::User { nick, user, host }) => {
                assert_eq!(nick, "alice");
                assert_eq!(user, "user");
                assert_eq!(host, "host");
            }
            _ => panic!("Expected user prefix"),
        }
        assert_eq!(msg.command, MessageType::PrivMsg);
        assert_eq!(msg.params, vec!["#channel", "Hello world"]);
    }
    
    #[test]
    fn test_serialize_message() {
        let msg = Message::new(MessageType::Nick, vec!["alice".to_string()]);
        assert_eq!(msg.to_string().trim(), "NICK alice");
    }
}
