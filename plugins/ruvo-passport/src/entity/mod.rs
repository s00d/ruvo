//! SeaORM entities for auth tables.

pub mod refresh_token;
pub mod user;

#[cfg(feature = "oauth")]
pub mod oauth_account;
