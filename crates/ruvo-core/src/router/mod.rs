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
use crate::route_value::{MetaMap, RouteValue};
use crate::state::TypeMap;
use http::Method;
use std::collections::HashMap;
use std::sync::Arc;

struct RouteDef {
    method: Method,
    /// Express-style path (`/users/:id`, `/assets/*path`).
    path: String,
    middleware: Vec<MwEntry>,
    handler: FallibleHandler,
    meta: MetaMap,
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
        meta: MetaMap,
    },
    Raw {
        path: String,
    },
}

/// Snapshot of all routes, inserted into app state at compile time.
#[derive(Clone)]
pub struct RouteTable(pub Vec<RouteEntry>);

/// Catchers registered on one router, later scoped by mount prefix.
pub(crate) type CatcherMap = HashMap<u16, FallibleHandler>;

/// Express-style router. Return from modules and `app.mount("/blog", routes())`.
pub struct Router {
    routes: Vec<RouteDef>,
    raw_routes: Vec<RawDef>,
    middleware: Vec<MwEntry>,
    pub(crate) state: TypeMap,
    /// Router/app-level [`RouteValue`] defaults (overridden per route).
    pub(crate) defaults: MetaMap,
    /// After `get`/`post`/…, [`Self::with`] writes to the last route.
    last_was_route: bool,
    /// Status → handler for this router's mount prefix (empty = app root).
    catchers: CatcherMap,
    /// Catchers collected from mounted children: `(prefix, status → handler)`.
    scoped_catchers: Vec<(String, CatcherMap)>,
    error_handler: Option<ErrorHandlerFn>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            raw_routes: Vec::new(),
            middleware: Vec::new(),
            state: TypeMap::new(),
            defaults: MetaMap::new(),
            last_was_route: false,
            catchers: HashMap::new(),
            scoped_catchers: Vec::new(),
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
            defaults: self.defaults.clone(),
            last_was_route: self.last_was_route,
            catchers: self
                .catchers
                .iter()
                .map(|(s, h)| (*s, Arc::clone(h)))
                .collect(),
            scoped_catchers: self
                .scoped_catchers
                .iter()
                .map(|(p, m)| {
                    (
                        p.clone(),
                        m.iter().map(|(s, h)| (*s, Arc::clone(h))).collect(),
                    )
                })
                .collect(),
            error_handler: self.error_handler.as_ref().map(Arc::clone),
        }
    }

    pub fn use_middleware<M>(&mut self, mw: M) -> &mut Self
    where
        M: IntoMwEntry,
    {
        self.last_was_route = false;
        self.middleware.push(mw.into_mw_entry());
        self
    }

    /// Attach a [`RouteValue`] to the last HTTP route, or to router defaults.
    ///
    /// After `get`/`post`/…, writes to that route. Otherwise writes to router/app
    /// defaults (inherited by routes: route > router > app).
    pub fn with<T: RouteValue>(&mut self, value: T) -> &mut Self {
        if self.last_was_route {
            if let Some(r) = self.routes.last_mut() {
                r.meta.insert(value);
            }
        } else {
            self.defaults.insert(value);
        }
        self
    }

    /// Update a [`RouteValue`] on the last route (insert `T::default()` if missing).
    pub fn with_update<T, F>(&mut self, f: F) -> &mut Self
    where
        T: RouteValue + Clone + Default,
        F: FnOnce(&mut T),
    {
        if let Some(r) = self.routes.last_mut() {
            let mut v = r.meta.get::<T>().map(|a| (*a).clone()).unwrap_or_default();
            f(&mut v);
            r.meta.insert(v);
            self.last_was_route = true;
        }
        self
    }

    /// Push middleware onto the last registered HTTP route only.
    pub fn route_middleware<M>(&mut self, mw: M) -> &mut Self
    where
        M: IntoMwEntry,
    {
        if let Some(r) = self.routes.last_mut() {
            let entry = mw.into_mw_entry();
            if !r.middleware.iter().any(|e| e.name == entry.name) {
                r.middleware.push(entry);
            }
        }
        self
    }

    /// Alias for [`Self::with`] (writes to the last route when present).
    pub fn route_meta<T: RouteValue>(&mut self, value: T) -> &mut Self {
        if let Some(r) = self.routes.last_mut() {
            r.meta.insert(value);
            self.last_was_route = true;
        } else {
            self.defaults.insert(value);
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

    /// Shared app state inserted via [`Self::state`], if present.
    pub fn try_state<T>(&self) -> Option<std::sync::Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.state.get::<T>()
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

    /// `GET from` → redirect to `to` with the given HTTP status (e.g. `302`, `301`, `303`).
    ///
    /// ```ignore
    /// app.redirect("/health", "/healthz", 302);
    /// app.redirect("/old", "/new", 301);
    /// ```
    pub fn redirect(&mut self, from: &str, to: impl Into<String>, status: u16) -> &mut Self {
        let location = to.into();
        self.get(from, move || {
            let location = location.clone();
            async move { crate::Redirect::with(status, location) }
        })
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
            let mut meta = other.defaults.clone();
            meta.extend(route.meta);
            route.meta = meta;
            self.routes.push(route);
        }
        self.last_was_route = false;

        for mut raw in other.raw_routes {
            raw.path = join_paths(&prefix, &raw.path);
            self.raw_routes.push(raw);
        }

        self.state.extend(other.state);

        if !other.catchers.is_empty() {
            self.scoped_catchers.push((prefix.clone(), other.catchers));
        }
        for (child_prefix, map) in other.scoped_catchers {
            self.scoped_catchers
                .push((join_paths(&prefix, &child_prefix), map));
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

    /// Register a catcher for HTTP `status` in this router's mount scope.
    ///
    /// At dispatch, the catcher with the **longest** matching prefix wins.
    /// `not_found` is sugar for `catch(404, …)`.
    pub fn catch<H, T>(&mut self, status: u16, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.last_was_route = false;
        self.catchers.insert(status, handler.into_handler());
        self
    }

    /// Sugar for [`Self::catch`]`(404, handler)`.
    pub fn not_found<H, T>(&mut self, handler: H) -> &mut Self
    where
        H: IntoHandler<T>,
    {
        self.catch(404, handler)
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
            let mut meta = self.defaults.clone();
            meta.extend(r.meta.clone());
            out.push(RouteEntry::Http {
                method: r.method.clone(),
                path: r.path.clone(),
                meta,
            });
        }
        for r in &self.raw_routes {
            out.push(RouteEntry::Raw {
                path: r.path.clone(),
            });
        }
        out
    }

    /// Run [`RouteValue::check`] for router defaults and every route.
    pub(crate) fn check_route_values(
        &self,
        state: &crate::state::StateMap,
        installed_plugins: &std::collections::HashSet<&'static str>,
    ) -> Result<(), String> {
        use crate::route_value::BuildCtx;
        let ctx = BuildCtx {
            state,
            installed_plugins,
            route_path: "<defaults>",
            route_method: None,
        };
        self.defaults.check_all(&ctx)?;
        for r in &self.routes {
            let mut meta = self.defaults.clone();
            meta.extend(r.meta.clone());
            let ctx = BuildCtx {
                state,
                installed_plugins,
                route_path: &r.path,
                route_method: Some(&r.method),
            };
            meta.check_all(&ctx)?;
        }
        Ok(())
    }

    /// Human-readable route map (method, path, middleware names).
    pub fn explain(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "root_middleware: [{}]", format_mw_names(&self.middleware));
        let _ = writeln!(out, "error_handler: {}", self.error_handler.is_some());
        let mut catch_lines = Vec::new();
        if !self.catchers.is_empty() {
            let mut codes: Vec<_> = self.catchers.keys().copied().collect();
            codes.sort_unstable();
            catch_lines.push(format!("/ → {:?}", codes));
        }
        for (prefix, map) in &self.scoped_catchers {
            let mut codes: Vec<_> = map.keys().copied().collect();
            codes.sort_unstable();
            let p = if prefix.is_empty() { "/" } else { prefix.as_str() };
            catch_lines.push(format!("{p} → {:?}", codes));
        }
        let _ = writeln!(out, "catchers: [{}]", catch_lines.join("; "));
        if !self.defaults.is_empty() {
            let _ = writeln!(out, "defaults: [{}]", self.defaults.labels().join(" "));
        }
        for r in &self.routes {
            let labels = r.meta.labels();
            if labels.is_empty() {
                let _ = writeln!(
                    out,
                    "{} {} mw=[{}]",
                    r.method,
                    r.path,
                    format_mw_names(&r.middleware)
                );
            } else {
                let _ = writeln!(
                    out,
                    "{} {} mw=[{}] {}",
                    r.method,
                    r.path,
                    format_mw_names(&r.middleware),
                    labels.join(" ")
                );
            }
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
            meta: MetaMap::new(),
        });
        self.last_was_route = true;
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
