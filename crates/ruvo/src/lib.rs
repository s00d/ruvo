//! Ruvo — Express-like HTTP framework for Rust.
//!
//! Thin facade over `ruvo-core` plus optional plugin crates.
//! Application names live at the crate root; plugin authors use [`extend`].

pub use ruvo_core::{
    logger, with_state, App, BackgroundService, Bind, BoundApp, ClientAddr, Error, Html, Http,
    IntoResponse, Json, NoContent, Next, OnUpgrade, Plugin, Redirect, Request, Response, Result,
    Router, Server, Shutdown, Text,
};
#[cfg(feature = "tls")]
pub use ruvo_core::Tls;

#[cfg(feature = "env")]
pub use ruvo_env::{self, require as env_require, EnvError};

#[cfg(feature = "store-crypto")]
pub use ruvo_store::{encrypted, encrypted_ns, AppKey, Encrypted};

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
pub use ruvo_session::{memory_sessions, Session, SessionExt, SessionLayer};

#[cfg(feature = "store")]
pub use ruvo_store::{namespace, KvStore, MemoryStore as KvMemoryStore, Namespace};

#[cfg(feature = "store-file")]
pub use ruvo_store_file::{Durability, FileStore as KvFileStore};

#[cfg(feature = "tasks-store")]
pub use ruvo_tasks_store::{
    EnqueueOpts, MemoryStore as TaskMemoryStore, Task, TaskError, TaskStatus, TaskStore,
};

#[cfg(feature = "tasks-file")]
pub use ruvo_tasks_file::FileTaskStore;

#[cfg(feature = "tasks")]
pub use ruvo_tasks::{bearer_guard, TaskBackend, Tasks};

#[cfg(feature = "udp")]
pub use ruvo_udp::UdpService;

#[cfg(feature = "quic-udp")]
pub use ruvo_quic::{QuicDatagramClient, QuicDatagramService};

#[cfg(feature = "sse-feed")]
pub use ruvo_sse::{sse_response, SseChannel, SseEvent};

#[cfg(feature = "templates")]
pub use ruvo_templates::{MiniJinjaEngine, MiniJinjaTemplates, RenderExt, Templates, TemplateEngine};

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
    mount_localized, template_fn, I18n, I18nExt, I18nRouteExt, I18nScope, Locale, PrefixMode,
    ROOT_SCOPE,
};

#[cfg(feature = "ws")]
pub use ruvo_ws::{
    origin_allowed, upgrade_ws, Hub, Message, RoomHandle, Ws, WsRouteExt, WsSession,
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
