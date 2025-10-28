//! Tests for IRC command handling

use rustircd_core::*;

#[tokio::test]
async fn test_nick_command() {
    let msg = Message::parse("NICK alice").unwrap();
    assert_eq!(msg.command, MessageType::Nick);
    assert_eq!(msg.params[0], "alice");
    
    // Test with colon prefix
    let msg = Message::parse("NICK :alice").unwrap();
    assert_eq!(msg.command, MessageType::Nick);
    assert_eq!(msg.params[0], "alice");
}

#[tokio::test]
async fn test_user_command() {
    let msg = Message::parse("USER alice 0 * :Alice Wonderland").unwrap();
    assert_eq!(msg.command, MessageType::User);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[3], "Alice Wonderland");
}

#[tokio::test]
async fn test_privmsg_command() {
    // Test channel message
    let msg = Message::parse("PRIVMSG #channel :Hello world").unwrap();
    assert_eq!(msg.command, MessageType::PrivMsg);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "Hello world");
    
    // Test user message
    let msg = Message::parse("PRIVMSG alice :Hi there").unwrap();
    assert_eq!(msg.command, MessageType::PrivMsg);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[1], "Hi there");
}

#[tokio::test]
async fn test_notice_command() {
    let msg = Message::parse("NOTICE #channel :This is a notice").unwrap();
    assert_eq!(msg.command, MessageType::Notice);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "This is a notice");
}

#[tokio::test]
async fn test_join_command() {
    // Simple join
    let msg = Message::parse("JOIN #channel").unwrap();
    assert_eq!(msg.command, MessageType::Join);
    assert_eq!(msg.params[0], "#channel");
    
    // Join with key
    let msg = Message::parse("JOIN #channel secret").unwrap();
    assert_eq!(msg.command, MessageType::Join);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "secret");
    
    // Join multiple channels
    let msg = Message::parse("JOIN #channel1,#channel2").unwrap();
    assert_eq!(msg.command, MessageType::Join);
    assert_eq!(msg.params[0], "#channel1,#channel2");
}

#[tokio::test]
async fn test_part_command() {
    // Part without reason
    let msg = Message::parse("PART #channel").unwrap();
    assert_eq!(msg.command, MessageType::Part);
    assert_eq!(msg.params[0], "#channel");
    
    // Part with reason
    let msg = Message::parse("PART #channel :Goodbye").unwrap();
    assert_eq!(msg.command, MessageType::Part);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "Goodbye");
}

#[tokio::test]
async fn test_quit_command() {
    // Quit without reason
    let msg = Message::parse("QUIT").unwrap();
    assert_eq!(msg.command, MessageType::Quit);
    
    // Quit with reason
    let msg = Message::parse("QUIT :Leaving").unwrap();
    assert_eq!(msg.command, MessageType::Quit);
    assert_eq!(msg.params[0], "Leaving");
}

#[tokio::test]
async fn test_ping_pong_commands() {
    let msg = Message::parse("PING :server").unwrap();
    assert_eq!(msg.command, MessageType::Ping);
    assert_eq!(msg.params[0], "server");
    
    let msg = Message::parse("PONG :server").unwrap();
    assert_eq!(msg.command, MessageType::Pong);
    assert_eq!(msg.params[0], "server");
}

#[tokio::test]
async fn test_mode_command() {
    // User mode
    let msg = Message::parse("MODE alice +i").unwrap();
    assert_eq!(msg.command, MessageType::Mode);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[1], "+i");
    
    // Channel mode
    let msg = Message::parse("MODE #channel +m").unwrap();
    assert_eq!(msg.command, MessageType::Mode);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "+m");
    
    // Mode with parameter
    let msg = Message::parse("MODE #channel +o alice").unwrap();
    assert_eq!(msg.command, MessageType::Mode);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "+o");
    assert_eq!(msg.params[2], "alice");
}

#[tokio::test]
async fn test_topic_command() {
    // Get topic
    let msg = Message::parse("TOPIC #channel").unwrap();
    assert_eq!(msg.command, MessageType::Topic);
    assert_eq!(msg.params[0], "#channel");
    
    // Set topic
    let msg = Message::parse("TOPIC #channel :New topic").unwrap();
    assert_eq!(msg.command, MessageType::Topic);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "New topic");
}

