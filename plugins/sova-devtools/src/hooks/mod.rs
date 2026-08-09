//! Soft-hook collectors (session / auth / mail / response meta).
//! Domain plugins emit EventBus events; DevTools is a sink — these hooks only snapshot request state.

#[cfg(any(feature = "session", feature = "auth", feature = "passport"))]
mod auth;
#[cfg(feature = "mail")]
mod mail;
mod response;

#[cfg(feature = "session")]
pub use auth::collect_session_auth;
#[cfg(all(not(feature = "session"), any(feature = "auth", feature = "passport")))]
pub use auth::fill_auth_without_session;
#[cfg(feature = "mail")]
pub use mail::collect_mail;
pub use response::collect_response_meta;
