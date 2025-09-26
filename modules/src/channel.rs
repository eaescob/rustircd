//! Channel operations module

use rustircd_core::{Module, ModuleResult, Client, Message, User, Error, Result, NumericReply};
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
    pub fn add_member(&mut self, user_id: Uuid) -> Result<(), String> {
        if self.members.contains_key(&user_id) {
            return Err("User already in channel".to_string());
        }
        
        if let Some(limit) = self.user_limit {
            if self.members.len() >= limit {
                return Err("Channel is full".to_string());
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
    pub fn set_operator(&mut self, user_id: &Uuid, is_op: bool) -> Result<(), String> {
        if let Some(member) = self.members.get_mut(user_id) {
            if is_op {
                member.add_mode('o');
            } else {
                member.remove_mode('o');
            }
            Ok(())
        } else {
            Err("User not in channel".to_string())
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
        self.key = key;
        if key.is_some() {
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
    
    async fn handle_message(&mut self, client: &Client, message: &Message) -> Result<ModuleResult> {
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
    
    async fn handle_server_message(&mut self, _server: &str, _message: &Message) -> Result<ModuleResult> {
        Ok(ModuleResult::NotHandled)
    }
    
    async fn handle_user_registration(&mut self, _user: &User) -> Result<()> {
        Ok(())
    }
    
    async fn handle_user_disconnection(&mut self, _user: &User) -> Result<()> {
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
        
        // Validate channel name
        if !rustircd_core::utils::string::is_valid_channel_name(channel_name) {
            return Err(Error::Channel("Invalid channel name".to_string()));
        }
        
        // TODO: Implement channel join logic
        tracing::info!("Client {} joining channel {}", client.id, channel_name);
        
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
        
        // TODO: Implement channel part logic
        tracing::info!("Client {} leaving channel {}", client.id, channel_name);
        
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
        
        // TODO: Implement mode logic
        tracing::info!("Client {} setting mode on {}", client.id, target);
        
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
        
        // TODO: Implement topic logic
        tracing::info!("Client {} setting topic on {}", client.id, channel_name);
        
        Ok(())
    }
    
    async fn handle_names(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        // TODO: Implement names logic
        tracing::info!("Client {} requesting names", client.id);
        
        Ok(())
    }
    
    async fn handle_list(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        // TODO: Implement list logic
        tracing::info!("Client {} requesting channel list", client.id);
        
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
        let channel = &message.params[1];
        
        // TODO: Implement invite logic
        tracing::info!("Client {} inviting {} to {}", client.id, nick, channel);
        
        Ok(())
    }
    
    async fn handle_kick(&self, client: &Client, message: &Message) -> Result<()> {
        if !client.is_registered() {
            return Err(Error::User("Client not registered".to_string()));
        }
        
        if message.params.len() < 2 {
            return Err(Error::User("Not enough parameters".to_string()));
        }
        
        let channel = &message.params[0];
        let nick = &message.params[1];
        
        // TODO: Implement kick logic
        tracing::info!("Client {} kicking {} from {}", client.id, nick, channel);
        
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
}
