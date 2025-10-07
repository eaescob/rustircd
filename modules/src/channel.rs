//! Channel operations module

use rustircd_core::{
    Module, module::ModuleResult, Client, Message, User, Error, Result,
    MessageType, Prefix, BroadcastSystem, BroadcastTarget, BroadcastPriority,
    BroadcastMessage, Database, ChannelInfo, module::ModuleContext
};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Channel modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    /// Secret channel
    Secret = 's' as isize,
    /// Private channel
    Private = 'p' as isize,
    /// Invite only
    InviteOnly = 'i' as isize,
    /// Topic settable by channel operator only
    TopicOps = 't' as isize,
    /// No messages to channel from clients on the outside
    NoExternal = 'n' as isize,
    /// Moderated channel
    Moderated = 'm' as isize,
    /// Channel is keyed (password protected)
    Keyed = 'k' as isize,
    /// User limit
    UserLimit = 'l' as isize,
    /// Ban mask
    Ban = 'b' as isize,
    /// Exception mask
    Exception = 'e' as isize,
    /// Invite mask
    Invite = 'I' as isize,
}

/// Channel member with modes
#[derive(Debug, Clone)]
pub struct ChannelMember {
    pub user_id: Uuid,
    pub modes: HashSet<char>,
}

impl ChannelMember {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            modes: HashSet::new(),
        }
    }
    
    pub fn is_operator(&self) -> bool {
        self.modes.contains(&'o')
    }
    
    pub fn is_voice(&self) -> bool {
        self.modes.contains(&'v')
    }
    
    pub fn add_mode(&mut self, mode: char) {
        self.modes.insert(mode);
    }
    
    pub fn remove_mode(&mut self, mode: char) {
        self.modes.remove(&mode);
    }
}

/// Channel information and state
#[derive(Debug, Clone)]
pub struct Channel {
    /// Unique channel ID
    pub id: Uuid,
    /// Channel name
    pub name: String,
    /// Channel topic
    pub topic: Option<String>,
    /// Topic setter
    pub topic_setter: Option<String>,
    /// Topic set time
    pub topic_time: Option<DateTime<Utc>>,
    /// Channel modes
    pub modes: HashSet<char>,
    /// Channel key (password)
    pub key: Option<String>,
    /// User limit
    pub user_limit: Option<usize>,
    /// Channel members
    pub members: HashMap<Uuid, ChannelMember>,
    /// Ban masks
    pub ban_masks: HashSet<String>,
    /// Exception masks
    pub exception_masks: HashSet<String>,
    /// Invite masks
    pub invite_masks: HashSet<String>,
    /// Creation time
    pub created_at: DateTime<Utc>,
}

impl Channel {
    /// Create a new channel
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            topic: None,
            topic_setter: None,
            topic_time: None,
            modes: HashSet::new(),
            key: None,
            user_limit: None,
            members: HashMap::new(),
            ban_masks: HashSet::new(),
            exception_masks: HashSet::new(),
            invite_masks: HashSet::new(),
            created_at: Utc::now(),
        }
    }
    
    /// Check if channel has a specific mode
    pub fn has_mode(&self, mode: char) -> bool {
        self.modes.contains(&mode)
    }
    
    /// Add a mode to the channel
    pub fn add_mode(&mut self, mode: char) {
        self.modes.insert(mode);
    }
    
    /// Remove a mode from the channel
    pub fn remove_mode(&mut self, mode: char) {
        self.modes.remove(&mode);
    }
    
    /// Get channel modes as a string
    pub fn modes_string(&self) -> String {
        let mut modes: Vec<char> = self.modes.iter().cloned().collect();
        modes.sort();
        modes.into_iter().collect()
    }
    
    /// Add a member to the channel
    pub fn add_member(&mut self, user_id: Uuid) -> Result<()> {
        if self.members.contains_key(&user_id) {
            return Err(Error::User("User already in channel".to_string()));
        }
        
        if let Some(limit) = self.user_limit {
            if self.members.len() >= limit {
                return Err(Error::User("Channel is full".to_string()));
            }
        }
        
        self.members.insert(user_id, ChannelMember::new(user_id));
        Ok(())
    }
    
    /// Remove a member from the channel
    pub fn remove_member(&mut self, user_id: &Uuid) {
        self.members.remove(user_id);
    }
    
    /// Check if user is in channel
    pub fn has_member(&self, user_id: &Uuid) -> bool {
        self.members.contains_key(user_id)
    }
    
    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
    
    /// Check if user is an operator
    pub fn is_operator(&self, user_id: &Uuid) -> bool {
        self.members.get(user_id)
            .map(|member| member.is_operator())
            .unwrap_or(false)
    }
    
    /// Set user as operator
    pub fn set_operator(&mut self, user_id: &Uuid, is_op: bool) -> Result<()> {
        if let Some(member) = self.members.get_mut(user_id) {
            if is_op {
                member.add_mode('o');
            } else {
                member.remove_mode('o');
            }
            Ok(())
        } else {
            Err(Error::User("User not in channel".to_string()))
        }
    }
    
    /// Set topic
    pub fn set_topic(&mut self, topic: String, setter: String) {
        self.topic = Some(topic);
        self.topic_setter = Some(setter);
        self.topic_time = Some(Utc::now());
    }
    
    /// Clear topic
    pub fn clear_topic(&mut self) {
        self.topic = None;
        self.topic_setter = None;
        self.topic_time = None;
    }
    
    /// Check if channel is invite only
    pub fn is_invite_only(&self) -> bool {
        self.has_mode('i')
    }
    
    /// Check if channel is moderated
    pub fn is_moderated(&self) -> bool {
        self.has_mode('m')
    }
    
    /// Check if channel is secret
    pub fn is_secret(&self) -> bool {
        self.has_mode('s')
    }
    
    /// Check if channel is private
    pub fn is_private(&self) -> bool {
        self.has_mode('p')
    }
    
    /// Check if channel has no external messages
    pub fn no_external(&self) -> bool {
        self.has_mode('n')
    }
    
    /// Check if topic is ops only
    pub fn topic_ops_only(&self) -> bool {
        self.has_mode('t')
    }
    
    /// Check if channel is keyed
    pub fn is_keyed(&self) -> bool {
        self.has_mode('k')
    }
    
    /// Check if key matches
    pub fn check_key(&self, key: &str) -> bool {
        self.key.as_ref().map_or(false, |k| k == key)
    }
    
    /// Set channel key
    pub fn set_key(&mut self, key: Option<String>) {
        let has_key = key.is_some();
        self.key = key;
        if has_key {
            self.add_mode('k');
        } else {
            self.remove_mode('k');
        }
    }
    
    /// Set user limit
    pub fn set_user_limit(&mut self, limit: Option<usize>) {
        self.user_limit = limit;
        if limit.is_some() {
            self.add_mode('l');
        } else {
            self.remove_mode('l');
        }
    }
}

