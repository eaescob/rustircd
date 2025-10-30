//! User management and tracking

use crate::config::OperatorFlag;
use crate::Prefix;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use uuid::Uuid;

/// User state for netsplit recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserState {
    /// User is actively connected
    Active,
    /// User is in a netsplit grace period
    NetSplit,
    /// User has been removed
    Removed,
}

/// Bot information for IRCv3 bot-mode
#[derive(Debug, Clone)]
pub struct BotInfo {
    /// Bot name
    pub name: String,
    /// Bot description
    pub description: Option<String>,
    /// Bot version
    pub version: Option<String>,
    /// Bot capabilities
    pub capabilities: Vec<String>,
    /// Registration time
    pub registered_at: DateTime<Utc>,
}

/// User information and state
#[derive(Debug, Clone)]
pub struct User {
    /// Unique user ID
    pub id: Uuid,
    /// Nickname
    pub nick: String,
    /// Username
    pub username: String,
    /// Real name
    pub realname: String,
    /// Hostname/IP
    pub host: String,
    /// Server name
    pub server: String,
    /// Registration time
    pub registered_at: DateTime<Utc>,
    /// Last activity time
    pub last_activity: DateTime<Utc>,
    /// User modes
    pub modes: HashSet<char>,
    /// Channels user is in
    pub channels: HashSet<String>,
    /// Whether user is registered
    pub registered: bool,
    /// Whether user is an operator
    pub is_operator: bool,
    /// Operator flags (if user is an operator)
    pub operator_flags: HashSet<OperatorFlag>,
    /// Away message (if any)
    pub away_message: Option<String>,
    /// Whether user is a bot (IRCv3 bot-mode)
    pub is_bot: bool,
    /// Bot information (if user is a bot)
    pub bot_info: Option<BotInfo>,
    /// User state (for netsplit recovery)
    pub state: UserState,
    /// Time when user entered netsplit state (for delayed cleanup)
    pub split_at: Option<DateTime<Utc>>,
}

