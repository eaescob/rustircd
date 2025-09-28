//! IRC numeric replies as defined in RFC 1459

use crate::Message;

/// IRC numeric reply codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NumericReply {
    // Connection registration
    RplWelcome = 001,
    RplYourHost = 002,
    RplCreated = 003,
    RplMyInfo = 004,
    RplBounce = 005,
    
    // Server queries
    RplAdminMe = 256,
    RplAdminLoc1 = 257,
    RplAdminLoc2 = 258,
    RplAdminEmail = 259,
    RplVersion = 351,
    RplWhoisUser = 311,
    RplWhoisServer = 312,
    RplWhoisOperator = 313,
    RplWhoisIdle = 317,
    RplEndOfWhois = 318,
    RplWhoisChannels = 319,
    RplWhoisSpecial = 320,
    RplList = 322,
    RplListEnd = 323,
    RplChannelModeIs = 324,
    RplNoTopic = 331,
    RplTopic = 332,
    RplInviting = 341,
    RplSummoning = 342,
    RplInviteList = 346,
    RplEndOfInviteList = 347,
    RplExceptList = 348,
    RplEndOfExceptList = 349,
    RplWhoReply = 352,
    RplEndOfWho = 315,
    RplNameReply = 353,
    RplEndOfNames = 366,
    RplLinks = 364,
    RplEndOfLinks = 365,
    RplBanList = 367,
    RplEndOfBanList = 368,
    RplEndOfWhoWas = 369,
    RplInfo = 371,
    RplEndOfInfo = 374,
    RplMotdStart = 375,
    RplMotd = 372,
    RplEndOfMotd = 376,
    RplYoureOper = 381,
    RplRehashing = 382,
    RplTime = 391,
    RplUsersStart = 392,
    RplUsers = 393,
    RplEndOfUsers = 394,
    RplNoUsers = 395,
    RplTraceLink = 200,
    RplTraceConnecting = 201,
    RplTraceHandshake = 202,
    RplTraceUnknown = 203,
    RplTraceOperator = 204,
    RplTraceUser = 205,
    RplTraceServer = 206,
    RplTraceService = 207,
    RplTraceNewType = 208,
    RplTraceClass = 209,
    RplTraceLog = 261,
    RplTraceEnd = 262,
    RplStatsLinkInfo = 211,
    RplStatsCommands = 212,
    RplStatsCLine = 213,
    RplStatsNLine = 214,
    RplStatsILine = 215,
    RplStatsKLine = 216,
    RplStatsYLine = 218,
    RplEndOfStats = 219,
    RplStatsLLine = 241,
    RplStatsUptime = 242,
    RplStatsOLine = 243,
    RplStatsHLine = 244,
    RplUmodeIs = 221,
    RplServList = 234,
    RplServListEnd = 235,
    RplLUserClient = 251,
    RplLUserOp = 252,
    RplLUserUnknown = 253,
    RplLUserChannels = 254,
    RplLUserMe = 255,
    RplLocalUsers = 265,
    RplGlobalUsers = 266,
    RplAway = 301,
    RplUnaway = 305,
    RplNowAway = 306,
    RplUserhost = 302,
    RplIson = 303,
    
    // Error replies
    ErrNoSuchNick = 401,
    ErrNoSuchServer = 402,
    ErrNoSuchChannel = 403,
    ErrCannotSendToChan = 404,
    ErrTooManyChannels = 405,
    ErrWasNoSuchNick = 406,
    ErrTooManyTargets = 407,
    ErrNoSuchService = 408,
    ErrNoOrigin = 409,
    ErrNoRecipients = 411,
    ErrNoTextToSend = 412,
    ErrNoTopLevel = 413,
    ErrWildTopLevel = 414,
    ErrBadMask = 415,
    ErrUnknownCommand = 421,
    ErrNoMotd = 422,
    ErrNoAdminInfo = 423,
    ErrFileError = 424,
    ErrNoNicknameGiven = 431,
    ErrErroneousNickname = 432,
    ErrNicknameInUse = 433,
    ErrNickCollision = 436,
    ErrUnavailResource = 437,
    ErrUserNotInChannel = 441,
    ErrNotOnChannel = 442,
    ErrUserOnChannel = 443,
    ErrNoLogin = 444,
    ErrSummonDisabled = 445,
    ErrUsersDisabled = 446,
    ErrNotRegistered = 451,
    ErrNeedMoreParams = 461,
    ErrAlreadyRegistered = 462,
    ErrNoPermForHost = 463,
    ErrPasswordMismatch = 464,
    ErrYoureBannedCreep = 465,
    ErrKeySet = 467,
    ErrChannelIsFull = 471,
    ErrUnknownMode = 472,
    ErrInviteOnlyChan = 473,
    ErrBannedFromChan = 474,
    ErrBadChannelKey = 475,
    ErrBadChanMask = 476,
    ErrNoChanModes = 477,
    ErrBanListFull = 478,
    ErrNoPrivileges = 481,
    ErrChanOpPrivsNeeded = 482,
    ErrCantKillServer = 483,
    ErrRestricted = 484,
    ErrUniqOpPrivsNeeded = 485,
    ErrNoOperHost = 491,
    ErrUModeUnknownFlag = 501,
    ErrUsersDontMatch = 502,
    // Custom numeric replies
    Custom(u16),
}