/// Channel operations module
pub struct ChannelModule {
    name: String,
    version: String,
    description: String,
    /// Channels by name
    channels: Arc<RwLock<HashMap<String, Channel>>>,
    /// Channel-specific numeric replies
    numeric_replies: Vec<u16>,
    /// Broadcast system for channel events
    broadcast_system: Arc<RwLock<BroadcastSystem>>,
    /// Database reference for user/channel tracking
    database: Arc<RwLock<Database>>,
    /// Invite list (nick -> set of channels they're invited to)
    invite_list: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl ChannelModule {
    pub fn new() -> Self {
        Self {
            name: "channel".to_string(),
            version: "1.0.0".to_string(),
            description: "Channel operations and management".to_string(),
            channels: Arc::new(RwLock::new(HashMap::new())),
            numeric_replies: vec![
                // Channel-related numeric replies
                403, // ERR_NOSUCHCHANNEL
                404, // ERR_CANNOTSENDTOCHAN
                405, // ERR_TOOMANYCHANNELS
                441, // ERR_USERNOTINCHANNEL
                442, // ERR_NOTONCHANNEL
                443, // ERR_USERONCHANNEL
                471, // ERR_CHANNELISFULL
                472, // ERR_UNKNOWNMODE
                473, // ERR_INVITEONLYCHAN
                474, // ERR_BANNEDFROMCHAN
                475, // ERR_BADCHANNELKEY
                476, // ERR_BADCHANMASK
                477, // ERR_NOCHANMODES
                478, // ERR_BANLISTFULL
                482, // ERR_CHANOPRIVSNEEDED
                324, // RPL_CHANNELMODEIS
                329, // RPL_CREATIONTIME
                331, // RPL_NOTOPIC
                332, // RPL_TOPIC
                333, // RPL_TOPICWHOTIME
                341, // RPL_INVITING
                346, // RPL_INVITELIST
                347, // RPL_ENDOFINVITELIST
                348, // RPL_EXCEPTLIST
                349, // RPL_ENDOFEXCEPTLIST
                367, // RPL_BANLIST
                368, // RPL_ENDOFBANLIST
                321, // RPL_LISTSTART
                322, // RPL_LIST
                323, // RPL_LISTEND
                353, // RPL_NAMREPLY
                366, // RPL_ENDOFNAMES
            ],
            broadcast_system: Arc::new(RwLock::new(BroadcastSystem::new())),
            database: Arc::new(RwLock::new(Database::new(10000, 30))),
            invite_list: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new channel module with external dependencies
    pub fn with_dependencies(
        broadcast_system: Arc<RwLock<BroadcastSystem>>,
        database: Arc<RwLock<Database>>,
    ) -> Self {
        Self {
            name: "channel".to_string(),
            version: "1.0.0".to_string(),
            description: "Channel operations and management".to_string(),
            channels: Arc::new(RwLock::new(HashMap::new())),
            numeric_replies: vec![
                403, 404, 405, 441, 442, 443, 471, 472, 473, 474, 475, 476, 477, 478, 482,
                324, 329, 331, 332, 333, 341, 346, 347, 348, 349, 367, 368,
                321, 322, 323, 353, 366,
            ],
            broadcast_system,
            database,
            invite_list: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Module for ChannelModule {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing channel module");
        Ok(())
    }
    
    async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Cleaning up channel module");
        Ok(())
    }

    fn register_numerics(&self, _manager: &mut rustircd_core::ModuleNumericManager) -> Result<()> {
        Ok(())
    }
    
    async fn handle_message(&mut self, client: &Client, message: &Message, _context: &ModuleContext) -> Result<ModuleResult> {
        match message.command {
            rustircd_core::MessageType::Join => {
                self.handle_join(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Part => {
                self.handle_part(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Mode => {
                self.handle_mode(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Topic => {
                self.handle_topic(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Names => {
                self.handle_names(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::List => {
                self.handle_list(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Invite => {
                self.handle_invite(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            rustircd_core::MessageType::Kick => {
                self.handle_kick(client, message).await?;
                Ok(ModuleResult::Handled)
            }
            _ => Ok(ModuleResult::NotHandled),
        }
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
        vec![
            "message_handler".to_string(),
        ]
    }
    
    fn supports_capability(&self, capability: &str) -> bool {
        capability == "message_handler"
    }
    
    fn get_numeric_replies(&self) -> Vec<u16> {
        self.numeric_replies.clone()
    }
    
    fn handles_numeric_reply(&self, numeric: u16) -> bool {
        self.numeric_replies.contains(&numeric)
    }
    
    async fn handle_numeric_reply(&mut self, numeric: u16, params: Vec<String>) -> Result<()> {
        // Handle channel-specific numeric replies if needed
        tracing::debug!("Channel module handling numeric reply {}: {:?}", numeric, params);
        Ok(())
    }

    async fn handle_stats_query(&mut self, _query: &str, _client_id: Uuid, _server: Option<&rustircd_core::Server>) -> Result<Vec<rustircd_core::module::ModuleStatsResponse>> {
        // Channel module doesn't provide STATS queries
        Ok(vec![])
    }

    fn get_stats_queries(&self) -> Vec<String> {
        // Channel module doesn't provide STATS queries
        vec![]
    }
}

impl ChannelModule {
    async fn handle_join(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        if message.params.is_empty() {
            return Err(Error::User("No channel specified".to_string()));
        }
        
        let channel_name = &message.params[0];
        let key = message.params.get(1);
        
        // Validate channel name
        if !self.is_valid_channel_name(channel_name) {
            return Err(Error::Channel("Invalid channel name".to_string()));
        }
        
        // Get user from database
        let database = self.database.read().await;
        let user = database.get_user(&client.id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if user is already in too many channels
        let user_channels = database.get_user_channels(&user.nick);
        if user_channels.len() >= 10 { // Default channel limit
            return Err(Error::User("Too many channels".to_string()));
        }
        
        // Check if user is already in the channel
        if user_channels.contains(channel_name) {
            return Err(Error::User("Already on channel".to_string()));
        }
        
        let mut channels = self.channels.write().await;
        
        // Get or create channel
        let channel = if let Some(channel) = channels.get_mut(channel_name) {
            // Check channel restrictions
            if channel.is_invite_only() && !self.is_user_invited(&user.nick, channel_name).await {
                return Err(Error::User("Cannot join channel (+i)".to_string()));
            }
            
            if channel.is_keyed() {
                if let Some(key) = key {
                    if !channel.check_key(key) {
                        return Err(Error::User("Cannot join channel (+k)".to_string()));
                    }
                } else {
                    return Err(Error::User("Cannot join channel (+k)".to_string()));
                }
            }
            
            // Check ban masks
            if self.is_user_banned(&user, channel).await {
                return Err(Error::User("Cannot join channel (+b)".to_string()));
            }
            
            // Check user limit
            if let Some(limit) = channel.user_limit {
                if channel.member_count() >= limit {
                    return Err(Error::User("Cannot join channel (+l)".to_string()));
                }
            }
            
            channel.clone()
        } else {
            // Create new channel
            let mut new_channel = Channel::new(channel_name.clone());
            // First user becomes operator
            new_channel.add_member(user.id)?;
            new_channel.set_operator(&user.id, true)?;
            new_channel.clone()
        };
        
        // Add user to channel
        let mut channel = channel;
        channel.add_member(user.id)?;
        
        // If this is a new channel, make the user an operator
        if channel.member_count() == 1 {
            channel.set_operator(&user.id, true)?;
        }
        
        // Update channels
        channels.insert(channel_name.clone(), channel.clone());
        
        // Update database
        drop(channels);
        drop(database);
        
        let database = self.database.write().await;
        database.add_user_to_channel(&user.nick, channel_name)?;
        
        // Remove from invite list if present
        self.remove_invite(&user.nick, channel_name).await;
        
        // Broadcast JOIN message to channel
        let join_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Join,
            vec![channel_name.clone()],
        );
        
        let broadcast = BroadcastMessage {
            message: join_message,
            target: BroadcastTarget::Channel(channel_name.clone()),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        // Subscribe user to channel for future broadcasts
        broadcast_system.subscribe_to_channel(user.id, channel_name.clone());
        
        tracing::info!("User {} joined channel {}", user.nick, channel_name);
        Ok(())
    }
    
    async fn handle_part(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        if message.params.is_empty() {
            return Err(Error::User("No channel specified".to_string()));
        }
        
        let channel_name = &message.params[0];
        let reason = message.params.get(1).map(|s| s.as_str());
        
        // Get user from database
        let database = self.database.read().await;
        let user = database.get_user(&client.id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if user is in the channel
        let user_channels = database.get_user_channels(&user.nick);
        if !user_channels.contains(channel_name) {
            return Err(Error::User("You're not on that channel".to_string()));
        }
        
        let mut channels = self.channels.write().await;
        
        // Get channel
        let mut channel = channels.get_mut(channel_name)
            .ok_or_else(|| Error::User("No such channel".to_string()))?
            .clone();
        
        // Remove user from channel
        channel.remove_member(&user.id);
        
        // Update channels
        channels.insert(channel_name.clone(), channel.clone());
        
        // Update database
        drop(channels);
        drop(database);
        
        let database = self.database.write().await;
        database.remove_user_from_channel(&user.nick, channel_name)?;
        
        // Broadcast PART message to channel
        let mut part_params = vec![channel_name.clone()];
        if let Some(reason) = reason {
            part_params.push(reason.to_string());
        }
        
        let part_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Part,
            part_params,
        );
        
        let broadcast = BroadcastMessage {
            message: part_message,
            target: BroadcastTarget::Channel(channel_name.clone()),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        // Unsubscribe user from channel
        broadcast_system.unsubscribe_from_channel(&user.id, channel_name);
        
        // If channel is empty, remove it
        if channel.member_count() == 0 {
            let mut channels = self.channels.write().await;
            channels.remove(channel_name);
            tracing::info!("Channel {} removed (empty)", channel_name);
        }
        
        tracing::info!("User {} left channel {}", user.nick, channel_name);
        Ok(())
    }
    
    async fn handle_mode(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        if message.params.is_empty() {
            return Err(Error::User("No target specified".to_string()));
        }
        
        let target = &message.params[0];
        
        // Get user from database
        let database = self.database.read().await;
        let user = database.get_user(&client.id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if target is a channel
        if self.is_valid_channel_name(target) {
            self.handle_channel_mode(&user, target, &message.params[1..]).await?;
        } else {
            // User mode - not implemented yet
            return Err(Error::User("User modes not implemented".to_string()));
        }
        
        Ok(())
    }
    
    async fn handle_channel_mode(&self, user: &User, channel_name: &str, params: &[String]) -> Result<()> {
        let mut channels = self.channels.write().await;
        
        // Get channel
        let mut channel = channels.get_mut(channel_name)
            .ok_or_else(|| Error::User("No such channel".to_string()))?
            .clone();
        
        // Check if user is in the channel
        if !channel.has_member(&user.id) {
            return Err(Error::User("You're not on that channel".to_string()));
        }
        
        // If no mode parameters, just show current modes
        if params.is_empty() {
            let modes = channel.modes_string();
            let mode_params = self.get_mode_params(&channel);
            
            // Send mode reply to user
            let _mode_reply = self.channel_mode_is(channel_name, &modes, &mode_params);
            // TODO: Send reply to client
            
            return Ok(());
        }
        
        // Check if user is an operator
        if !channel.is_operator(&user.id) {
            return Err(Error::User("You're not channel operator".to_string()));
        }
        
        // Parse mode changes
        let mode_string = &params[0];
        let mode_params = &params[1..];
        
        let (add_modes, remove_modes, mode_param_map) = self.parse_mode_string(mode_string, mode_params)?;
        
        let mut changes = Vec::new();
        
        // Apply mode changes
        for mode in &add_modes {
            match mode {
                'o' => {
                    if let Some(nick) = mode_param_map.get(&mode) {
                        if let Some(target_user) = self.get_user_by_nick(nick).await? {
                            if channel.has_member(&target_user.id) {
                                channel.set_operator(&target_user.id, true)?;
                                changes.push(format!("+o {}", nick));
                            }
                        }
                    }
                }
                'v' => {
                    if let Some(nick) = mode_param_map.get(&mode) {
                        if let Some(target_user) = self.get_user_by_nick(nick).await? {
                            if channel.has_member(&target_user.id) {
                                if let Some(member) = channel.members.get_mut(&target_user.id) {
                                    member.add_mode('v');
                                    changes.push(format!("+v {}", nick));
                                }
                            }
                        }
                    }
                }
                'k' => {
                    if let Some(key) = mode_param_map.get(&mode) {
                        channel.set_key(Some(key.clone()));
                        changes.push("+k".to_string());
                    }
                }
                'l' => {
                    if let Some(limit_str) = mode_param_map.get(&mode) {
                        if let Ok(limit) = limit_str.parse::<usize>() {
                            channel.set_user_limit(Some(limit));
                            changes.push(format!("+l {}", limit));
                        }
                    }
                }
                'b' => {
                    if let Some(ban_mask) = mode_param_map.get(&mode) {
                        channel.ban_masks.insert(ban_mask.clone());
                        changes.push(format!("+b {}", ban_mask));
                    }
                }
                'e' => {
                    if let Some(except_mask) = mode_param_map.get(&mode) {
                        channel.exception_masks.insert(except_mask.clone());
                        changes.push(format!("+e {}", except_mask));
                    }
                }
                'I' => {
                    if let Some(invite_mask) = mode_param_map.get(&mode) {
                        channel.invite_masks.insert(invite_mask.clone());
                        changes.push(format!("+I {}", invite_mask));
                    }
                }
                'i' | 'm' | 'n' | 'p' | 's' | 't' => {
                    channel.add_mode(*mode);
                    changes.push(format!("+{}", mode));
                }
                _ => return Err(Error::User("Unknown mode".to_string())),
            }
        }
        
        for mode in &remove_modes {
            match mode {
                'o' => {
                    if let Some(nick) = mode_param_map.get(&mode) {
                        if let Some(target_user) = self.get_user_by_nick(nick).await? {
                            if channel.has_member(&target_user.id) {
                                channel.set_operator(&target_user.id, false)?;
                                changes.push(format!("-o {}", nick));
                            }
                        }
                    }
                }
                'v' => {
                    if let Some(nick) = mode_param_map.get(&mode) {
                        if let Some(target_user) = self.get_user_by_nick(nick).await? {
                            if channel.has_member(&target_user.id) {
                                if let Some(member) = channel.members.get_mut(&target_user.id) {
                                    member.remove_mode('v');
                                    changes.push(format!("-v {}", nick));
                                }
                            }
                        }
                    }
                }
                'k' => {
                    channel.set_key(None);
                    changes.push("-k".to_string());
                }
                'l' => {
                    channel.set_user_limit(None);
                    changes.push("-l".to_string());
                }
                'b' => {
                    if let Some(ban_mask) = mode_param_map.get(&mode) {
                        channel.ban_masks.remove(ban_mask);
                        changes.push(format!("-b {}", ban_mask));
                    }
                }
                'e' => {
                    if let Some(except_mask) = mode_param_map.get(&mode) {
                        channel.exception_masks.remove(except_mask);
                        changes.push(format!("-e {}", except_mask));
                    }
                }
                'I' => {
                    if let Some(invite_mask) = mode_param_map.get(&mode) {
                        channel.invite_masks.remove(invite_mask);
                        changes.push(format!("-I {}", invite_mask));
                    }
                }
                'i' | 'm' | 'n' | 'p' | 's' | 't' => {
                    channel.remove_mode(*mode);
                    changes.push(format!("-{}", mode));
                }
                _ => return Err(Error::User("Unknown mode".to_string())),
            }
        }
        
        // Update channel
        channels.insert(channel_name.to_string(), channel.clone());
        
        // Broadcast mode change to channel
        if !changes.is_empty() {
            let changes_str = changes.join(" ");
            let mut mode_params = vec![channel_name.to_string(), changes_str];
            
            // Add mode parameters
            for (mode, param) in &mode_param_map {
                if add_modes.contains(mode) || remove_modes.contains(mode) {
                    mode_params.push(param.clone());
                }
            }
            
            let mode_message = Message::with_prefix(
                Prefix::User {
                    nick: user.nick.clone(),
                    user: user.username.clone(),
                    host: user.host.clone(),
                },
                MessageType::Mode,
                mode_params,
            );
            
            let broadcast = BroadcastMessage {
                message: mode_message,
                target: BroadcastTarget::Channel(channel_name.to_string()),
                sender: Some(user.id),
                priority: BroadcastPriority::Normal,
            };
            
            let mut broadcast_system = self.broadcast_system.write().await;
            broadcast_system.queue_message(broadcast)?;
        }
        
        tracing::info!("User {} changed modes on channel {}: {:?}", user.nick, channel_name, changes);
        Ok(())
    }
    
    async fn handle_topic(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        if message.params.is_empty() {
            return Err(Error::User("No channel specified".to_string()));
        }
        
        let channel_name = &message.params[0];
        
        // Get user from database
        let database = self.database.read().await;
        let user = database.get_user(&client.id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if user is in the channel
        let user_channels = database.get_user_channels(&user.nick);
        if !user_channels.contains(channel_name) {
            return Err(Error::User("You're not on that channel".to_string()));
        }
        
        let mut channels = self.channels.write().await;
        
        // Get channel
        let mut channel = channels.get_mut(channel_name)
            .ok_or_else(|| Error::User("No such channel".to_string()))?
            .clone();
        
        // If no topic provided, show current topic
        if message.params.len() == 1 {
            if let Some(ref topic) = channel.topic {
                let _topic_reply = self.topic(channel_name, topic);
                // TODO: Send reply to client
                tracing::info!("User {} requested topic for channel {}", user.nick, channel_name);
            } else {
                let _no_topic_reply = self.no_topic(channel_name);
                // TODO: Send reply to client
                tracing::info!("User {} requested topic for channel {} (no topic set)", user.nick, channel_name);
            }
            return Ok(());
        }
        
        // Check if user has permission to set topic
        if channel.topic_ops_only() && !channel.is_operator(&user.id) {
            return Err(Error::User("You're not channel operator".to_string()));
        }
        
        // Set new topic
        let new_topic = &message.params[1];
        let setter = format!("{}!{}@{}", user.nick, user.username, user.host);
        channel.set_topic(new_topic.to_string(), setter);
        
        // Update channel
        channels.insert(channel_name.to_string(), channel.clone());
        
        // Broadcast topic change to channel
        let topic_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Topic,
            vec![channel_name.to_string(), new_topic.to_string()],
        );
        
        let broadcast = BroadcastMessage {
            message: topic_message,
            target: BroadcastTarget::Channel(channel_name.to_string()),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        tracing::info!("User {} set topic on channel {}: {}", user.nick, channel_name, new_topic);
        Ok(())
    }
    
    async fn handle_names(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        // Get user from database
        let database = self.database.read().await;
        let user = database.get_user(&client.id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        let channels = self.channels.read().await;
        
        // If no channels specified, show names for all channels user is in
        let channels_to_show = if message.params.is_empty() {
            database.get_user_channels(&user.nick)
        } else {
            message.params.clone()
        };
        
        for channel_name in channels_to_show {
            if let Some(channel) = channels.get(&channel_name) {
                // Check if user can see this channel
                if channel.is_secret() && !channel.has_member(&user.id) {
                    continue; // Skip secret channels user is not in
                }
                
                // Get member names with prefixes
                let mut names = Vec::new();
                for (member_id, member) in &channel.members {
                    if let Some(member_user) = database.get_user(member_id) {
                        let mut name = String::new();
                        
                        // Add prefixes based on modes
                        if member.is_operator() {
                            name.push('@');
                        } else if member.is_voice() {
                            name.push('+');
                        }
                        
                        name.push_str(&member_user.nick);
                        names.push(name);
                    }
                }
                
                // Sort names (operators first, then voiced users, then regular users)
                names.sort_by(|a, b| {
                    let a_prefix = a.chars().next().unwrap_or(' ');
                    let b_prefix = b.chars().next().unwrap_or(' ');
                    
                    match (a_prefix, b_prefix) {
                        ('@', '@') => a.cmp(b),
                        ('@', _) => std::cmp::Ordering::Less,
                        (_, '@') => std::cmp::Ordering::Greater,
                        ('+', '+') => a.cmp(b),
                        ('+', _) => std::cmp::Ordering::Less,
                        (_, '+') => std::cmp::Ordering::Greater,
                        _ => a.cmp(b),
                    }
                });
                
                // Send names reply (split into multiple messages if too long)
                let names_str = names.join(" ");
                let _names_reply = self.names_reply(&channel_name, &names_str);
                // TODO: Send reply to client
                
                // Send end of names
                let _end_reply = self.end_of_names(&channel_name);
                // TODO: Send reply to client
                
                tracing::info!("Sent names for channel {} to user {}", channel_name, user.nick);
            }
        }
        
        Ok(())
    }
    
    async fn handle_list(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        // Get user from database
        let database = self.database.read().await;
        let user = database.get_user(&client.id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        let channels = self.channels.read().await;
        
        // Send list start
        let _list_start = self.list_start();
        // TODO: Send reply to client
        
        // Get channels to list
        let channels_to_list = if message.params.is_empty() {
            // List all channels
            channels.keys().cloned().collect()
        } else {
            // List specific channels
            message.params.clone()
        };
        
        for channel_name in channels_to_list {
            if let Some(channel) = channels.get(&channel_name) {
                // Check if channel should be visible to user
                let visible = if channel.is_secret() {
                    // Only show secret channels if user is a member
                    channel.has_member(&user.id)
                } else if channel.is_private() {
                    // Only show private channels if user is a member
                    channel.has_member(&user.id)
                } else {
                    // Public channels are always visible
                    true
                };
                
                if visible {
                    let topic = channel.topic.as_deref().unwrap_or("");
                    let member_count = channel.member_count();
                    
                    let _list_reply = self.list(&channel_name, &member_count.to_string(), topic);
                    // TODO: Send reply to client
                    
                    tracing::debug!("Listed channel {} to user {}", channel_name, user.nick);
                }
            }
        }
        
        // Send list end
        let _list_end = self.list_end();
        // TODO: Send reply to client
        
        tracing::info!("Sent channel list to user {}", user.nick);
        Ok(())
    }
    
    async fn handle_invite(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        if message.params.len() < 2 {
            return Err(Error::User("Not enough parameters".to_string()));
        }
        
        let nick = &message.params[0];
        let channel_name = &message.params[1];
        
        // Get user from database
        let database = self.database.read().await;
        let user = database.get_user(&client.id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if user is in the channel
        let user_channels = database.get_user_channels(&user.nick);
        if !user_channels.contains(channel_name) {
            return Err(Error::User("You're not on that channel".to_string()));
        }
        
        // Check if target user exists
        let _target_user = database.get_user_by_nick(nick)
            .ok_or_else(|| Error::User("No such nick".to_string()))?;
        
        // Check if target user is already in the channel
        let target_channels = database.get_user_channels(nick);
        if target_channels.contains(channel_name) {
            return Err(Error::User("is already on channel".to_string()));
        }
        
        let channels = self.channels.read().await;
        
        // Get channel
        let channel = channels.get(channel_name)
            .ok_or_else(|| Error::User("No such channel".to_string()))?;
        
        // Check if user has permission to invite
        if channel.is_operator(&user.id) || !channel.is_invite_only() {
            // User is an operator or channel is not invite-only
        } else {
            return Err(Error::User("You're not channel operator".to_string()));
        }
        
        // Add invite to invite list
        self.add_invite(nick, channel_name).await;
        
        // Send INVITE message to target user
        let invite_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Invite,
            vec![nick.to_string(), channel_name.to_string()],
        );
        
        let broadcast = BroadcastMessage {
            message: invite_message,
            target: BroadcastTarget::Users(vec![nick.to_string()]),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        // Send confirmation to inviting user
        let _inviting_reply = self.inviting(nick, channel_name);
        // TODO: Send reply to client
        
        tracing::info!("User {} invited {} to channel {}", user.nick, nick, channel_name);
        Ok(())
    }
    
    async fn handle_kick(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        if message.params.len() < 2 {
            return Err(Error::User("Not enough parameters".to_string()));
        }
        
        let channel_name = &message.params[0];
        let nick = &message.params[1];
        let reason = message.params.get(2).map(|s| s.as_str());
        
        // Get user from database
        let database = self.database.read().await;
        let user = database.get_user(&client.id)
            .ok_or_else(|| Error::User("User not found".to_string()))?;
        
        // Check if user is in the channel
        let user_channels = database.get_user_channels(&user.nick);
        if !user_channels.contains(channel_name) {
            return Err(Error::User("You're not on that channel".to_string()));
        }
        
        // Check if target user exists
        let target_user = database.get_user_by_nick(nick)
            .ok_or_else(|| Error::User("No such nick".to_string()))?;
        
        // Check if target user is in the channel
        let target_channels = database.get_user_channels(nick);
        if !target_channels.contains(channel_name) {
            return Err(Error::User("They aren't on that channel".to_string()));
        }
        
        let mut channels = self.channels.write().await;
        
        // Get channel
        let mut channel = channels.get_mut(channel_name)
            .ok_or_else(|| Error::User("No such channel".to_string()))?
            .clone();
        
        // Check if user has permission to kick
        if !channel.is_operator(&user.id) {
            return Err(Error::User("You're not channel operator".to_string()));
        }
        
        // Remove target user from channel
        channel.remove_member(&target_user.id);
        
        // Update channel
        channels.insert(channel_name.to_string(), channel.clone());
        
        // Update database
        drop(channels);
        drop(database);
        
        let database = self.database.write().await;
        database.remove_user_from_channel(nick, channel_name)?;
        
        // Remove from invite list if present
        self.remove_invite(nick, channel_name).await;
        
        // Broadcast KICK message to channel
        let mut kick_params = vec![channel_name.to_string(), nick.to_string()];
        if let Some(reason) = reason {
            kick_params.push(reason.to_string());
        }
        
        let kick_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Kick,
            kick_params,
        );
        
        let broadcast = BroadcastMessage {
            message: kick_message,
            target: BroadcastTarget::Channel(channel_name.to_string()),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        // Unsubscribe target user from channel
        broadcast_system.unsubscribe_from_channel(&target_user.id, channel_name);
        
        // If channel is empty, remove it
        if channel.member_count() == 0 {
            let mut channels = self.channels.write().await;
            channels.remove(channel_name);
            tracing::info!("Channel {} removed (empty after kick)", channel_name);
        }
        
        tracing::info!("User {} kicked {} from channel {}", user.nick, nick, channel_name);
        Ok(())
    }
    
    /// Channel-specific error and reply methods
    fn no_such_channel(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("403".to_string()),
            vec!["*".to_string(), channel.to_string(), "No such channel".to_string()],
        )
    }
    
    fn cannot_send_to_chan(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("404".to_string()),
            vec!["*".to_string(), channel.to_string(), "Cannot send to channel".to_string()],
        )
    }
    
    fn too_many_channels(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("405".to_string()),
            vec!["*".to_string(), channel.to_string(), "You have joined too many channels".to_string()],
        )
    }
    
    fn user_not_in_channel(&self, nick: &str, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("441".to_string()),
            vec![nick.to_string(), channel.to_string(), "They aren't on that channel".to_string()],
        )
    }
    
    fn not_on_channel(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("442".to_string()),
            vec!["*".to_string(), channel.to_string(), "You're not on that channel".to_string()],
        )
    }
    
    fn user_on_channel(&self, nick: &str, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("443".to_string()),
            vec![nick.to_string(), channel.to_string(), "is already on channel".to_string()],
        )
    }
    
    fn channel_is_full(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("471".to_string()),
            vec!["*".to_string(), channel.to_string(), "Cannot join channel (+l)".to_string()],
        )
    }
    
    fn unknown_mode(&self, mode: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("472".to_string()),
            vec!["*".to_string(), mode.to_string(), "is unknown mode char to me".to_string()],
        )
    }
    
    fn invite_only_chan(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("473".to_string()),
            vec!["*".to_string(), channel.to_string(), "Cannot join channel (+i)".to_string()],
        )
    }
    
    fn banned_from_chan(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("474".to_string()),
            vec!["*".to_string(), channel.to_string(), "Cannot join channel (+b)".to_string()],
        )
    }
    
    fn bad_channel_key(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("475".to_string()),
            vec!["*".to_string(), channel.to_string(), "Cannot join channel (+k)".to_string()],
        )
    }
    
    fn bad_chan_mask(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("476".to_string()),
            vec!["*".to_string(), channel.to_string(), "Bad Channel Mask".to_string()],
        )
    }
    
    fn no_chan_modes(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("477".to_string()),
            vec!["*".to_string(), channel.to_string(), "Channel doesn't support modes".to_string()],
        )
    }
    
    fn ban_list_full(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("478".to_string()),
            vec!["*".to_string(), channel.to_string(), "Channel list is full".to_string()],
        )
    }
    
    fn chan_op_privs_needed(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("482".to_string()),
            vec!["*".to_string(), channel.to_string(), "You're not channel operator".to_string()],
        )
    }
    
    fn channel_mode_is(&self, channel: &str, modes: &str, mode_params: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("324".to_string()),
            vec!["*".to_string(), channel.to_string(), modes.to_string(), mode_params.to_string()],
        )
    }
    
    fn creation_time(&self, channel: &str, creation_time: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("329".to_string()),
            vec!["*".to_string(), channel.to_string(), creation_time.to_string()],
        )
    }
    
    fn no_topic(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("331".to_string()),
            vec!["*".to_string(), channel.to_string(), "No topic is set".to_string()],
        )
    }
    
    fn topic(&self, channel: &str, topic: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("332".to_string()),
            vec!["*".to_string(), channel.to_string(), topic.to_string()],
        )
    }
    
    fn topic_who_time(&self, channel: &str, nick: &str, time: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("333".to_string()),
            vec!["*".to_string(), channel.to_string(), nick.to_string(), time.to_string()],
        )
    }
    
    fn inviting(&self, nick: &str, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("341".to_string()),
            vec!["*".to_string(), nick.to_string(), channel.to_string()],
        )
    }
    
    fn list_start(&self) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("321".to_string()),
            vec!["*".to_string(), "Channel".to_string(), "Users Name".to_string()],
        )
    }
    
    fn list(&self, channel: &str, visible: &str, topic: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("322".to_string()),
            vec!["*".to_string(), channel.to_string(), visible.to_string(), topic.to_string()],
        )
    }
    
    fn list_end(&self) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("323".to_string()),
            vec!["*".to_string(), "End of /LIST".to_string()],
        )
    }
    
    fn names_reply(&self, channel: &str, names: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("353".to_string()),
            vec!["*".to_string(), "=".to_string(), channel.to_string(), names.to_string()],
        )
    }
    
    fn end_of_names(&self, channel: &str) -> Message {
        Message::new(
            rustircd_core::MessageType::Custom("366".to_string()),
            vec!["*".to_string(), channel.to_string(), "End of /NAMES list".to_string()],
        )
    }
    
    // Helper methods
    
    /// Check if a string is a valid channel name
    fn is_valid_channel_name(&self, name: &str) -> bool {
        if name.is_empty() || name.len() > 50 {
            return false;
        }
        
        // Channel names must start with #, &, +, or !
        match name.chars().next() {
            Some('#') | Some('&') | Some('+') | Some('!') => {},
            _ => return false,
        }
        
        // Check for invalid characters
        for c in name.chars().skip(1) {
            if c == ' ' || c == ',' || c == 7 as char { // 7 = BELL character
                return false;
            }
        }
        
        true
    }
    
    /// Check if user is invited to a channel
    async fn is_user_invited(&self, nick: &str, channel: &str) -> bool {
        let invite_list = self.invite_list.read().await;
        invite_list.get(nick)
            .map(|channels| channels.contains(channel))
            .unwrap_or(false)
    }
    
    /// Add user to invite list
    async fn add_invite(&self, nick: &str, channel: &str) {
        let mut invite_list = self.invite_list.write().await;
        invite_list.entry(nick.to_string())
            .or_insert_with(HashSet::new)
            .insert(channel.to_string());
    }
    
    /// Remove user from invite list
    async fn remove_invite(&self, nick: &str, channel: &str) {
        let mut invite_list = self.invite_list.write().await;
        if let Some(channels) = invite_list.get_mut(nick) {
            channels.remove(channel);
            if channels.is_empty() {
                invite_list.remove(nick);
            }
        }
    }
    
    /// Check if user is banned from channel
    async fn is_user_banned(&self, user: &User, channel: &Channel) -> bool {
        // Check ban masks
        for ban_mask in &channel.ban_masks {
            if self.matches_mask(user, ban_mask) {
                // Check exception masks
                for except_mask in &channel.exception_masks {
                    if self.matches_mask(user, except_mask) {
                        return false; // Exception overrides ban
                    }
                }
                return true; // User is banned
            }
        }
        false
    }
    
    /// Check if user matches a mask (nick!user@host format)
    fn matches_mask(&self, user: &User, mask: &str) -> bool {
        let user_mask = format!("{}!{}@{}", user.nick, user.username, user.host);
        self.matches_pattern(&user_mask, mask)
    }
    
    /// Simple pattern matching for IRC masks
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();
        
        self.matches_pattern_recursive(&text_chars, &pattern_chars, 0, 0)
    }
    
    fn matches_pattern_recursive(&self, text: &[char], pattern: &[char], text_idx: usize, pattern_idx: usize) -> bool {
        if pattern_idx >= pattern.len() {
            return text_idx >= text.len();
        }
        
        match pattern[pattern_idx] {
            '*' => {
                // Try matching * with 0 or more characters
                for i in text_idx..=text.len() {
                    if self.matches_pattern_recursive(text, pattern, i, pattern_idx + 1) {
                        return true;
                    }
                }
                false
            }
            '?' => {
                // Match any single character
                if text_idx < text.len() {
                    self.matches_pattern_recursive(text, pattern, text_idx + 1, pattern_idx + 1)
                } else {
                    false
                }
            }
            c => {
                // Match exact character (case-insensitive)
                if text_idx < text.len() && text[text_idx].to_lowercase().next() == c.to_lowercase().next() {
                    self.matches_pattern_recursive(text, pattern, text_idx + 1, pattern_idx + 1)
                } else {
                    false
                }
            }
        }
    }
    
    /// Get user by nickname
    async fn get_user_by_nick(&self, nick: &str) -> Result<Option<User>> {
        let database = self.database.read().await;
        Ok(database.get_user_by_nick(nick))
    }
    
    /// Get mode parameters for a channel
    fn get_mode_params(&self, channel: &Channel) -> String {
        let mut params = Vec::new();
        
        if let Some(ref key) = channel.key {
            params.push(key.clone());
        }
        
        if let Some(limit) = channel.user_limit {
            params.push(limit.to_string());
        }
        
        params.join(" ")
    }
    
    /// Parse mode string and parameters
    fn parse_mode_string(&self, mode_string: &str, mode_params: &[String]) -> Result<(Vec<char>, Vec<char>, HashMap<char, String>)> {
        let mut add_modes = Vec::new();
        let mut remove_modes = Vec::new();
        let mut mode_param_map = HashMap::new();
        
        let mut chars = mode_string.chars();
        let mut adding = true;
        let mut param_idx = 0;
        
        while let Some(c) = chars.next() {
            match c {
                '+' => adding = true,
                '-' => adding = false,
                'o' | 'v' | 'k' | 'l' | 'b' | 'e' | 'I' => {
                    if adding {
                        add_modes.push(c);
                    } else {
                        remove_modes.push(c);
                    }
                    
                    // These modes require parameters
                    if param_idx < mode_params.len() {
                        mode_param_map.insert(c, mode_params[param_idx].clone());
                        param_idx += 1;
                    }
                }
                'i' | 'm' | 'n' | 'p' | 's' | 't' => {
                    if adding {
                        add_modes.push(c);
                    } else {
                        remove_modes.push(c);
                    }
                }
                _ => return Err(Error::User("Unknown mode character".to_string())),
            }
        }
        
        Ok((add_modes, remove_modes, mode_param_map))
    }
    
    // Notification methods
    
    /// Notify channel members of a user joining
    async fn notify_user_joined(&self, user: &User, channel_name: &str) -> Result<()> {
        let join_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Join,
            vec![channel_name.to_string()],
        );
        
        let broadcast = BroadcastMessage {
            message: join_message,
            target: BroadcastTarget::Channel(channel_name.to_string()),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        tracing::info!("Notified channel {} of user {} joining", channel_name, user.nick);
        Ok(())
    }
    
    /// Notify channel members of a user leaving
    async fn notify_user_left(&self, user: &User, channel_name: &str, reason: Option<&str>) -> Result<()> {
        let mut part_params = vec![channel_name.to_string()];
        if let Some(reason) = reason {
            part_params.push(reason.to_string());
        }
        
        let part_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Part,
            part_params,
        );
        
        let broadcast = BroadcastMessage {
            message: part_message,
            target: BroadcastTarget::Channel(channel_name.to_string()),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        tracing::info!("Notified channel {} of user {} leaving", channel_name, user.nick);
        Ok(())
    }
    
    /// Notify channel members of a user being kicked
    async fn notify_user_kicked(&self, kicker: &User, kicked_user: &User, channel_name: &str, reason: Option<&str>) -> Result<()> {
        let mut kick_params = vec![channel_name.to_string(), kicked_user.nick.clone()];
        if let Some(reason) = reason {
            kick_params.push(reason.to_string());
        }
        
        let kick_message = Message::with_prefix(
            Prefix::User {
                nick: kicker.nick.clone(),
                user: kicker.username.clone(),
                host: kicker.host.clone(),
            },
            MessageType::Kick,
            kick_params,
        );
        
        let broadcast = BroadcastMessage {
            message: kick_message,
            target: BroadcastTarget::Channel(channel_name.to_string()),
            sender: Some(kicker.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        tracing::info!("Notified channel {} of user {} being kicked by {}", channel_name, kicked_user.nick, kicker.nick);
        Ok(())
    }
    
    /// Notify channel members of a topic change
    async fn notify_topic_changed(&self, user: &User, channel_name: &str, topic: &str) -> Result<()> {
        let topic_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Topic,
            vec![channel_name.to_string(), topic.to_string()],
        );
        
        let broadcast = BroadcastMessage {
            message: topic_message,
            target: BroadcastTarget::Channel(channel_name.to_string()),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        tracing::info!("Notified channel {} of topic change by user {}", channel_name, user.nick);
        Ok(())
    }
    
    /// Notify channel members of mode changes
    async fn notify_mode_changed(&self, user: &User, channel_name: &str, mode_string: &str, mode_params: Vec<String>) -> Result<()> {
        let mut mode_message_params = vec![channel_name.to_string(), mode_string.to_string()];
        mode_message_params.extend(mode_params);
        
        let mode_message = Message::with_prefix(
            Prefix::User {
                nick: user.nick.clone(),
                user: user.username.clone(),
                host: user.host.clone(),
            },
            MessageType::Mode,
            mode_message_params,
        );
        
        let broadcast = BroadcastMessage {
            message: mode_message,
            target: BroadcastTarget::Channel(channel_name.to_string()),
            sender: Some(user.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        tracing::info!("Notified channel {} of mode change by user {}", channel_name, user.nick);
        Ok(())
    }
    
    /// Notify user of channel invitation
    async fn notify_invitation(&self, inviter: &User, target_user: &User, channel_name: &str) -> Result<()> {
        let invite_message = Message::with_prefix(
            Prefix::User {
                nick: inviter.nick.clone(),
                user: inviter.username.clone(),
                host: inviter.host.clone(),
            },
            MessageType::Invite,
            vec![target_user.nick.clone(), channel_name.to_string()],
        );
        
        let broadcast = BroadcastMessage {
            message: invite_message,
            target: BroadcastTarget::Users(vec![target_user.nick.clone()]),
            sender: Some(inviter.id),
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        tracing::info!("Notified user {} of invitation to channel {} by {}", target_user.nick, channel_name, inviter.nick);
        Ok(())
    }
    
    /// Notify all users when a channel is created
    async fn notify_channel_created(&self, creator: &User, channel_name: &str) -> Result<()> {
        tracing::info!("Channel {} created by user {}", channel_name, creator.nick);
        
        // Channel creation is typically only visible to the creator
        // Additional notifications could be added here if needed
        Ok(())
    }
    
    /// Notify all users when a channel is destroyed
    async fn notify_channel_destroyed(&self, channel_name: &str, reason: Option<&str>) -> Result<()> {
        // This would typically be used when the last user leaves a channel
        // or when a server admin destroys a channel
        tracing::info!("Channel {} destroyed. Reason: {:?}", channel_name, reason);
        
        // Additional cleanup and notifications could be added here
        Ok(())
    }
    
    /// Send error message to a specific user
    async fn send_error_to_user(&self, user_id: Uuid, error_message: Message) -> Result<()> {
        let broadcast = BroadcastMessage {
            message: error_message,
            target: BroadcastTarget::Users(vec![user_id.to_string()]),
            sender: None,
            priority: BroadcastPriority::High,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        Ok(())
    }
    
    /// Send reply message to a specific user
    async fn send_reply_to_user(&self, user_id: Uuid, reply_message: Message) -> Result<()> {
        let broadcast = BroadcastMessage {
            message: reply_message,
            target: BroadcastTarget::Users(vec![user_id.to_string()]),
            sender: None,
            priority: BroadcastPriority::Normal,
        };
        
        let mut broadcast_system = self.broadcast_system.write().await;
        broadcast_system.queue_message(broadcast)?;
        
        Ok(())
    }
}

// BurstExtension implementation removed - extensions system was removed
