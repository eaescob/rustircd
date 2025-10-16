//! External authentication modules
//! 
//! This module provides various external authentication providers
//! that can be used with the SASL system.

pub mod ldap;
pub mod database;
pub mod file;
pub mod http;
pub mod supabase;

pub use ldap::LdapAuthProvider;
pub use database::DatabaseAuthProvider;
pub use file::FileAuthProvider;
pub use http::HttpAuthProvider;
pub use supabase::{SupabaseAuthProvider, SupabaseAuthConfig, SupabaseAuthProviderBuilder};