impl NumericReply {
    /// Get the numeric code as a u16
    pub fn numeric_code(&self) -> u16 {
        match self {
            NumericReply::RplWelcome => 001,
            NumericReply::RplYourHost => 002,
            NumericReply::RplCreated => 003,
            NumericReply::RplMyInfo => 004,
            NumericReply::RplBounce => 005,
            NumericReply::RplAdminMe => 256,
            NumericReply::RplAdminLoc1 => 257,
            NumericReply::RplAdminLoc2 => 258,
            NumericReply::RplAdminEmail => 259,
            NumericReply::RplVersion => 351,
            NumericReply::RplWhoisUser => 311,
            NumericReply::RplWhoisServer => 312,
            NumericReply::RplWhoisOperator => 313,
            NumericReply::RplWhoisIdle => 317,
            NumericReply::RplEndOfWhois => 318,
            NumericReply::RplWhoisChannels => 319,
            NumericReply::RplWhoisSpecial => 320,
            NumericReply::RplList => 322,
            NumericReply::RplListEnd => 323,
            NumericReply::RplChannelModeIs => 324,
            NumericReply::RplNoTopic => 331,
            NumericReply::RplTopic => 332,
            NumericReply::RplInviting => 341,
            NumericReply::RplSummoning => 342,
            NumericReply::RplInviteList => 346,
            NumericReply::RplEndOfInviteList => 347,
            NumericReply::RplExceptList => 348,
            NumericReply::RplEndOfExceptList => 349,
            NumericReply::RplWhoReply => 352,
            NumericReply::RplEndOfWho => 315,
            NumericReply::RplNameReply => 353,
            NumericReply::RplEndOfNames => 366,
            NumericReply::RplLinks => 364,
            NumericReply::RplEndOfLinks => 365,
            NumericReply::RplBanList => 367,
            NumericReply::RplEndOfBanList => 368,
            NumericReply::RplEndOfWhoWas => 369,
            NumericReply::RplInfo => 371,
            NumericReply::RplEndOfInfo => 374,
            NumericReply::RplMotdStart => 375,
            NumericReply::RplMotd => 372,
            NumericReply::RplEndOfMotd => 376,
            NumericReply::RplYoureOper => 381,
            NumericReply::RplRehashing => 382,
            NumericReply::RplTime => 391,
            NumericReply::RplUsersStart => 392,
            NumericReply::RplUsers => 393,
            NumericReply::RplEndOfUsers => 394,
            NumericReply::RplNoUsers => 395,
            NumericReply::RplTraceLink => 200,
            NumericReply::RplTraceConnecting => 201,
            NumericReply::RplTraceHandshake => 202,
            NumericReply::RplTraceUnknown => 203,
            NumericReply::RplTraceOperator => 204,
            NumericReply::RplTraceUser => 205,
            NumericReply::RplTraceServer => 206,
            NumericReply::RplTraceService => 207,
            NumericReply::RplTraceNewType => 208,
            NumericReply::RplTraceClass => 209,
            NumericReply::RplTraceLog => 261,
            NumericReply::RplTraceEnd => 262,
            NumericReply::RplStatsLinkInfo => 211,
            NumericReply::RplStatsCommands => 212,
            NumericReply::RplEndOfStats => 219,
            NumericReply::RplStatsUptime => 242,
            NumericReply::RplStatsOLine => 243,
            NumericReply::RplUmodeIs => 221,
            NumericReply::RplServList => 234,
            NumericReply::RplServListEnd => 235,
            NumericReply::RplLUserClient => 251,
            NumericReply::RplLUserOp => 252,
            NumericReply::RplLUserUnknown => 253,
            NumericReply::RplLUserChannels => 254,
            NumericReply::RplLUserMe => 255,
            NumericReply::RplAway => 301,
            NumericReply::RplUserhost => 302,
            NumericReply::RplIson => 303,
            NumericReply::RplUnaway => 305,
            NumericReply::RplNowAway => 306,
            NumericReply::ErrNoSuchNick => 401,
            NumericReply::ErrNoSuchServer => 402,
            NumericReply::ErrNoSuchChannel => 403,
            NumericReply::ErrCannotSendToChan => 404,
            NumericReply::ErrTooManyChannels => 405,
            NumericReply::ErrWasNoSuchNick => 406,
            NumericReply::ErrTooManyTargets => 407,
            NumericReply::ErrNoSuchService => 408,
            NumericReply::ErrNoOrigin => 409,
            NumericReply::ErrNoRecipients => 411,
            NumericReply::ErrNoTextToSend => 412,
            NumericReply::ErrNoTopLevel => 413,
            NumericReply::ErrWildTopLevel => 414,
            NumericReply::ErrBadMask => 415,
            NumericReply::ErrUnknownCommand => 421,
            NumericReply::ErrNoMotd => 422,
            NumericReply::ErrNoAdminInfo => 423,
            NumericReply::ErrFileError => 424,
            NumericReply::ErrNoNicknameGiven => 431,
            NumericReply::ErrErroneousNickname => 432,
            NumericReply::ErrNicknameInUse => 433,
            NumericReply::ErrNickCollision => 436,
            NumericReply::ErrUnavailResource => 437,
            NumericReply::ErrUserNotInChannel => 441,
            NumericReply::ErrNotOnChannel => 442,
            NumericReply::ErrUserOnChannel => 443,
            NumericReply::ErrNoLogin => 444,
            NumericReply::ErrSummonDisabled => 445,
            NumericReply::ErrUsersDisabled => 446,
            NumericReply::ErrNotRegistered => 451,
            NumericReply::ErrNeedMoreParams => 461,
            NumericReply::ErrAlreadyRegistered => 462,
            NumericReply::ErrNoPermForHost => 463,
            NumericReply::ErrPasswordMismatch => 464,
            NumericReply::ErrYoureBannedCreep => 465,
            NumericReply::ErrKeySet => 467,
            NumericReply::ErrChannelIsFull => 471,
            NumericReply::ErrUnknownMode => 472,
            NumericReply::ErrInviteOnlyChan => 473,
            NumericReply::ErrBannedFromChan => 474,
            NumericReply::ErrBadChannelKey => 475,
            NumericReply::ErrBadChanMask => 476,
            NumericReply::ErrNoChanModes => 477,
            NumericReply::ErrBanListFull => 478,
            NumericReply::ErrNoPrivileges => 481,
            NumericReply::ErrChanOpPrivsNeeded => 482,
            NumericReply::ErrCantKillServer => 483,
            NumericReply::ErrRestricted => 484,
            NumericReply::ErrUsersDontMatch => 502,
            NumericReply::RplStatsCLine => 213,
            NumericReply::RplStatsNLine => 214,
            NumericReply::RplStatsILine => 215,
            NumericReply::RplStatsKLine => 216,
            NumericReply::RplStatsYLine => 218,
            NumericReply::RplStatsLLine => 241,
            NumericReply::RplStatsHLine => 244,
            NumericReply::RplLocalUsers => 265,
            NumericReply::RplGlobalUsers => 266,
            NumericReply::ErrUniqOpPrivsNeeded => 485,
            NumericReply::ErrNoOperHost => 491,
            NumericReply::ErrUModeUnknownFlag => 501,
            NumericReply::Custom(code) => *code,
        }
    }
    
