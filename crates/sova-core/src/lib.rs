//! Sova core — App, Router, Request, Response, middleware, server.
//!
//! # Request path
//!
//! `accept` → `server/conn` → `to_sova_request` → `CompiledRouter::dispatch`
//! → root middleware → matchit → route middleware → handler → [`IntoResponse`]
//! → hyper response.
//!
//! # Public surface
//!
//! This crate root exports names used by applications. Plugin authors and
//! advanced integrations use [`extend`].
//!
//! Product docs: <https://s00d.github.io/sova/> (VitePress guide `/guide/concepts`).

mod request_id;
mod app;
mod config;
mod error;
mod handler;
mod human;
mod limits;
mod middleware;
mod plugin;
mod raw;
mod request;
mod response;
mod route_value;
mod router;
mod server;
mod service;
mod share;
mod state;
mod tracing_init;
mod upgrade;
#[cfg(any(test, feature = "testing"))]
mod test_client;
#[cfg(feature = "tls")]
mod tls;

// Application-facing API (~16–18 names).
pub use app::{App, BoundApp, CheckKind, CheckResult, Http, Server};
pub use config::ConfigDoc;
pub use error::{Error, IntoResponse, Result};
pub use middleware::{logger, with_state, Next};
pub use plugin::{
    check_plugin_sdk, InstalledPlugin, Plugin, PluginMeta, PluginSdkVersion, SdkCompat,
    PLUGIN_SDK_VERSION,
};
pub use request::{FormData, Request, Upload, UploadRules};
pub use request_id::{ensure_request_id, request_id, RequestId};
pub use response::{referer_or, Html, Json, NoContent, Redirect, Response, Text};
pub use router::Router;
pub use server::{ClientAddr, RateLimitIdentity};
pub use service::{BackgroundService, Shutdown};
pub use share::{Cell, Slot};
pub use state::{MatchedRoute, MatchedRouteCapture};
pub use tracing_init::{
    parse_log_rotate, set_log_event_hook, LogConfig, LogEventHook, LogRecord, LogRotate,
};
#[cfg(any(test, feature = "testing"))]
pub use service::{shutdown_channel, ShutdownSender};
pub use upgrade::{OnUpgrade, UpgradePermit};
#[cfg(any(test, feature = "testing"))]
pub use test_client::{ClientRequest, RequestHook, ResponseAssert, TestClient};
#[cfg(feature = "tls")]
pub use tls::Tls;

/// Extension / plugin-author API (handlers, bodies, route introspection, …).
///
/// Prefer this module when writing plugins: middleware helpers (`with_leaked`,
/// `named`), [`PluginMeta`](crate::PluginMeta) / [`PLUGIN_SDK_VERSION`](crate::PLUGIN_SDK_VERSION),
/// and route introspection types. Application `main` usually imports from the crate root
/// (or the `sova` facade) instead.
pub mod extend {
    pub use crate::app::Bind;
    pub use crate::plugin::{
        check_plugin_sdk, InstalledPlugin, PluginMeta, PluginSdkVersion, SdkCompat,
        PLUGIN_SDK_VERSION,
    };
    pub use crate::handler::{
        BoxFuture, ErrorResponse, FallibleHandler, Handler, IntoHandler,
    };
    pub use crate::middleware::{
        named, with_leaked, IntoMiddleware, IntoMwEntry, Middleware, MwEntry,
    };
    pub use crate::raw::{IntoRawHandler, RawHandler};
    pub use crate::request::{FormData, RequestBuilder, Upload, UploadRules};
    pub use crate::request_id::{ensure_request_id, request_id, RequestId};
    pub use crate::response::{Body, BoxError, HttpBody, ResponseBody};
    pub use crate::human::{parse_bytes, parse_duration};
    pub use crate::limits::{tighten_deadline, Deadline, MaxBody, RequestTimeout};
    pub use crate::route_value::{BuildCtx, MetaMap, Needs, RouteValue};
    pub use crate::router::{
        join_paths, normalize_path, to_brace_path, RouteEntry, RouteTable,
    };
    pub use crate::service::wait_shutdown;
    pub use crate::share::{Cell, Slot};
    pub use crate::state::{
        Extensions, MatchedMeta, MatchedMetaCapture, MatchedRoute, MatchedRouteCapture, StateMap,
        TypeMap,
    };
    pub use crate::tracing_init::{
        ensure_tracing, parse_log_rotate, set_log_event_hook, LogConfig, LogEventHook, LogRecord,
        LogRotate,
    };
}
