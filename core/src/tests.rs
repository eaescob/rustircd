//! Tests for the core IRC daemon functionality

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, MessageType, Prefix, NumericReply};
    
    #[test]
    fn test_message_parsing() {
        // Test simple message
        let msg = Message::parse("NICK alice").unwrap();
        assert_eq!(msg.command, MessageType::Nick);
        assert_eq!(msg.params, vec!["alice"]);
        assert!(msg.prefix.is_none());
        
        // Test message with prefix
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
        
        // Test numeric reply
        let msg = Message::parse(":server 001 alice :Welcome to the Internet Relay Network alice!user@host").unwrap();
        assert_eq!(msg.command, MessageType::Custom("001".to_string()));
        assert_eq!(msg.params, vec!["alice", "Welcome to the Internet Relay Network alice!user@host"]);
    }
    
    #[test]
    fn test_message_serialization() {
        let msg = Message::new(MessageType::Nick, vec!["alice".to_string()]);
        assert_eq!(msg.to_string().trim(), "NICK alice");
        
        let msg = Message::with_prefix(
            Prefix::User {
                nick: "alice".to_string(),
                user: "user".to_string(),
                host: "host".to_string(),
            },
            MessageType::PrivMsg,
            vec!["#channel".to_string(), "Hello world".to_string()],
        );
        assert_eq!(msg.to_string().trim(), ":alice!user@host PRIVMSG #channel :Hello world");
    }
    
    #[test]
    fn test_numeric_replies() {
        let welcome = NumericReply::welcome("server", "alice", "user", "host");
        assert_eq!(welcome.command, MessageType::Custom("001".to_string()));
        assert_eq!(welcome.params[0], "alice");
        
        let no_nick = NumericReply::no_nickname_given();
        assert_eq!(no_nick.command, MessageType::Custom("431".to_string()));
        assert_eq!(no_nick.params[0], "*");
    }
    
    #[test]
    fn test_user_creation() {
        let user = crate::User::new(
            "alice".to_string(),
            "user".to_string(),
            "Alice User".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        
        assert_eq!(user.nick, "alice");
        assert_eq!(user.username, "user");
        assert_eq!(user.realname, "Alice User");
        assert_eq!(user.host, "host.example.com");
        assert_eq!(user.server, "server.example.com");
        assert!(!user.registered);
        assert!(!user.is_operator);
    }
    
    
    #[test]
    fn test_utils() {
        use crate::utils::string;
        
        assert!(string::is_valid_channel_name("#channel"));
        assert!(string::is_valid_channel_name("&channel"));
        assert!(!string::is_valid_channel_name("channel"));
        assert!(!string::is_valid_channel_name(""));
        
        assert!(string::is_valid_nickname("alice", 9));
        assert!(string::is_valid_nickname("alice123", 9));
        assert!(!string::is_valid_nickname("", 9));
        assert!(!string::is_valid_nickname("123alice", 9));
        
        assert!(string::is_valid_username("user"));
        assert!(string::is_valid_username("user123"));
        assert!(!string::is_valid_username(""));
        assert!(!string::is_valid_username("user name"));
    }
}