    /// Get the numeric code as a string
    pub fn code(&self) -> String {
        match self {
            NumericReply::Custom(code) => format!("{:03}", code),
            _ => {
                // For non-Custom variants, we need to match each case
                let code = match self {
                    NumericReply::RplWelcome => 1,
                    NumericReply::RplYourHost => 2,
                    NumericReply::RplCreated => 3,
                    NumericReply::RplMyInfo => 4,
                    NumericReply::RplBounce => 5,
                    NumericReply::RplAdminMe => 256,
                    NumericReply::RplAdminLoc1 => 257,
                    NumericReply::RplAdminLoc2 => 258,
                    NumericReply::RplAdminEmail => 259,
                    NumericReply::RplVersion => 351,
                    NumericReply::RplWhoisUser => 311,
                    NumericReply::RplWhoisServer => 312,
                    NumericReply::RplWhoisOperator => 313,
                    NumericReply::RplWhoisIdle => 317,
                    NumericReply::RplEndOfWhois => 318,
                    NumericReply::RplWhoisChannels => 319,
                    NumericReply::RplWhoisSpecial => 320,
                    NumericReply::RplList => 322,
                    NumericReply::RplListEnd => 323,
                    NumericReply::RplChannelModeIs => 324,
                    NumericReply::RplNoTopic => 331,
                    NumericReply::RplTopic => 332,
                    NumericReply::RplInviting => 341,
                    NumericReply::RplSummoning => 342,
                    NumericReply::RplInviteList => 346,
                    NumericReply::RplEndOfInviteList => 347,
                    NumericReply::RplExceptList => 348,
                    NumericReply::RplEndOfExceptList => 349,
                    NumericReply::RplWhoReply => 352,
                    NumericReply::RplEndOfWho => 315,
                    NumericReply::RplNameReply => 353,
                    NumericReply::RplEndOfNames => 366,
                    NumericReply::RplLinks => 364,
                    NumericReply::RplEndOfLinks => 365,
                    NumericReply::RplBanList => 367,
                    NumericReply::RplEndOfBanList => 368,
                    NumericReply::RplEndOfWhoWas => 369,
                    NumericReply::RplInfo => 371,
                    NumericReply::RplEndOfInfo => 374,
                    NumericReply::RplMotdStart => 375,
                    NumericReply::RplMotd => 372,
                    NumericReply::RplEndOfMotd => 376,
                    NumericReply::RplYoureOper => 381,
                    NumericReply::RplRehashing => 382,
                    NumericReply::RplTime => 391,
                    NumericReply::RplUsersStart => 392,
                    NumericReply::RplUsers => 393,
                    NumericReply::RplEndOfUsers => 394,
                    NumericReply::RplNoUsers => 395,
                    NumericReply::RplTraceLink => 200,
                    NumericReply::RplTraceConnecting => 201,
                    NumericReply::RplTraceHandshake => 202,
                    NumericReply::RplTraceUnknown => 203,
                    NumericReply::RplTraceOperator => 204,
                    NumericReply::RplTraceUser => 205,
                    NumericReply::RplTraceServer => 206,
                    NumericReply::RplTraceService => 207,
                    NumericReply::RplTraceNewType => 208,
                    NumericReply::RplTraceClass => 209,
                    NumericReply::RplTraceLog => 261,
                    NumericReply::RplTraceEnd => 262,
                    NumericReply::RplStatsLinkInfo => 211,
                    NumericReply::RplStatsCommands => 212,
                    NumericReply::RplEndOfStats => 219,
                    NumericReply::RplStatsUptime => 242,
                    NumericReply::RplStatsOLine => 243,
                    NumericReply::RplUmodeIs => 221,
                    NumericReply::RplServList => 234,
                    NumericReply::RplServListEnd => 235,
                    NumericReply::RplLUserClient => 251,
                    NumericReply::RplLUserOp => 252,
                    NumericReply::RplLUserUnknown => 253,
                    NumericReply::RplLUserChannels => 254,
                    NumericReply::RplLUserMe => 255,
                    NumericReply::RplAway => 301,
                    NumericReply::RplUserhost => 302,
                    NumericReply::RplIson => 303,
                    NumericReply::RplUnaway => 305,
                    NumericReply::RplNowAway => 306,
                    NumericReply::ErrNoSuchNick => 401,
                    NumericReply::ErrNoSuchServer => 402,
                    NumericReply::ErrNoSuchChannel => 403,
                    NumericReply::ErrCannotSendToChan => 404,
                    NumericReply::ErrTooManyChannels => 405,
                    NumericReply::ErrWasNoSuchNick => 406,
                    NumericReply::ErrTooManyTargets => 407,
                    NumericReply::ErrNoSuchService => 408,
                    NumericReply::ErrNoOrigin => 409,
                    NumericReply::ErrNoRecipients => 411,
                    NumericReply::ErrNoTextToSend => 412,
                    NumericReply::ErrNoTopLevel => 413,
                    NumericReply::ErrWildTopLevel => 414,
                    NumericReply::ErrBadMask => 415,
                    NumericReply::ErrUnknownCommand => 421,
                    NumericReply::ErrNoMotd => 422,
                    NumericReply::ErrNoAdminInfo => 423,
                    NumericReply::ErrFileError => 424,
                    NumericReply::ErrNoNicknameGiven => 431,
                    NumericReply::ErrErroneousNickname => 432,
                    NumericReply::ErrNicknameInUse => 433,
                    NumericReply::ErrNickCollision => 436,
                    NumericReply::ErrUnavailResource => 437,
                    NumericReply::ErrUserNotInChannel => 441,
                    NumericReply::ErrNotOnChannel => 442,
                    NumericReply::ErrUserOnChannel => 443,
                    NumericReply::ErrNoLogin => 444,
                    NumericReply::ErrSummonDisabled => 445,
                    NumericReply::ErrUsersDisabled => 446,
                    NumericReply::ErrNotRegistered => 451,
                    NumericReply::ErrNeedMoreParams => 461,
                    NumericReply::ErrAlreadyRegistered => 462,
                    NumericReply::ErrNoPermForHost => 463,
                    NumericReply::ErrPasswordMismatch => 464,
                    NumericReply::ErrYoureBannedCreep => 465,
                    NumericReply::ErrKeySet => 467,
                    NumericReply::ErrChannelIsFull => 471,
                    NumericReply::ErrUnknownMode => 472,
                    NumericReply::ErrInviteOnlyChan => 473,
                    NumericReply::ErrBannedFromChan => 474,
                    NumericReply::ErrBadChannelKey => 475,
                    NumericReply::ErrBadChanMask => 476,
                    NumericReply::ErrNoChanModes => 477,
                    NumericReply::ErrBanListFull => 478,
                    NumericReply::ErrNoPrivileges => 481,
                    NumericReply::ErrChanOpPrivsNeeded => 482,
                    NumericReply::ErrCantKillServer => 483,
                    NumericReply::ErrRestricted => 484,
                    NumericReply::ErrUniqOpPrivsNeeded => 485,
                    NumericReply::ErrNoOperHost => 491,
                    NumericReply::ErrUModeUnknownFlag => 501,
                    NumericReply::ErrUsersDontMatch => 502,
                    NumericReply::RplStatsCLine => 213,
                    NumericReply::RplStatsNLine => 214,
                    NumericReply::RplStatsILine => 215,
                    NumericReply::RplStatsKLine => 216,
                    NumericReply::RplStatsYLine => 218,
                    NumericReply::RplStatsLLine => 241,
                    NumericReply::RplStatsHLine => 244,
                    NumericReply::RplLocalUsers => 265,
                    NumericReply::RplGlobalUsers => 266,
                    NumericReply::Custom(_) => unreachable!(), // Already handled above
                };
                format!("{:03}", code)
            }
        }
    }
    