#[tokio::test]
async fn test_kick_command() {
    // Kick without reason
    let msg = Message::parse("KICK #channel alice").unwrap();
    assert_eq!(msg.command, MessageType::Kick);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "alice");
    
    // Kick with reason
    let msg = Message::parse("KICK #channel alice :Spamming").unwrap();
    assert_eq!(msg.command, MessageType::Kick);
    assert_eq!(msg.params[0], "#channel");
    assert_eq!(msg.params[1], "alice");
    assert_eq!(msg.params[2], "Spamming");
}

#[tokio::test]
async fn test_invite_command() {
    let msg = Message::parse("INVITE alice #channel").unwrap();
    assert_eq!(msg.command, MessageType::Invite);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[1], "#channel");
}

#[tokio::test]
async fn test_names_command() {
    // All channels
    let msg = Message::parse("NAMES").unwrap();
    assert_eq!(msg.command, MessageType::Names);
    
    // Specific channel
    let msg = Message::parse("NAMES #channel").unwrap();
    assert_eq!(msg.command, MessageType::Names);
    assert_eq!(msg.params[0], "#channel");
}

#[tokio::test]
async fn test_list_command() {
    // All channels
    let msg = Message::parse("LIST").unwrap();
    assert_eq!(msg.command, MessageType::List);
    
    // Specific channels
    let msg = Message::parse("LIST #channel1,#channel2").unwrap();
    assert_eq!(msg.command, MessageType::List);
    assert_eq!(msg.params[0], "#channel1,#channel2");
}

#[tokio::test]
async fn test_who_command() {
    let msg = Message::parse("WHO #channel").unwrap();
    assert_eq!(msg.command, MessageType::Who);
    assert_eq!(msg.params[0], "#channel");
    
    let msg = Message::parse("WHO alice").unwrap();
    assert_eq!(msg.command, MessageType::Who);
    assert_eq!(msg.params[0], "alice");
}

#[tokio::test]
async fn test_whois_command() {
    let msg = Message::parse("WHOIS alice").unwrap();
    assert_eq!(msg.command, MessageType::Whois);
    assert_eq!(msg.params[0], "alice");
    
    // Multiple users
    let msg = Message::parse("WHOIS alice,bob").unwrap();
    assert_eq!(msg.command, MessageType::Whois);
    assert_eq!(msg.params[0], "alice,bob");
}

#[tokio::test]
async fn test_whowas_command() {
    let msg = Message::parse("WHOWAS alice").unwrap();
    assert_eq!(msg.command, MessageType::Whowas);
    assert_eq!(msg.params[0], "alice");
    
    // With count
    let msg = Message::parse("WHOWAS alice 5").unwrap();
    assert_eq!(msg.command, MessageType::Whowas);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[1], "5");
}

#[tokio::test]
async fn test_away_command() {
    // Set away
    let msg = Message::parse("AWAY :Gone for lunch").unwrap();
    assert_eq!(msg.command, MessageType::Away);
    assert_eq!(msg.params[0], "Gone for lunch");
    
    // Unset away
    let msg = Message::parse("AWAY").unwrap();
    assert_eq!(msg.command, MessageType::Away);
}

#[tokio::test]
async fn test_ison_command() {
    let msg = Message::parse("ISON alice bob charlie").unwrap();
    assert_eq!(msg.command, MessageType::Ison);
    assert_eq!(msg.params.len(), 3);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[1], "bob");
    assert_eq!(msg.params[2], "charlie");
}

#[tokio::test]
async fn test_userhost_command() {
    let msg = Message::parse("USERHOST alice bob").unwrap();
    assert_eq!(msg.command, MessageType::Userhost);
    assert_eq!(msg.params.len(), 2);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[1], "bob");
}

#[tokio::test]
async fn test_motd_command() {
    let msg = Message::parse("MOTD").unwrap();
    assert_eq!(msg.command, MessageType::Motd);
    
    let msg = Message::parse("MOTD server.example.com").unwrap();
    assert_eq!(msg.command, MessageType::Motd);
    assert_eq!(msg.params[0], "server.example.com");
}