impl User {
    /// Create a new user
    pub fn new(
        nick: String,
        username: String,
        realname: String,
        host: String,
        server: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            nick,
            username,
            realname,
            host,
            server,
            registered_at: now,
            last_activity: now,
            modes: HashSet::new(),
            channels: HashSet::new(),
            registered: false,
            is_operator: false,
            operator_flags: HashSet::new(),
            away_message: None,
            is_bot: false,
            bot_info: None,
            state: UserState::Active,
            split_at: None,
        }
    }

    /// Get user prefix for messages
    pub fn prefix(&self) -> Prefix {
        Prefix::User {
            nick: self.nick.clone(),
            user: self.username.clone(),
            host: self.host.clone(),
        }
    }

    /// Get user nickname
    pub fn nickname(&self) -> &str {
        &self.nick
    }

    /// Get username
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Get hostname
    pub fn hostname(&self) -> &str {
        &self.host
    }

    /// Check if user is an operator
    pub fn is_operator(&self) -> bool {
        self.is_operator
    }

    /// Check if user is an admin (has umode +a)
    pub fn is_admin(&self) -> bool {
        self.has_mode('a')
    }

    /// Check if user can set admin umode (must be operator with Administrator flag)
    pub fn can_set_admin_mode(&self) -> bool {
        self.is_operator && self.operator_flags.contains(&OperatorFlag::Administrator)
    }

    /// Check if user has a specific mode
    pub fn has_mode(&self, mode: char) -> bool {
        self.modes.contains(&mode)
    }

    /// Add a mode to the user
    pub fn add_mode(&mut self, mode: char) {
        // Prevent clients from setting operator mode directly
        if mode == 'o' {
            tracing::warn!("Attempted to set operator mode 'o' directly - this should only be done through OPER command");
            return;
        }
        // Prevent clients from setting admin mode without proper privileges
        if mode == 'a' && !self.can_set_admin_mode() {
            tracing::warn!("Attempted to set admin mode 'a' without Administrator privileges");
            return;
        }
        self.modes.insert(mode);
    }

    /// Remove a mode from the user
    pub fn remove_mode(&mut self, mode: char) {
        self.modes.remove(&mode);
    }

    /// Add a mode to the user (internal use only - bypasses security checks)
    pub fn add_mode_internal(&mut self, mode: char) {
        self.modes.insert(mode);
    }

    /// Remove a mode from the user (internal use only - bypasses security checks)
    pub fn remove_mode_internal(&mut self, mode: char) {
        self.modes.remove(&mode);
    }

    /// Get user modes as a string
    pub fn modes_string(&self) -> String {
        let mut modes: Vec<char> = self.modes.iter().cloned().collect();
        modes.sort();
        modes.into_iter().collect()
    }

    /// Join a channel
    pub fn join_channel(&mut self, channel: String) {
        self.channels.insert(channel);
    }

    /// Leave a channel
    pub fn part_channel(&mut self, channel: &str) {
        self.channels.remove(channel);
    }

    /// Check if user is in a channel
    pub fn is_in_channel(&self, channel: &str) -> bool {
        self.channels.contains(channel)
    }

    /// Update last activity time
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Set away message
    pub fn set_away(&mut self, message: Option<String>) {
        self.away_message = message.clone();
        // Note: Away status is tracked via away_message, not as a user mode
        // User mode 'a' is reserved for admin status (following Solanum convention)
    }

    /// Check if user is away
    pub fn is_away(&self) -> bool {
        self.away_message.is_some()
    }

    /// Get user info string for WHOIS
    pub fn whois_info(&self) -> String {
        format!(
            "{} {} {} {} {} :{}",
            self.nick, self.username, self.host, "*", self.server, self.realname
        )
    }

    /// Get user info for WHO command
    pub fn who_info(&self, channel: &str) -> String {
        let modes = if self.is_operator { "@" } else { "" };
        format!(
            "{} {} {} {} {} {} :0 {} {}",
            channel,
            self.username,
            self.host,
            self.server,
            self.nick,
            if self.is_away() { "G" } else { "H" },
            modes,
            self.realname
        )
    }

    /// Set bot mode for user
    pub fn set_bot_mode(&mut self, bot_info: BotInfo) {
        self.is_bot = true;
        self.bot_info = Some(bot_info);
    }

    /// Remove bot mode from user
    pub fn remove_bot_mode(&mut self) {
        self.is_bot = false;
        self.bot_info = None;
    }

    /// Check if user is a bot
    pub fn is_bot(&self) -> bool {
        self.is_bot
    }

    /// Get bot information
    pub fn get_bot_info(&self) -> Option<&BotInfo> {
        self.bot_info.as_ref()
    }

    /// Get bot tag for messages
    pub fn get_bot_tag(&self) -> Option<String> {
        if self.is_bot {
            Some("bot".to_string())
        } else {
            None
        }
    }

    /// Set operator flags (internal use only - requires proper authentication)
    pub fn set_operator_flags(&mut self, flags: HashSet<OperatorFlag>) {
        self.is_operator = !flags.is_empty();
        self.operator_flags = flags;

        // Set or remove operator mode based on flags
        if self.is_operator {
            self.add_mode_internal('o');
        } else {
            self.remove_mode_internal('o');
        }
    }

    /// Grant operator privileges (internal use only - requires proper authentication)
    pub fn grant_operator_privileges(&mut self, flags: HashSet<OperatorFlag>) {
        self.set_operator_flags(flags);
        tracing::info!(
            "Granted operator privileges to user {} with flags: {:?}",
            self.nick,
            self.operator_flags
        );
    }

    /// Revoke operator privileges (internal use only)
    pub fn revoke_operator_privileges(&mut self) {
        self.is_operator = false;
        self.operator_flags.clear();
        self.remove_mode_internal('o');
        // Remove admin umode as well since it requires operator status
        self.remove_mode_internal('a');
        tracing::info!("Revoked operator privileges from user {}", self.nick);
    }

    /// Check if user has a specific operator flag
    pub fn has_operator_flag(&self, flag: OperatorFlag) -> bool {
        self.operator_flags.contains(&flag)
    }

    /// Check if user is a global operator
    pub fn is_global_oper(&self) -> bool {
        self.has_operator_flag(OperatorFlag::GlobalOper)
    }

    /// Check if user is a local operator
    pub fn is_local_oper(&self) -> bool {
        self.has_operator_flag(OperatorFlag::LocalOper)
    }

    /// Check if user can do remote connect
    pub fn can_remote_connect(&self) -> bool {
        self.has_operator_flag(OperatorFlag::RemoteConnect)
    }

    /// Check if user can do local connect
    pub fn can_local_connect(&self) -> bool {
        self.has_operator_flag(OperatorFlag::LocalConnect)
    }

    /// Check if user is administrator
    pub fn is_administrator(&self) -> bool {
        self.has_operator_flag(OperatorFlag::Administrator)
    }

    /// Check if user has spy privileges
    pub fn is_spy(&self) -> bool {
        self.has_operator_flag(OperatorFlag::Spy)
    }

    /// Check if user can use SQUIT command
    pub fn can_squit(&self) -> bool {
        self.has_operator_flag(OperatorFlag::Squit)
    }
}