    /// Create a numeric reply message
    pub fn reply(&self, target: &str, params: Vec<String>) -> Message {
        let mut all_params = vec![target.to_string()];
        all_params.extend(params);
        
        Message::new(
            crate::MessageType::Custom(self.code()),
            all_params,
        )
    }
    
    /// Create a numeric reply message using configurable replies
    pub fn reply_with_config(&self, target: &str, params: &std::collections::HashMap<String, String>, replies_config: &crate::RepliesConfig, server_info: &crate::RepliesServerInfo) -> Message {
        let code = self.numeric_code();
        
        // Try to get custom reply text from configuration
        if let Some(reply_text) = replies_config.format_reply(code, params, server_info) {
            // Split the reply text into parts (target + message)
            let parts: Vec<&str> = reply_text.splitn(2, ' ').collect();
            if parts.len() >= 2 {
                let message = parts[1].to_string();
                return Message::new(
                    crate::MessageType::Custom(self.code()),
                    vec![target.to_string(), message],
                );
            }
        }
        
        // Fall back to default behavior
        self.reply(target, vec![])
    }
}

/// Common numeric replies
impl NumericReply {
    /// RPL_WELCOME
    pub fn welcome(_server: &str, nick: &str, user: &str, host: &str) -> Message {
        Self::RplWelcome.reply(
            nick,
            vec![format!("Welcome to the Internet Relay Network {}!{}@{}", nick, user, host)],
        )
    }
    