#[tokio::test]
async fn test_lusers_command() {
    let msg = Message::parse("LUSERS").unwrap();
    assert_eq!(msg.command, MessageType::Lusers);
}

#[tokio::test]
async fn test_version_command() {
    let msg = Message::parse("VERSION").unwrap();
    assert_eq!(msg.command, MessageType::Version);
    
    let msg = Message::parse("VERSION server.example.com").unwrap();
    assert_eq!(msg.command, MessageType::Version);
    assert_eq!(msg.params[0], "server.example.com");
}

#[tokio::test]
async fn test_stats_command() {
    let msg = Message::parse("STATS m").unwrap();
    assert_eq!(msg.command, MessageType::Stats);
    assert_eq!(msg.params[0], "m");
    
    let msg = Message::parse("STATS l server.example.com").unwrap();
    assert_eq!(msg.command, MessageType::Stats);
    assert_eq!(msg.params[0], "l");
    assert_eq!(msg.params[1], "server.example.com");
}

#[tokio::test]
async fn test_links_command() {
    let msg = Message::parse("LINKS").unwrap();
    assert_eq!(msg.command, MessageType::Links);
    
    let msg = Message::parse("LINKS *.edu").unwrap();
    assert_eq!(msg.command, MessageType::Links);
    assert_eq!(msg.params[0], "*.edu");
}

#[tokio::test]
async fn test_time_command() {
    let msg = Message::parse("TIME").unwrap();
    assert_eq!(msg.command, MessageType::Time);
    
    let msg = Message::parse("TIME server.example.com").unwrap();
    assert_eq!(msg.command, MessageType::Time);
    assert_eq!(msg.params[0], "server.example.com");
}

#[tokio::test]
async fn test_admin_command() {
    let msg = Message::parse("ADMIN").unwrap();
    assert_eq!(msg.command, MessageType::Admin);
    
    let msg = Message::parse("ADMIN server.example.com").unwrap();
    assert_eq!(msg.command, MessageType::Admin);
    assert_eq!(msg.params[0], "server.example.com");
}

#[tokio::test]
async fn test_info_command() {
    let msg = Message::parse("INFO").unwrap();
    assert_eq!(msg.command, MessageType::Info);
}

#[tokio::test]
async fn test_oper_command() {
    let msg = Message::parse("OPER alice secretpass").unwrap();
    assert_eq!(msg.command, MessageType::Oper);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[1], "secretpass");
}

#[tokio::test]
async fn test_kill_command() {
    let msg = Message::parse("KILL alice :Spamming").unwrap();
    assert_eq!(msg.command, MessageType::Kill);
    assert_eq!(msg.params[0], "alice");
    assert_eq!(msg.params[1], "Spamming");
}

#[tokio::test]
async fn test_wallops_command() {
    let msg = Message::parse("WALLOPS :Server maintenance in 5 minutes").unwrap();
    assert_eq!(msg.command, MessageType::Custom("WALLOPS".to_string()));
    assert_eq!(msg.params[0], "Server maintenance in 5 minutes");
}

#[tokio::test]
async fn test_cap_command() {
    // CAP LS
    let msg = Message::parse("CAP LS").unwrap();
    assert_eq!(msg.command, MessageType::Custom("CAP".to_string()));
    assert_eq!(msg.params[0], "LS");
    
    // CAP REQ
    let msg = Message::parse("CAP REQ :multi-prefix extended-join").unwrap();
    assert_eq!(msg.command, MessageType::Custom("CAP".to_string()));
    assert_eq!(msg.params[0], "REQ");
    assert_eq!(msg.params[1], "multi-prefix extended-join");
}

#[tokio::test]
async fn test_authenticate_command() {
    let msg = Message::parse("AUTHENTICATE PLAIN").unwrap();
    assert_eq!(msg.command, MessageType::Custom("AUTHENTICATE".to_string()));
    assert_eq!(msg.params[0], "PLAIN");
}






