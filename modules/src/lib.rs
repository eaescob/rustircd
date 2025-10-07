//! Rust IRC Daemon Modules
//! 
//! This crate provides modules and extensions for the IRC daemon.

pub mod channel;
pub mod ircv3;
pub mod messaging;
pub mod optional;
pub mod throttling;
pub mod help;
pub mod monitor;
pub mod knock;
pub mod set;
pub mod gline;
pub mod kline;
pub mod dline;
pub mod xline;
pub mod admin;
pub mod testing;
pub mod services;
pub mod oper;
pub mod sasl;
pub mod opme;

pub use channel::{ChannelModule, Channel, ChannelMember, ChannelMode};
pub use ircv3::Ircv3Module;
pub use messaging::{MessagingModule, MessagingManager, WallopsModule, MessagingWrapper, create_default_messaging_module};
pub use optional::OptionalModule;
pub use throttling::ThrottlingModule;
pub use help::{HelpModule, HelpProvider, HelpTopic};
pub use monitor::MonitorModule;
pub use knock::{KnockModule, KnockConfig, KnockRequest};
pub use set::{SetModule, SettingValue, SettingType, SettingMetadata};
pub use gline::{GlineModule, GlineConfig, GlobalBan as GlineGlobalBan};
pub use kline::{KlineModule, KlineConfig, KillLine as KlineKillLine};
pub use dline::{DlineModule, DlineConfig, DnsLine as DlineDnsLine};
pub use xline::{XlineModule, XlineConfig, ExtendedLine as XlineExtendedLine};
pub use admin::{AdminModule, AdminInfo, AdminWallMessage};
pub use testing::{TestingModule, TestConfig, TestResult, TestLineResult, TestStatistics};
pub use services::{ServicesModule, ServiceConfig, Service, ServiceType, ServiceStatistics};
pub use oper::{OperModule, OperConfig, OperatorAware, DefaultOperatorAware, OperatorChecker, OperatorAction};
pub use sasl::{SaslModule, SaslConfig, SaslSession, SaslAuthData, SaslState, SaslMechanism, SaslResponse, SaslResponseType, SaslCapabilityExtension};
pub use opme::{OpmeModule, OpmeConfig, OpmeRateLimit, OpmeStats, OpmeConfigBuilder};