    /// RPL_YOURHOST
    pub fn your_host(server: &str, version: &str) -> Message {
        Self::RplYourHost.reply(
            "client",
            vec![format!("Your host is {}, running version {}", server, version)],
        )
    }
    
    /// RPL_CREATED
    pub fn created(_server: &str, date: &str) -> Message {
        Self::RplCreated.reply(
            "client",
            vec![format!("This server was created {}", date)],
        )
    }
    
    /// RPL_MYINFO
    pub fn my_info(server: &str, version: &str, user_modes: &str, channel_modes: &str) -> Message {
        Self::RplMyInfo.reply(
            "client",
            vec![format!("{} {} {} {}", server, version, user_modes, channel_modes)],
        )
    }
    
    /// ERR_NONICKNAMEGIVEN
    pub fn no_nickname_given() -> Message {
        Self::ErrNoNicknameGiven.reply(
            "*",
            vec!["No nickname given".to_string()],
        )
    }
    
    /// ERR_ERRONEUSNICKNAME
    pub fn erroneous_nickname(nick: &str) -> Message {
        Self::ErrErroneousNickname.reply(
            nick,
            vec!["Erroneous nickname".to_string()],
        )
    }
    
    /// ERR_NICKNAMEINUSE
    pub fn nickname_in_use(nick: &str) -> Message {
        Self::ErrNicknameInUse.reply(
            nick,
            vec!["Nickname is already in use".to_string()],
        )
    }
    
