//! Ruvo core — App, Router, Request, Response, middleware, server.
//!
//! # Request path
//!
//! `accept` → `server/conn` → `to_ruvo_request` → `CompiledRouter::dispatch`
//! → root middleware → matchit → route middleware → handler → [`IntoResponse`]
//! → hyper response.
//!
//! # Public surface
//!
//! This crate root exports names used by applications. Plugin authors and
//! advanced integrations use [`extend`].
//!
//! See also the repo-root `ARCHITECTURE.md`.

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
mod state;
mod tracing_init;
mod upgrade;
mod test_client;
#[cfg(feature = "tls")]
mod tls;

// Application-facing API (~16–18 names).
pub use app::{App, BoundApp, Http, Server};
pub use config::ConfigDoc;
pub use error::{Error, IntoResponse, Result};
pub use middleware::{logger, with_state, Next};
pub use plugin::Plugin;
pub use request::Request;
pub use response::{Html, Json, NoContent, Redirect, Response, Text};
pub use router::Router;
pub use server::ClientAddr;
pub use service::{BackgroundService, Shutdown};
#[cfg(any(test, feature = "testing"))]
pub use service::{shutdown_channel, ShutdownSender};
pub use upgrade::{OnUpgrade, UpgradePermit};
pub use test_client::{ClientRequest, TestClient};
#[cfg(feature = "tls")]
pub use tls::Tls;

/// Extension / plugin-author API (handlers, bodies, route introspection, …).
pub mod extend {
    pub use crate::app::Bind;
    pub use crate::handler::{
        BoxFuture, ErrorResponse, FallibleHandler, Handler, IntoHandler,
    };
    pub use crate::middleware::{
        named, with_leaked, IntoMiddleware, IntoMwEntry, Middleware, MwEntry,
    };
    pub use crate::raw::{IntoRawHandler, RawHandler};
    pub use crate::request::RequestBuilder;
    pub use crate::response::{Body, BoxError, HttpBody, ResponseBody};
    pub use crate::human::{parse_bytes, parse_duration};
    pub use crate::limits::{tighten_deadline, Deadline, MaxBody, RequestTimeout};
    pub use crate::route_value::{BuildCtx, MetaMap, Needs, RouteValue};
    pub use crate::router::{
        join_paths, normalize_path, to_brace_path, RouteEntry, RouteTable,
    };
    pub use crate::service::wait_shutdown;
    pub use crate::state::{Extensions, MatchedMeta, StateMap, TypeMap};
    pub use crate::tracing_init::ensure_tracing;
}
