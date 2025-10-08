//! Configurable IRC numeric replies system
//! 
//! This module allows server administrators to customize IRC numeric replies
//! by loading them from a TOML configuration file, while maintaining
//! sensible defaults for all RFC 1459 defined replies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;

/// Configuration for a single numeric reply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyConfig {
    /// The numeric code (e.g., 001, 401, etc.)
    pub code: u16,
    /// The reply text template with placeholders
    /// Placeholders: {nick}, {user}, {host}, {server}, {channel}, {target}, {reason}, {count}, {info}
    pub text: String,
    /// Optional description of what this reply is for
    pub description: Option<String>,
}

/// Configuration for all numeric replies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepliesConfig {
    /// All numeric replies by their code
    pub replies: HashMap<u16, ReplyConfig>,
}

/// Server information used in reply templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Server description
    pub description: String,
    /// Server creation date
    pub created: String,
    /// Administrator email
    pub admin_email: String,
    /// Administrator location line 1
    pub admin_location1: String,
    /// Administrator location line 2
    pub admin_location2: String,
}

impl Default for ServerInfo {
    fn default() -> Self {
        Self {
            name: "rustircd".to_string(),
            version: "1.0.0".to_string(),
            description: "Rust IRC Daemon".to_string(),
            created: "2025-01-01".to_string(),
            admin_email: "admin@example.com".to_string(),
            admin_location1: "Rust IRC Network".to_string(),
            admin_location2: "https://github.com/rustircd/rustircd".to_string(),
        }
    }
}

impl Default for RepliesConfig {
    fn default() -> Self {
        Self {
            replies: Self::default_replies(),
        }
    }
}

impl RepliesConfig {
    /// Load replies configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read replies config file: {}", e))?;
        
        let mut config: RepliesConfig = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse replies config file: {}", e))?;
        
        // Merge with defaults for any missing replies
        let defaults = Self::default_replies();
        for (code, default_reply) in defaults {
            config.replies.entry(code).or_insert(default_reply);
        }
        