    /// ERR_NOTREGISTERED
    pub fn not_registered() -> Message {
        Self::ErrNotRegistered.reply(
            "*",
            vec!["You have not registered".to_string()],
        )
    }
    
    /// ERR_NORECIPIENT
    pub fn no_recipients(command: &str) -> Message {
        Self::ErrNoRecipients.reply(
            "*",
            vec![format!("No recipient given ({})", command)],
        )
    }
    
    /// ERR_NOTEXTTOSEND
    pub fn no_text_to_send() -> Message {
        Self::ErrNoTextToSend.reply(
            "*",
            vec!["No text to send".to_string()],
        )
    }
    
    /// ERR_NOSUCHNICK
    pub fn no_such_nick(nick: &str) -> Message {
        Self::ErrNoSuchNick.reply(
            "*",
            vec![format!("No such nick/channel: {}", nick)],
        )
    }
    
    /// ERR_NEEDMOREPARAMS
    pub fn need_more_params(command: &str) -> Message {
        Self::ErrNeedMoreParams.reply(
            "*",
            vec![format!("Not enough parameters"), command.to_string()],
        )
    }
    
    /// ERR_ALREADYREGISTERED
    pub fn already_registered() -> Message {
        Self::ErrAlreadyRegistered.reply(
            "*",
            vec!["You may not reregister".to_string()],
        )
    }
    
    /// ERR_PASSWORDMISMATCH
    pub fn password_mismatch() -> Message {
        Self::ErrPasswordMismatch.reply(
            "*",
            vec!["Password incorrect".to_string()],
        )
    }
    
    // Server query replies
    
    /// RPL_ADMINME
    pub fn admin_me(server: &str) -> Message {
        Self::RplAdminMe.reply(
            "*",
            vec![format!("Administrative info for {}", server)],
        )
    }
    
    /// RPL_ADMINLOC1
    pub fn admin_loc1(location: &str) -> Message {
        Self::RplAdminLoc1.reply(
            "*",
            vec![location.to_string()],
        )
    }
    
    /// RPL_ADMINLOC2
    pub fn admin_loc2(location: &str) -> Message {
        Self::RplAdminLoc2.reply(
            "*",
            vec![location.to_string()],
        )
    }
    
    /// RPL_ADMINEMAIL
    pub fn admin_email(email: &str) -> Message {
        Self::RplAdminEmail.reply(
            "*",
            vec![email.to_string()],
        )
    }
    
    /// RPL_VERSION
    pub fn version(server: &str, version: &str, debug_level: &str, server_name: &str, comments: &str) -> Message {
        Self::RplVersion.reply(
            "*",
            vec![
                server.to_string(),
                version.to_string(),
                debug_level.to_string(),
                server_name.to_string(),
                comments.to_string(),
            ],
        )
    }
    
    /// RPL_TIME
    pub fn time(server: &str, time: &str) -> Message {
        Self::RplTime.reply(
            "*",
            vec![server.to_string(), time.to_string()],
        )
    }
    
