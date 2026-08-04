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
mod error;
mod handler;
mod middleware;
mod plugin;
mod raw;
mod request;
mod response;
mod router;
mod server;
mod state;

// Application-facing API (~16–18 names).
pub use app::{App, Server};
pub use error::{Error, IntoResponse, Result};
pub use middleware::{logger, with_state, Next};
pub use plugin::Plugin;
pub use request::Request;
pub use response::{Html, Json, NoContent, Redirect, Response, Text};
pub use router::Router;
pub use server::ClientAddr;

/// Extension / plugin-author API (handlers, bodies, route introspection, …).
pub mod extend {
    pub use crate::handler::{
        BoxFuture, ErrorResponse, FallibleHandler, Handler, IntoHandler,
    };
    pub use crate::middleware::{
        named, with_leaked, IntoMiddleware, IntoMwEntry, Middleware, MwEntry,
    };
    pub use crate::raw::{IntoRawHandler, RawHandler};
    pub use crate::request::RequestBuilder;
    pub use crate::response::{Body, BoxError, HttpBody, ResponseBody};
    pub use crate::router::{
        join_paths, normalize_path, to_brace_path, RouteEntry, RouteTable,
    };
    pub use crate::state::{Extensions, MatchedMeta, StateMap, TypeMap};
}