        info!("Loaded replies configuration with {} custom replies", config.replies.len());
        Ok(config)
    }
    
    /// Save current configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize replies config: {}", e))?;
        
        fs::write(path, content)
            .map_err(|e| format!("Failed to write replies config file: {}", e))?;
        
        Ok(())
    }
    
    /// Get a reply configuration by code
    pub fn get_reply(&self, code: u16) -> Option<&ReplyConfig> {
        self.replies.get(&code)
    }
    
    /// Format a reply with the given parameters and server information
    pub fn format_reply(&self, code: u16, params: &HashMap<String, String>, server_info: &ServerInfo) -> Option<String> {
        let reply_config = self.get_reply(code)?;
        Some(self.format_template(&reply_config.text, params, server_info))
    }
    
    /// Format a template string with parameters and server information
    fn format_template(&self, template: &str, params: &HashMap<String, String>, server_info: &ServerInfo) -> String {
        let mut result = template.to_string();
        
        // Replace server-specific placeholders
        result = result.replace("{server_name}", &server_info.name);
        result = result.replace("{server_version}", &server_info.version);
        result = result.replace("{server_description}", &server_info.description);
        result = result.replace("{server_created}", &server_info.created);
        result = result.replace("{admin_email}", &server_info.admin_email);
        result = result.replace("{admin_location1}", &server_info.admin_location1);
        result = result.replace("{admin_location2}", &server_info.admin_location2);
        
        // Replace parameter placeholders
        for (key, value) in params {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        result
    }
    
    /// Create default replies configuration based on RFC 1459
    fn default_replies() -> HashMap<u16, ReplyConfig> {
        let mut replies = HashMap::new();
        
        // Connection registration replies
        replies.insert(001, ReplyConfig {
            code: 001,
            text: "Welcome to the Internet Relay Network {nick}!{user}@{host}".to_string(),
            description: Some("RPL_WELCOME - Welcome message".to_string()),
        });
        
        replies.insert(002, ReplyConfig {
            code: 002,
            text: "Your host is {server_name}, running version {server_version}".to_string(),
            description: Some("RPL_YOURHOST - Server information".to_string()),
        });
        
        replies.insert(003, ReplyConfig {
            code: 003,
            text: "This server was created {server_created}".to_string(),
            description: Some("RPL_CREATED - Server creation date".to_string()),
        });
        
        replies.insert(004, ReplyConfig {
            code: 004,
            text: "{server_name} {server_version} {usermodes} {channelmodes}".to_string(),
            description: Some("RPL_MYINFO - Server capabilities".to_string()),
        });
        
        // Server queries
        replies.insert(256, ReplyConfig {
            code: 256,
            text: "{server_name} :Administrative info".to_string(),
            description: Some("RPL_ADMINME - Admin info header".to_string()),
        });
        
        replies.insert(257, ReplyConfig {
            code: 257,
            text: ":{admin_location1}".to_string(),
            description: Some("RPL_ADMINLOC1 - Admin location line 1".to_string()),
        });
        
        replies.insert(258, ReplyConfig {
            code: 258,
            text: ":{admin_location2}".to_string(),
            description: Some("RPL_ADMINLOC2 - Admin location line 2".to_string()),
        });
        
        replies.insert(259, ReplyConfig {
            code: 259,
            text: ":{admin_email}".to_string(),
            description: Some("RPL_ADMINEMAIL - Admin email".to_string()),
        });
        
        replies.insert(351, ReplyConfig {
            code: 351,
            text: "{server_name} :{server_version} {server_name} :{server_description}".to_string(),
            description: Some("RPL_VERSION - Server version info".to_string()),
        });
        
        // User queries
        replies.insert(311, ReplyConfig {
            code: 311,
            text: "{nick} {user} {host} * :{realname}".to_string(),
            description: Some("RPL_WHOISUSER - User information".to_string()),
        });
        
        replies.insert(312, ReplyConfig {
            code: 312,
            text: "{nick} {server_name} :{server_description}".to_string(),
            description: Some("RPL_WHOISSERVER - Server information".to_string()),
        });
        
        replies.insert(313, ReplyConfig {
            code: 313,
            text: "{nick} :is an IRC operator".to_string(),
            description: Some("RPL_WHOISOPERATOR - Operator status".to_string()),
        });
        
        replies.insert(315, ReplyConfig {
            code: 315,
            text: "{target} :End of WHO list".to_string(),
            description: Some("RPL_ENDOFWHO - End of WHO list".to_string()),
        });
        
        replies.insert(318, ReplyConfig {
            code: 318,
            text: "{nick} :End of WHOIS list".to_string(),
            description: Some("RPL_ENDOFWHOIS - End of WHOIS list".to_string()),
        });
        
        replies.insert(319, ReplyConfig {
            code: 319,
            text: "{nick} :{channels}".to_string(),
            description: Some("RPL_WHOISCHANNELS - User channels".to_string()),
        });
        
        // Channel operations
        replies.insert(322, ReplyConfig {
            code: 322,
            text: "{channel} {visible} :{topic}".to_string(),
            description: Some("RPL_LIST - Channel list entry".to_string()),
        });
        
        replies.insert(323, ReplyConfig {
            code: 323,
            text: ":End of LIST".to_string(),
            description: Some("RPL_LISTEND - End of LIST".to_string()),
        });
        
        replies.insert(331, ReplyConfig {
            code: 331,
            text: "{channel} :No topic is set".to_string(),
            description: Some("RPL_NOTOPIC - No topic set".to_string()),
        });
        
        replies.insert(332, ReplyConfig {
            code: 332,
            text: "{channel} :{topic}".to_string(),
            description: Some("RPL_TOPIC - Channel topic".to_string()),
        });
        
        replies.insert(341, ReplyConfig {
            code: 341,
            text: "{nick} {channel}".to_string(),
            description: Some("RPL_INVITING - Invitation sent".to_string()),
        });
        
        replies.insert(353, ReplyConfig {
            code: 353,
            text: "= {channel} :{names}".to_string(),
            description: Some("RPL_NAMREPLY - Channel names list".to_string()),
        });
        
        replies.insert(366, ReplyConfig {
            code: 366,
            text: "{channel} :End of NAMES list".to_string(),
            description: Some("RPL_ENDOFNAMES - End of NAMES list".to_string()),
        });
        
        // Error replies
        replies.insert(401, ReplyConfig {
            code: 401,
            text: "{nick} :No such nick/channel".to_string(),
            description: Some("ERR_NOSUCHNICK - No such nick/channel".to_string()),
        });
        
        replies.insert(402, ReplyConfig {
            code: 402,
            text: "{server} :No such server".to_string(),
            description: Some("ERR_NOSUCHSERVER - No such server".to_string()),
        });
        
        replies.insert(403, ReplyConfig {
            code: 403,
            text: "{channel} :No such channel".to_string(),
            description: Some("ERR_NOSUCHCHANNEL - No such channel".to_string()),
        });
        
        replies.insert(404, ReplyConfig {
            code: 404,
            text: "{channel} :Cannot send to channel".to_string(),
            description: Some("ERR_CANNOTSENDTOCHAN - Cannot send to channel".to_string()),
        });
        
        replies.insert(405, ReplyConfig {
            code: 405,
            text: "{channel} :You have joined too many channels".to_string(),
            description: Some("ERR_TOOMANYCHANNELS - Too many channels".to_string()),
        });
        
        replies.insert(421, ReplyConfig {
            code: 421,
            text: "{command} :Unknown command".to_string(),
            description: Some("ERR_UNKNOWNCOMMAND - Unknown command".to_string()),
        });
        
        replies.insert(422, ReplyConfig {
            code: 422,
            text: ":MOTD File is missing".to_string(),
            description: Some("ERR_NOMOTD - No MOTD file".to_string()),
        });
        
        replies.insert(431, ReplyConfig {
            code: 431,
            text: ":No nickname given".to_string(),
            description: Some("ERR_NONICKNAMEGIVEN - No nickname given".to_string()),
        });
        
        replies.insert(432, ReplyConfig {
            code: 432,
            text: "{nick} :Erroneous nickname".to_string(),
            description: Some("ERR_ERRONEUSNICKNAME - Erroneous nickname".to_string()),
        });
        
        replies.insert(433, ReplyConfig {
            code: 433,
            text: "{nick} :Nickname is already in use".to_string(),
            description: Some("ERR_NICKNAMEINUSE - Nickname in use".to_string()),
        });
        
        replies.insert(436, ReplyConfig {
            code: 436,
            text: "{nick} :Nickname collision KILL from {server}".to_string(),
            description: Some("ERR_NICKCOLLISION - Nickname collision".to_string()),
        });
        
        replies.insert(441, ReplyConfig {
            code: 441,
            text: "{nick} {channel} :They aren't on that channel".to_string(),
            description: Some("ERR_USERNOTINCHANNEL - User not in channel".to_string()),
        });
        
        replies.insert(442, ReplyConfig {
            code: 442,
            text: "{channel} :You're not on that channel".to_string(),
            description: Some("ERR_NOTONCHANNEL - Not on channel".to_string()),
        });
        
        replies.insert(443, ReplyConfig {
            code: 443,
            text: "{user} {channel} :is already on channel".to_string(),
            description: Some("ERR_USERONCHANNEL - User already on channel".to_string()),
        });
        
        replies.insert(451, ReplyConfig {
            code: 451,
            text: ":You have not registered".to_string(),
            description: Some("ERR_NOTREGISTERED - Not registered".to_string()),
        });
        
        replies.insert(461, ReplyConfig {
            code: 461,
            text: "{command} :Not enough parameters".to_string(),
            description: Some("ERR_NEEDMOREPARAMS - Need more parameters".to_string()),
        });
        
        replies.insert(462, ReplyConfig {
            code: 462,
            text: ":You may not reregister".to_string(),
            description: Some("ERR_ALREADYREGISTERED - Already registered".to_string()),
        });
        
        replies.insert(463, ReplyConfig {
            code: 463,
            text: ":Your host isn't among the privileged".to_string(),
            description: Some("ERR_NOPERMFORHOST - No permission for host".to_string()),
        });
        
        replies.insert(464, ReplyConfig {
            code: 464,
            text: ":Password incorrect".to_string(),
            description: Some("ERR_PASSWDMISMATCH - Password mismatch".to_string()),
        });
        
        replies.insert(465, ReplyConfig {
            code: 465,
            text: ":You are banned from this server".to_string(),
            description: Some("ERR_YOUREBANNEDCREEP - Banned from server".to_string()),
        });
        
        replies.insert(471, ReplyConfig {
            code: 471,
            text: "{channel} :Cannot join channel (+l)".to_string(),
            description: Some("ERR_CHANNELISFULL - Channel is full".to_string()),
        });
        
        replies.insert(472, ReplyConfig {
            code: 472,
            text: "{char} :is unknown mode char to me for {channel}".to_string(),
            description: Some("ERR_UNKNOWNMODE - Unknown mode".to_string()),
        });
        
        replies.insert(473, ReplyConfig {
            code: 473,
            text: "{channel} :Cannot join channel (+i)".to_string(),
            description: Some("ERR_INVITEONLYCHAN - Invite only channel".to_string()),
        });
        
        replies.insert(474, ReplyConfig {
            code: 474,
            text: "{channel} :Cannot join channel (+b)".to_string(),
            description: Some("ERR_BANNEDFROMCHAN - Banned from channel".to_string()),
        });
        
        replies.insert(475, ReplyConfig {
            code: 475,
            text: "{channel} :Cannot join channel (+k)".to_string(),
            description: Some("ERR_BADCHANNELKEY - Bad channel key".to_string()),
        });
        
        replies.insert(476, ReplyConfig {
            code: 476,
            text: "{channel} :Bad channel mask".to_string(),
            description: Some("ERR_BADCHANMASK - Bad channel mask".to_string()),
        });
        
        replies.insert(477, ReplyConfig {
            code: 477,
            text: "{channel} :Channel doesn't support modes".to_string(),
            description: Some("ERR_NOCHANMODES - No channel modes".to_string()),
        });
        
        replies.insert(478, ReplyConfig {
            code: 478,
            text: "{channel} {char} :Channel list is full".to_string(),
            description: Some("ERR_BANLISTFULL - Ban list full".to_string()),
        });
        
        replies.insert(481, ReplyConfig {
            code: 481,
            text: ":Permission Denied- You're not an IRC operator".to_string(),
            description: Some("ERR_NOPRIVILEGES - No privileges".to_string()),
        });
        
        replies.insert(482, ReplyConfig {
            code: 482,
            text: "{channel} :You're not channel operator".to_string(),
            description: Some("ERR_CHANOPRIVSNEEDED - Channel operator privileges needed".to_string()),
        });
        
        replies.insert(483, ReplyConfig {
            code: 483,
            text: ":You can't kill a server!".to_string(),
            description: Some("ERR_CANTKILLSERVER - Can't kill server".to_string()),
        });
        
        replies.insert(484, ReplyConfig {
            code: 484,
            text: ":Your connection is restricted!".to_string(),
            description: Some("ERR_RESTRICTED - Connection restricted".to_string()),
        });
        
        // Operator commands
        replies.insert(381, ReplyConfig {
            code: 381,
            text: ":You are now an IRC operator".to_string(),
            description: Some("RPL_YOUREOPER - You are now an operator".to_string()),
        });
        
        replies.insert(200, ReplyConfig {
            code: 200,
            text: "Link {version} {destination} {next_server} {hopcount} {protocol_version} {flags} {link_data} {link_creation}".to_string(),
            description: Some("RPL_TRACELINK - Trace link information".to_string()),
        });
        
        replies.insert(201, ReplyConfig {
            code: 201,
            text: "Try. {class} {server}".to_string(),
            description: Some("RPL_TRACECONNECTING - Trace connecting".to_string()),
        });
        
        replies.insert(202, ReplyConfig {
            code: 202,
            text: "H.S. {class} {server}".to_string(),
            description: Some("RPL_TRACEHANDSHAKE - Trace handshake".to_string()),
        });
        
        replies.insert(203, ReplyConfig {
            code: 203,
            text: "???? {class} {client_ip} {client_port} {server_ip} {server_port}".to_string(),
            description: Some("RPL_TRACEUNKNOWN - Trace unknown".to_string()),
        });
        
        replies.insert(204, ReplyConfig {
            code: 204,
            text: "Oper {class} {nick}".to_string(),
            description: Some("RPL_TRACEOPERATOR - Trace operator".to_string()),
        });
        
        replies.insert(205, ReplyConfig {
            code: 205,
            text: "User {class} {nick}".to_string(),
            description: Some("RPL_TRACEUSER - Trace user".to_string()),
        });
        
        replies.insert(206, ReplyConfig {
            code: 206,
            text: "Serv {class} {int}S {int}C {server} {nick}!{user}@{host}".to_string(),
            description: Some("RPL_TRACESERVER - Trace server".to_string()),
        });
        
        replies.insert(207, ReplyConfig {
            code: 207,
            text: "Service {class} {name} {type} {active_type}".to_string(),
            description: Some("RPL_TRACESERVICE - Trace service".to_string()),
        });
        
        replies.insert(208, ReplyConfig {
            code: 208,
            text: "{newtype} 0 {client_name}".to_string(),
            description: Some("RPL_TRACENEWTYPE - Trace new type".to_string()),
        });
        
        replies.insert(209, ReplyConfig {
            code: 209,
            text: "Class {class} {count}".to_string(),
            description: Some("RPL_TRACECLASS - Trace class".to_string()),
        });
        
        replies.insert(261, ReplyConfig {
            code: 261,
            text: "File {logfile} {debug_level}".to_string(),
            description: Some("RPL_TRACELOG - Trace log".to_string()),
        });
        
        replies.insert(262, ReplyConfig {
            code: 262,
            text: "{server_name} {version} {debug_level} {current_path} {max_connections}".to_string(),
            description: Some("RPL_TRACEEND - Trace end".to_string()),
        });
        
        // Stats replies
        replies.insert(211, ReplyConfig {
            code: 211,
            text: "Link {hostname} {port} {port} {class} {sendq} {sent_msgs} {sent_bytes} {recv_msgs} {recv_bytes} {time_open}".to_string(),
            description: Some("RPL_STATSLINKINFO - Stats link info".to_string()),
        });
        
        replies.insert(212, ReplyConfig {
            code: 212,
            text: "Commands {char} {count} {byte_count} {remote_count}".to_string(),
            description: Some("RPL_STATSCOMMANDS - Stats commands".to_string()),
        });
        
        replies.insert(213, ReplyConfig {
            code: 213,
            text: "C {host} * {name} {port} {class}".to_string(),
            description: Some("RPL_STATSCLINE - Stats C line".to_string()),
        });
        
        replies.insert(214, ReplyConfig {
            code: 214,
            text: "N {host} * {name} {port} {class}".to_string(),
            description: Some("RPL_STATSNLINE - Stats N line".to_string()),
        });
        
        replies.insert(215, ReplyConfig {
            code: 215,
            text: "I {host} * {name} {port} {class}".to_string(),
            description: Some("RPL_STATSILINE - Stats I line".to_string()),
        });
        
        replies.insert(216, ReplyConfig {
            code: 216,
            text: "K {host} * {username} {port} {class}".to_string(),
            description: Some("RPL_STATSKLINE - Stats K line".to_string()),
        });
        
        replies.insert(218, ReplyConfig {
            code: 218,
            text: "Y {class} {ping_freq} {connect_freq} {max_sendq}".to_string(),
            description: Some("RPL_STATSYLINE - Stats Y line".to_string()),
        });
        
        replies.insert(219, ReplyConfig {
            code: 219,
            text: "{letter} :End of STATS report".to_string(),
            description: Some("RPL_ENDOFSTATS - End of stats".to_string()),
        });
        
        replies.insert(242, ReplyConfig {
            code: 242,
            text: ":Server Up {uptime} seconds".to_string(),
            description: Some("RPL_STATSUPTIME - Server uptime".to_string()),
        });
        
        replies.insert(243, ReplyConfig {
            code: 243,
            text: "O {hostmask} * {name} {port} {class}".to_string(),
            description: Some("RPL_STATSOLINE - Stats O line".to_string()),
        });
        
        replies.insert(244, ReplyConfig {
            code: 244,
            text: "H {host} * {name} {port} {class}".to_string(),
            description: Some("RPL_STATSHLINE - Stats H line".to_string()),
        });
        
        replies.insert(375, ReplyConfig {
            code: 375,
            text: ":- {server} Message of the Day -".to_string(),
            description: Some("RPL_MOTDSTART - MOTD start".to_string()),
        });
        
        replies.insert(372, ReplyConfig {
            code: 372,
            text: ":- {line}".to_string(),
            description: Some("RPL_MOTD - MOTD line".to_string()),
        });
        
        replies.insert(376, ReplyConfig {
            code: 376,
            text: ":End of /MOTD command.".to_string(),
            description: Some("RPL_ENDOFMOTD - MOTD end".to_string()),
        });
        
        replies.insert(422, ReplyConfig {
            code: 422,
            text: ":MOTD file is missing".to_string(),
            description: Some("ERR_NOMOTD - No MOTD file".to_string()),
        });
        
        replies.insert(215, ReplyConfig {
            code: 215,
            text: "I {host} * {host} {port} {class}".to_string(),
            description: Some("RPL_STATSILINE - Stats I line".to_string()),
        });
        
        replies.insert(216, ReplyConfig {
            code: 216,
            text: "K {host} * {username} {port} {class}".to_string(),
            description: Some("RPL_STATSKLINE - Stats K line".to_string()),
        });
        
        replies.insert(218, ReplyConfig {
            code: 218,
            text: "Y {class} {ping_freq} {connect_freq} {max_sendq}".to_string(),
            description: Some("RPL_STATSYLINE - Stats Y line".to_string()),
        });
        
        replies.insert(219, ReplyConfig {
            code: 219,
            text: "{char} :End of STATS report".to_string(),
            description: Some("RPL_ENDOFSTATS - End of stats".to_string()),
        });
        
        replies.insert(241, ReplyConfig {
            code: 241,
            text: "L {hostmask} * {servername} {port} {class}".to_string(),
            description: Some("RPL_STATSLLINE - Stats L line".to_string()),
        });
        
        replies.insert(242, ReplyConfig {
            code: 242,
            text: ":Server Up {days} days, {hours:02}:{minutes:02}:{seconds:02}".to_string(),
            description: Some("RPL_STATSUPTIME - Server uptime".to_string()),
        });
        
        replies.insert(243, ReplyConfig {
            code: 243,
            text: "O {hostmask} * {name}".to_string(),
            description: Some("RPL_STATSOLINE - Stats O line".to_string()),
        });
        
        replies.insert(244, ReplyConfig {
            code: 244,
            text: "H {hostmask} * {servername}".to_string(),
            description: Some("RPL_STATSHLINE - Stats H line".to_string()),
        });
        
        replies.insert(251, ReplyConfig {
            code: 251,
            text: ":There are {users} users and {invisible} invisible on {servers} servers".to_string(),
            description: Some("RPL_LUSERCLIENT - LUSER client".to_string()),
        });
        
        replies.insert(252, ReplyConfig {
            code: 252,
            text: "{ops} :operator(s) online".to_string(),
            description: Some("RPL_LUSEROP - LUSER op".to_string()),
        });
        
        replies.insert(253, ReplyConfig {
            code: 253,
            text: "{unknown} :unknown connection(s)".to_string(),
            description: Some("RPL_LUSERUNKNOWN - LUSER unknown".to_string()),
        });
        
        replies.insert(254, ReplyConfig {
            code: 254,
            text: "{channels} :channels formed".to_string(),
            description: Some("RPL_LUSERCHANNELS - LUSER channels".to_string()),
        });
        
        replies.insert(255, ReplyConfig {
            code: 255,
            text: ":I have {clients} clients and {servers} servers".to_string(),
            description: Some("RPL_LUSERME - LUSER me".to_string()),
        });
        
        replies.insert(265, ReplyConfig {
            code: 265,
            text: ":Current local users: {current} Max: {max}".to_string(),
            description: Some("RPL_LOCALUSERS - Local users".to_string()),
        });
        
        replies.insert(266, ReplyConfig {
            code: 266,
            text: ":Current global users: {current} Max: {max}".to_string(),
            description: Some("RPL_GLOBALUSERS - Global users".to_string()),
        });
        
        // Time and info
        replies.insert(391, ReplyConfig {
            code: 391,
            text: "{server} :{time}".to_string(),
            description: Some("RPL_TIME - Server time".to_string()),
        });
        
        replies.insert(371, ReplyConfig {
            code: 371,
            text: ":{info}".to_string(),
            description: Some("RPL_INFO - Server info".to_string()),
        });
        
        replies.insert(374, ReplyConfig {
            code: 374,
            text: ":End of INFO list".to_string(),
            description: Some("RPL_ENDOFINFO - End of info".to_string()),
        });
        
        // Links
        replies.insert(364, ReplyConfig {
            code: 364,
            text: "{mask} {server} :{hopcount} {server_info}".to_string(),
            description: Some("RPL_LINKS - Server links".to_string()),
        });
        
        replies.insert(365, ReplyConfig {
            code: 365,
            text: "{mask} :End of LINKS list".to_string(),
            description: Some("RPL_ENDOFLINKS - End of links".to_string()),
        });
        
        // Away
        replies.insert(301, ReplyConfig {
            code: 301,
            text: "{nick} :{away_message}".to_string(),
            description: Some("RPL_AWAY - User is away".to_string()),
        });
        
        replies.insert(305, ReplyConfig {
            code: 305,
            text: ":You are no longer marked as being away".to_string(),
            description: Some("RPL_UNAWAY - No longer away".to_string()),
        });
        
        replies.insert(306, ReplyConfig {
            code: 306,
            text: ":You have been marked as being away".to_string(),
            description: Some("RPL_NOWAWAY - Now away".to_string()),
        });
        
        // Userhost and Ison
        replies.insert(302, ReplyConfig {
            code: 302,
            text: ":{reply}".to_string(),
            description: Some("RPL_USERHOST - User host".to_string()),
        });
        
        replies.insert(303, ReplyConfig {
            code: 303,
            text: ":{nick_list}".to_string(),
            description: Some("RPL_ISON - Is on".to_string()),
        });
        
        // MOTD
        replies.insert(375, ReplyConfig {
            code: 375,
            text: ":- {server_name} Message of the day - ".to_string(),
            description: Some("RPL_MOTDSTART - MOTD start".to_string()),
        });
        
        replies.insert(372, ReplyConfig {
            code: 372,
            text: ":- {line}".to_string(),
            description: Some("RPL_MOTD - MOTD line".to_string()),
        });
        
        replies.insert(376, ReplyConfig {
            code: 376,
            text: ":End of MOTD command".to_string(),
            description: Some("RPL_ENDOFMOTD - End of MOTD".to_string()),
        });
        
        // LUSERS command replies
        replies.insert(251, ReplyConfig {
            code: 251,
            text: ":There are {param0} users and {param1} services on {param2} servers".to_string(),
            description: Some("RPL_LUSERCLIENT - LUSER client info".to_string()),
        });
        
        replies.insert(252, ReplyConfig {
            code: 252,
            text: "{param0} :operator(s) online".to_string(),
            description: Some("RPL_LUSEROP - LUSER operator info".to_string()),
        });
        
        replies.insert(253, ReplyConfig {
            code: 253,
            text: "{param0} :unknown connection(s)".to_string(),
            description: Some("RPL_LUSERUNKNOWN - LUSER unknown connections".to_string()),
        });
        
        replies.insert(254, ReplyConfig {
            code: 254,
            text: "{param0} :channels formed".to_string(),
            description: Some("RPL_LUSERCHANNELS - LUSER channels".to_string()),
        });
        
        replies.insert(255, ReplyConfig {
            code: 255,
            text: ":I have {param0} clients and {param1} servers".to_string(),
            description: Some("RPL_LUSERME - LUSER server info".to_string()),
        });
        
        replies.insert(265, ReplyConfig {
            code: 265,
            text: ":Current local users: {param0}, max: {param1}".to_string(),
            description: Some("RPL_LOCALUSERS - Local users info".to_string()),
        });
        
        replies.insert(266, ReplyConfig {
            code: 266,
            text: ":Current global users: {param0}, max: {param1}".to_string(),
            description: Some("RPL_GLOBALUSERS - Global users info".to_string()),
        });
        
        // User mode replies
        replies.insert(221, ReplyConfig {
            code: 221,
            text: "{param0} {param1}".to_string(),
            description: Some("RPL_UMODEIS - User mode is".to_string()),
        });
        
        replies.insert(502, ReplyConfig {
            code: 502,
            text: ":Cannot change mode for other users".to_string(),
            description: Some("ERR_USERSDONTMATCH - Users don't match".to_string()),
        });
        
        replies.insert(503, ReplyConfig {
            code: 503,
            text: ":Operator mode can only be granted through OPER command".to_string(),
            description: Some("ERR_CANTSETOPERATORMODE - Can't set operator mode".to_string()),
        });
        
        // Rehash
        replies.insert(382, ReplyConfig {
            code: 382,
            text: "{file} :Rehashing".to_string(),
            description: Some("RPL_REHASHING - Rehashing".to_string()),
        });
        
        // Connect command
        replies.insert(200, ReplyConfig {
            code: 200,
            text: "Link {version} {destination} {next_server} {hopcount} {protocol_version} {flags} {link_data} {link_creation}".to_string(),
            description: Some("RPL_CONNECTSUCCESS - Connect success".to_string()),
        });
        
        replies.insert(201, ReplyConfig {
            code: 201,
            text: "Try. {class} {server}".to_string(),
            description: Some("RPL_CONNECTFAILED - Connect failed".to_string()),
        });
        
        // Custom numeric replies
        replies.insert(320, ReplyConfig {
            code: 320,
            text: "{nick} :{info}".to_string(),
            description: Some("RPL_WHOISSPECIAL - Special WHOIS info".to_string()),
        });
        
        replies.insert(317, ReplyConfig {
            code: 317,
            text: "{nick} {seconds} {signon}".to_string(),
            description: Some("RPL_WHOISIDLE - WHOIS idle time".to_string()),
        });
        
        replies.insert(324, ReplyConfig {
            code: 324,
            text: "{channel} {mode} {mode_params}".to_string(),
            description: Some("RPL_CHANNELMODEIS - Channel mode is".to_string()),
        });
        
        replies.insert(341, ReplyConfig {
            code: 341,
            text: "{nick} {channel}".to_string(),
            description: Some("RPL_INVITING - Inviting".to_string()),
        });
        
        replies.insert(346, ReplyConfig {
            code: 346,
            text: "{channel} {invite_mask}".to_string(),
            description: Some("RPL_INVITELIST - Invite list".to_string()),
        });
        
        replies.insert(347, ReplyConfig {
            code: 347,
            text: "{channel} :End of channel invite list".to_string(),
            description: Some("RPL_ENDOFINVITELIST - End of invite list".to_string()),
        });
        
        replies.insert(348, ReplyConfig {
            code: 348,
            text: "{channel} {exception_mask}".to_string(),
            description: Some("RPL_EXCEPTLIST - Exception list".to_string()),
        });
        
        replies.insert(349, ReplyConfig {
            code: 349,
            text: "{channel} :End of channel exception list".to_string(),
            description: Some("RPL_ENDOFEXCEPTLIST - End of exception list".to_string()),
        });
        
        replies.insert(352, ReplyConfig {
            code: 352,
            text: "{channel} {user} {host} {server} {nick} {flags} :{hopcount} {realname}".to_string(),
            description: Some("RPL_WHOREPLY - WHO reply".to_string()),
        });
        
        replies.insert(367, ReplyConfig {
            code: 367,
            text: "{channel} {ban_mask}".to_string(),
            description: Some("RPL_BANLIST - Ban list".to_string()),
        });
        
        replies.insert(368, ReplyConfig {
            code: 368,
            text: "{channel} :End of channel ban list".to_string(),
            description: Some("RPL_ENDOFBANLIST - End of ban list".to_string()),
        });
        
        replies.insert(369, ReplyConfig {
            code: 369,
            text: "{nick} :End of WHOWAS".to_string(),
            description: Some("RPL_ENDOFWHOWAS - End of WHOWAS".to_string()),
        });
        
        replies.insert(221, ReplyConfig {
            code: 221,
            text: "{user_modes}".to_string(),
            description: Some("RPL_UMODEIS - User mode is".to_string()),
        });
        
        replies.insert(392, ReplyConfig {
            code: 392,
            text: ":UserID Terminal Host".to_string(),
            description: Some("RPL_USERSSTART - Users start".to_string()),
        });
        
        replies.insert(393, ReplyConfig {
            code: 393,
            text: ":%-8s %-9s %-8s".to_string(),
            description: Some("RPL_USERS - Users".to_string()),
        });
        
        replies.insert(394, ReplyConfig {
            code: 394,
            text: ":End of users".to_string(),
            description: Some("RPL_ENDOFUSERS - End of users".to_string()),
        });
        
        replies.insert(395, ReplyConfig {
            code: 395,
            text: ":Nobody logged in".to_string(),
            description: Some("RPL_NOUSERS - No users".to_string()),
        });
        
        // Bot mode IRCv3
        replies.insert(320, ReplyConfig {
            code: 320,
            text: "{nick} :{bot_info}".to_string(),
            description: Some("RPL_WHOISBOT - WHOIS bot info".to_string()),
        });
        
        replies
    }
}