    /// RPL_INFO
    pub fn info(text: &str) -> Message {
        Self::RplInfo.reply(
            "*",
            vec![text.to_string()],
        )
    }
    
    /// RPL_ENDOFINFO
    pub fn end_of_info() -> Message {
        Self::RplEndOfInfo.reply(
            "*",
            vec!["End of INFO list".to_string()],
        )
    }
    
    /// RPL_LINKS
    pub fn links(mask: &str, server: &str, hopcount: u32, server_info: &str) -> Message {
        Self::RplLinks.reply(
            "*",
            vec![
                mask.to_string(),
                server.to_string(),
                hopcount.to_string(),
                server_info.to_string(),
            ],
        )
    }
    
    /// RPL_ENDOFLINKS
    pub fn end_of_links(mask: &str) -> Message {
        Self::RplEndOfLinks.reply(
            "*",
            vec![mask.to_string(), "End of LINKS list".to_string()],
        )
    }
    
    /// RPL_STATSLINKINFO
    pub fn stats_link_info(server: &str, sendq: u32, sent_messages: u32, sent_bytes: u32, received_messages: u32, received_bytes: u32, time_online: u32) -> Message {
        Self::RplStatsLinkInfo.reply(
            "*",
            vec![
                server.to_string(),
                sendq.to_string(),
                sent_messages.to_string(),
                sent_bytes.to_string(),
                received_messages.to_string(),
                received_bytes.to_string(),
                time_online.to_string(),
            ],
        )
    }
    
    /// RPL_STATSCOMMANDS
    pub fn stats_commands(command: &str, count: u32, bytes: u32, remote_count: u32) -> Message {
        Self::RplStatsCommands.reply(
            "*",
            vec![
                command.to_string(),
                count.to_string(),
                bytes.to_string(),
                remote_count.to_string(),
            ],
        )
    }
    
    /// RPL_ENDOFSTATS
    pub fn end_of_stats(letter: &str) -> Message {
        Self::RplEndOfStats.reply(
            "*",
            vec![letter.to_string(), "End of STATS report".to_string()],
        )
    }
    
    /// RPL_TRACEUSER
    pub fn trace_user(class: &str, client: &str) -> Message {
        Self::RplTraceUser.reply(
            "*",
            vec![class.to_string(), client.to_string()],
        )
    }
    
    /// RPL_TRACESERVER
    pub fn trace_server(class: &str, server: &str, version: &str, debug_level: &str, server_name: &str) -> Message {
        Self::RplTraceServer.reply(
            "*",
            vec![
                class.to_string(),
                server.to_string(),
                version.to_string(),
                debug_level.to_string(),
                server_name.to_string(),
            ],
        )
    }
    
    /// RPL_TRACEEND
    pub fn trace_end(server: &str, version: &str) -> Message {
        Self::RplTraceEnd.reply(
            "*",
            vec![server.to_string(), version.to_string(), "End of TRACE".to_string()],
        )
    }
    
    // User query replies
    
    /// RPL_WHOREPLY
    pub fn who_reply(channel: &str, username: &str, host: &str, server: &str, nick: &str, flags: &str, hopcount: &str, realname: &str) -> Message {
        Self::RplWhoReply.reply(
            "*",
            vec![
                channel.to_string(),
                username.to_string(),
                host.to_string(),
                server.to_string(),
                nick.to_string(),
                flags.to_string(),
                hopcount.to_string(),
                realname.to_string(),
            ],
        )
    }
    
    /// RPL_ENDOFWHO
    pub fn end_of_who(name: &str) -> Message {
        Self::RplEndOfWho.reply(
            "*",
            vec![name.to_string(), "End of WHO list".to_string()],
        )
    }
    
    /// RPL_WHOISUSER
    pub fn whois_user(nick: &str, username: &str, host: &str, realname: &str) -> Message {
        Self::RplWhoisUser.reply(
            "*",
            vec![
                nick.to_string(),
                username.to_string(),
                host.to_string(),
                "*".to_string(),
                realname.to_string(),
            ],
        )
    }
    
    /// RPL_WHOISSERVER
    pub fn whois_server(nick: &str, server: &str, server_info: &str) -> Message {
        Self::RplWhoisServer.reply(
            "*",
            vec![
                nick.to_string(),
                server.to_string(),
                server_info.to_string(),
            ],
        )
    }
    
