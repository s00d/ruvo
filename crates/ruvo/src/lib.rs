//! Ruvo — Express-like HTTP framework for Rust.
//!
//! Thin facade over `ruvo-core` plus optional plugin crates.
//! Application names live at the crate root; plugin authors use [`extend`].

pub use ruvo_core::{
    logger, with_state, App, ClientAddr, Error, Html, IntoResponse, Json, NoContent, Next, Plugin,
    Redirect, Request, Response, Result, Router, Server, Text,
};

/// Extension API (handlers, bodies, route table, …) — see `ruvo_core::extend`.
pub mod extend {
    pub use ruvo_core::extend::*;
}

#[cfg(feature = "cors")]
pub use ruvo_cors::Cors;

#[cfg(feature = "cookies")]
pub use ruvo_cookies::{CookieBuilder, CookieLayer, CookieLayerPresent, Cookies, ResponseCookieExt};

#[cfg(feature = "static-files")]
pub use ruvo_static::Static;

#[cfg(feature = "compress")]
pub use ruvo_compress::Compress;

#[cfg(feature = "rate-limit")]
pub use ruvo_rate_limit::RateLimit;

#[cfg(feature = "session")]
pub use ruvo_session::{
    memory_sessions, MemoryStore, NullStore, Session, SessionExt, SessionLayer, SessionStore,
};

#[cfg(feature = "templates")]
pub use ruvo_templates::{MiniJinjaEngine, TemplateEngine};

#[cfg(feature = "multipart")]
pub use ruvo_multipart::{MultipartExt, MultipartField};

#[cfg(feature = "cli")]
pub use ruvo_cli::{ListenArgs, Parser, ServerArgs};

#[cfg(feature = "vld")]
pub use ruvo_vld::{ValidationError, ValidationExt};

#[cfg(feature = "vld")]
pub use vld;

#[cfg(feature = "openapi")]
pub use ruvo_openapi::{undocumented, Doc, OpenApi, OpenApiDocExt};

#[cfg(feature = "i18n")]
pub use ruvo_i18n::{
    mount_localized, I18n, I18nExt, I18nRouteExt, I18nScope, Locale, PrefixMode, ROOT_SCOPE,
};

#[cfg(feature = "vld-openapi")]
pub use ruvo_vld::{doc_schema, DocVldExt, VldDocSchema};

/// Install a default `tracing` subscriber (`RUST_LOG`, default `ruvo=info`).
/// Call once from `main` — never from inside the library `listen` path.
/// With the `cli` feature, prefer `ServerArgs::init_tracing` when using `--log-level`.
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("ruvo=info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

#[cfg(test)]
mod tests_extra;
