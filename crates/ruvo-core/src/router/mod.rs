mod compile;
mod path;

pub use path::{join_paths, normalize_path, to_brace_path};
pub(crate) use compile::{compile_router, CompiledRouter};
pub(crate) use path::to_matchit_path;

use path::normalize_prefix;

use crate::handler::{ErrorHandlerFn, FallibleHandler, IntoHandler};
use crate::middleware::{IntoMwEntry, MwEntry};
use crate::raw::{IntoRawHandler, RawHandler};
use crate::response::Response;
use crate::state::TypeMap;
use http::Method;
use std::sync::Arc;

struct RouteDef {
    method: Method,
    /// Express-style path (`/users/:id`, `/assets/*path`).
    path: String,
    middleware: Vec<MwEntry>,
    handler: FallibleHandler,
    meta: TypeMap,
}

struct RawDef {
    path: String,
    handler: RawHandler,
}

/// Registered route for introspection (OpenAPI, metrics, debug).
#[derive(Clone)]
pub enum RouteEntry {
    Http {
        method: Method,
        path: String,
        /// Typed metadata bag (one value per `TypeId`; last insert wins).
        meta: TypeMap,
    },
    Raw {
        path: String,
    },
}

/// Snapshot of all routes, inserted into app state at compile time.
#[derive(Clone)]
pub struct RouteTable(pub Vec<RouteEntry>);

/// Express-style router. Return from modules and `app.mount("/blog", routes())`.
pub struct Router {
    routes: Vec<RouteDef>,
    raw_routes: Vec<RawDef>,
    middleware: Vec<MwEntry>,
    pub(crate) state: TypeMap,
    not_found: Option<FallibleHandler>,
    error_handler: Option<ErrorHandlerFn>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            raw_routes: Vec::new(),
            middleware: Vec::new(),
            state: TypeMap::new(),
            not_found: None,
            error_handler: None,
        }
    }

    /// Deep-enough clone for test dispatch without consuming the live router.
    pub(crate) fn clone_for_compile(&self) -> Router {
        Router {
            routes: self
                .routes
                .iter()
                .map(|r| RouteDef {
                    method: r.method.clone(),
                    path: r.path.clone(),
                    middleware: r.middleware.clone(),
                    handler: Arc::clone(&r.handler),
                    meta: r.meta.clone(),
                })
                .collect(),
            raw_routes: self
                .raw_routes
                .iter()
                .map(|r| RawDef {
                    path: r.path.clone(),
                    handler: Arc::clone(&r.handler),
                })
                .collect(),
            middleware: self.middleware.clone(),
            state: self.state.clone_map(),
            not_found: self.not_found.as_ref().map(Arc::clone),
            error_handler: self.error_handler.as_ref().map(Arc::clone),
        }
    }

    pub fn use_middleware<M>(&mut self, mw: M) -> &mut Self
    where
        M: IntoMwEntry,
    {
        self.middleware.push(mw.into_mw_entry());
        self
    }

    /// Attach typed metadata to the last registered HTTP route.
    ///
    /// Same `T` twice keeps the **last** value. Different types never conflict.
    pub fn route_meta<T>(&mut self, value: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        if let Some(r) = self.routes.last_mut() {
            r.meta.insert(value);
        }
        self
    }

    pub fn state<T>(&mut self, value: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.state.insert(value);
        self
    }

    pub fn get<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.add(Method::GET, path, handler)
    }

    pub fn post<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.add(Method::POST, path, handler)
    }

    pub fn put<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.add(Method::PUT, path, handler)
    }

    pub fn patch<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.add(Method::PATCH, path, handler)
    }

    pub fn delete<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.add(Method::DELETE, path, handler)
    }

    /// Escape hatch: handle a path with a raw Hyper request/response (no Ruvo middleware).
    pub fn raw<H>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: IntoRawHandler,
    {
        self.raw_routes.push(RawDef {
            path: normalize_path(path),
            handler: handler.into_raw_handler(),
        });
        self
    }

    /// Mount a child router under `prefix`.
    ///
    /// Bakes the child's middleware stack into its routes. Does **not** prepend
    /// this router's middleware — the eventual root stack is applied once in
    /// `compile_router` as an outer wrap (so App-level middleware is not doubled).
    pub fn mount(&mut self, prefix: &str, other: Router) -> &mut Self {
        let prefix = normalize_prefix(prefix);
        let child_mw = other.middleware;

        for mut route in other.routes {
            route.path = join_paths(&prefix, &route.path);
            let mut mw = child_mw.clone();
            mw.extend(route.middleware);
            route.middleware = mw;
            self.routes.push(route);
        }

        for mut raw in other.raw_routes {
            raw.path = join_paths(&prefix, &raw.path);
            self.raw_routes.push(raw);
        }

        self.state.extend(other.state);

        if self.not_found.is_none() {
            self.not_found = other.not_found;
        }
        if self.error_handler.is_none() {
            self.error_handler = other.error_handler;
        }

        self
    }

    /// Sugar over [`Self::mount`]: build a child router in a closure.
    pub fn group<F>(&mut self, prefix: &str, f: F) -> &mut Self
    where
        F: FnOnce(&mut Router),
    {
        let mut child = Router::new();
        f(&mut child);
        self.mount(prefix, child)
    }

    pub fn not_found<H, T>(&mut self, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.not_found = Some(handler.into_handler());
        self
    }

    /// Called when a leaf handler returns `Err`. Request is already consumed.
    pub fn error_handler<F, Fut>(&mut self, f: F) -> &mut Self
    where
        F: Fn(crate::error::Error) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Response> + Send + 'static,
    {
        self.error_handler = Some(Arc::new(move |err| Box::pin(f(err))));
        self
    }

    /// Full introspection: HTTP routes and raw paths.
    pub fn route_entries(&self) -> Vec<RouteEntry> {
        let mut out = Vec::new();
        for r in &self.routes {
            out.push(RouteEntry::Http {
                method: r.method.clone(),
                path: r.path.clone(),
                meta: r.meta.clone(),
            });
        }
        for r in &self.raw_routes {
            out.push(RouteEntry::Raw {
                path: r.path.clone(),
            });
        }
        out
    }

    /// Human-readable route map (method, path, middleware names).
    pub fn explain(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "root_middleware: [{}]", format_mw_names(&self.middleware));
        let _ = writeln!(out, "not_found: {}", self.not_found.is_some());
        let _ = writeln!(out, "error_handler: {}", self.error_handler.is_some());
        for r in &self.routes {
            let _ = writeln!(
                out,
                "{} {} mw=[{}]",
                r.method,
                r.path,
                format_mw_names(&r.middleware)
            );
        }
        for r in &self.raw_routes {
            let _ = writeln!(out, "RAW {} mw=[]", r.path);
        }
        out
    }

    fn add<H, T>(&mut self, method: Method, path: &str, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.routes.push(RouteDef {
            method,
            path: normalize_path(path),
            // Module stack is baked at `mount`, not here — keeps root mw outer-only.
            middleware: Vec::new(),
            handler: handler.into_handler(),
            meta: TypeMap::new(),
        });
        self
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

fn format_mw_names(entries: &[MwEntry]) -> String {
    entries
        .iter()
        .map(|e| e.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