    /// RPL_CONNECTSUCCESS
    pub fn connect_success(server: &str, port: u16) -> Message {
        Self::Custom(200).reply(
            "*",
            vec![format!("Connection to {}:{} successful", server, port)],
        )
    }
    
    /// RPL_CONNECTFAILED
    pub fn connect_failed(server: &str, error: &str) -> Message {
        Self::Custom(201).reply(
            "*",
            vec![format!("Connection to {} failed: {}", server, error)],
        )
    }
    
    /// RPL_WHOISOPERATOR
    pub fn whois_operator(nick: &str) -> Message {
        Self::RplWhoisOperator.reply(
            "*",
            vec![nick.to_string(), "is an IRC operator".to_string()],
        )
    }
    
    /// RPL_WHOISIDLE
    pub fn whois_idle(nick: &str, signon_time: &str, idle_time: &str) -> Message {
        Self::RplWhoisIdle.reply(
            "*",
            vec![
                nick.to_string(),
                idle_time.to_string(),
                signon_time.to_string(),
                "seconds idle, signon time".to_string(),
            ],
        )
    }
    
    /// RPL_ENDOFWHOIS
    pub fn end_of_whois(nick: &str) -> Message {
        Self::RplEndOfWhois.reply(
            "*",
            vec![nick.to_string(), "End of WHOIS list".to_string()],
        )
    }
    
    /// RPL_WHOISCHANNELS
    pub fn whois_channels(nick: &str, channels: &str) -> Message {
        Self::RplWhoisChannels.reply(
            "*",
            vec![nick.to_string(), channels.to_string()],
        )
    }
    
    /// RPL_WHOWASUSER
    pub fn whowas_user(nick: &str, username: &str, host: &str, realname: &str) -> Message {
        Self::RplWhoisUser.reply( // Reuse WHOISUSER numeric
            "*",
            vec![
                nick.to_string(),
                username.to_string(),
                host.to_string(),
                "*".to_string(),
                realname.to_string(),
            ],
        )
    }
    
    /// RPL_ENDOFWHOWAS
    pub fn end_of_whowas(nick: &str) -> Message {
        Self::RplEndOfWhoWas.reply(
            "*",
            vec![nick.to_string(), "End of WHOWAS list".to_string()],
        )
    }
    
    // Bot mode replies
    
    /// RPL_WHOISBOT
    pub fn whois_bot(nick: &str, bot_name: &str, description: &str) -> Message {
        Self::RplWhoisSpecial.reply(
            "*",
            vec![nick.to_string(), format!("is a bot named {}: {}", bot_name, description)],
        )
    }
    
    /// RPL_BOTINFO
    pub fn bot_info(nick: &str, version: &str, capabilities: &str) -> Message {
        Self::RplWhoisSpecial.reply(
            "*",
            vec![nick.to_string(), format!("Bot version: {} | Capabilities: {}", version, capabilities)],
        )
    }
    
    // AWAY command replies
    
    /// RPL_AWAY
    pub fn away(nick: &str, away_message: &str) -> Message {
        Self::RplAway.reply(
            "*",
            vec![nick.to_string(), away_message.to_string()],
        )
    }
    
    /// RPL_UNAWAY
    pub fn unaway() -> Message {
        Self::RplUnaway.reply(
            "*",
            vec!["You are no longer marked as being away".to_string()],
        )
    }
    
    /// RPL_NOWAWAY
    pub fn now_away() -> Message {
        Self::RplNowAway.reply(
            "*",
            vec!["You have been marked as being away".to_string()],
        )
    }
    
    // ISON command replies
    
    /// RPL_ISON
    pub fn ison(nicks: &[String]) -> Message {
        Self::RplIson.reply(
            "*",
            vec![format!(":{}", nicks.join(" "))],
        )
    }
    
    // USERHOST command replies
    
    /// RPL_USERHOST
    pub fn userhost(entries: &[String]) -> Message {
        Self::RplUserhost.reply(
            "*",
            vec![format!(":{}", entries.join(" "))],
        )
    }

        /// ERR_NOPRIVILEGES
        pub fn no_privileges() -> Message {
            Self::ErrNoPrivileges.reply(
                "*",
                vec!["Permission Denied- You're not an IRC operator".to_string()],
            )
        }

        /// RPL_YOUREOPER
        pub fn youre_oper() -> Message {
            Self::RplYoureOper.reply(
                "*",
                vec!["You are now an IRC operator".to_string()],
            )
        }

}
