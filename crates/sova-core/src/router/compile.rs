use super::{to_matchit_path, CatcherMap, Router};
use crate::handler::{wrap_errors, ErrorHandlerFn, FallibleHandler, Handler};
use crate::middleware::{chain_from_entries, MwEntry};
use crate::raw::RawHandler;
use crate::request::{percent_decode, Request};
use crate::response::Response;
use crate::route_value::MetaMap;
use crate::state::{Extensions, MatchedMeta, MatchedMetaCapture, MatchedRoute, MatchedRouteCapture, TypeMap};
use http::{HeaderMap, HeaderValue, Method};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Immutable compiled router used at runtime.
pub(crate) struct CompiledRouter {
    pub(crate) dispatch: Handler,
    pub(crate) raw_table: matchit::Router<RawHandler>,
    pub(crate) state: Arc<TypeMap>,
    pub(crate) error_handler: Option<ErrorHandlerFn>,
}

impl CompiledRouter {
    pub(crate) fn lookup_raw(&self, path: &str) -> Option<RawHandler> {
        self.raw_table.at(path).ok().map(|m| Arc::clone(m.value))
    }

    pub(crate) async fn dispatch(&self, mut req: Request) -> Response {
        req.state = Arc::clone(&self.state);
        (self.dispatch)(req).await
    }
}

/// Longest-prefix status catchers.
pub(crate) struct CatcherTable {
    /// Sorted by prefix length descending.
    entries: Vec<(String, HashMap<u16, Handler>)>,
}

impl CatcherTable {
    fn from_scopes(scopes: Vec<(String, CatcherMap)>, eh: Option<ErrorHandlerFn>) -> Self {
        let mut entries: Vec<(String, HashMap<u16, Handler>)> = scopes
            .into_iter()
            .map(|(prefix, map)| {
                let handlers = map
                    .into_iter()
                    .map(|(status, h)| (status, wrap_errors(h, eh.clone())))
                    .collect();
                (prefix, handlers)
            })
            .collect();
        entries.sort_by_key(|b| std::cmp::Reverse(b.0.len()));
        Self { entries }
    }

    fn find(&self, path: &str, status: u16) -> Option<&Handler> {
        for (prefix, map) in &self.entries {
            if prefix_matches(path, prefix) {
                if let Some(h) = map.get(&status) {
                    return Some(h);
                }
            }
        }
        None
    }
}

fn prefix_matches(path: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

struct InnerRouter {
    table: matchit::Router<HashMap<Method, Handler>>,
    catchers: Arc<CatcherTable>,
}

impl InnerRouter {
    async fn dispatch(&self, mut req: Request) -> Response {
        let path = req.path.clone();
        let method = req.method.clone();

        let matched = match self.table.at(&path) {
            Ok(m) => m,
            Err(_) => return invoke_catcher(&self.catchers, req, 404).await,
        };

        let mut params = HashMap::new();
        for (k, v) in matched.params.iter() {
            params.insert(k.to_string(), percent_decode(v));
        }
        req.params = params;

        let methods = matched.value;

        if method == Method::OPTIONS {
            return allow_response(methods.keys());
        }

        if method == Method::HEAD {
            if let Some(handler) = methods.get(&Method::GET) {
                let mut res = handler(req).await;
                res.clear_body();
                return res;
            }
        }

        if let Some(handler) = methods.get(&method) {
            return handler(req).await;
        }

        if self.catchers.find(&path, 405).is_some() {
            return invoke_catcher(&self.catchers, req, 405).await;
        }

        let mut res = Response::text("Method Not Allowed").status(405);
        if let Ok(v) = HeaderValue::from_str(&allow_header(methods.keys())) {
            res.headers.insert(http::header::ALLOW, v);
        }
        res
    }
}

async fn invoke_catcher(catchers: &CatcherTable, req: Request, status: u16) -> Response {
    match catchers.find(&req.path, status) {
        Some(h) => h(req).await.status(status),
        None => default_status_response(status),
    }
}

fn default_status_response(status: u16) -> Response {
    match status {
        404 => Response::text("Not Found").status(404),
        405 => Response::text("Method Not Allowed").status(405),
        _ => Response::text(format!("Error {status}")).status(status),
    }
}

pub(crate) fn compile_router(router: Router) -> crate::error::Result<CompiledRouter> {
    let Router {
        routes,
        raw_routes,
        middleware: root_mw,
        mut state,
        defaults,
        last_was_route: _,
        catchers,
        scoped_catchers,
        error_handler,
    } = router;

    let mut entries = Vec::with_capacity(routes.len() + raw_routes.len());
    for r in &routes {
        let mut meta = defaults.clone();
        meta.extend(r.meta.clone());
        entries.push(super::RouteEntry::Http {
            method: r.method.clone(),
            path: r.path.clone(),
            meta,
        });
    }
    for r in &raw_routes {
        entries.push(super::RouteEntry::Raw {
            path: r.path.clone(),
        });
    }
    state.insert(super::RouteTable(entries));

    let eh = error_handler.clone();

    let mut scopes = scoped_catchers;
    if !catchers.is_empty() {
        scopes.push((String::new(), catchers));
    }
    let catcher_table = Arc::new(CatcherTable::from_scopes(scopes, eh.clone()));

    type MethodMap = HashMap<Method, (Vec<MwEntry>, FallibleHandler, MetaMap)>;
    let mut by_path: HashMap<String, MethodMap> = HashMap::new();

    for route in routes {
        let mut meta = defaults.clone();
        meta.extend(route.meta);
        by_path.entry(route.path.clone()).or_default().insert(
            route.method.clone(),
            (route.middleware, route.handler, meta),
        );
    }

    let mut table = matchit::Router::<HashMap<Method, Handler>>::new();

    for (path, methods) in by_path {
        let matchit_path = to_matchit_path(&path);
        let mut map = HashMap::new();
        for (method, (mw, fallible, meta)) in methods {
            // MatchedMeta / MaxBody before route middleware so meta-driven mw (e.g. vld) works.
            let leaf = wrap_with_catchers(fallible, eh.clone(), Arc::clone(&catcher_table));
            let with_mw = chain_from_entries(&mw, leaf);
            map.insert(method, inject_matched_meta(with_mw, meta, path.clone()));
        }
        table.insert(matchit_path.clone(), map).map_err(|err| {
            crate::error::Error::Internal(format!(
                "route conflict for {path} ({matchit_path}): {err}"
            ))
        })?;
    }

    let mut raw_table = matchit::Router::<RawHandler>::new();
    for raw in raw_routes {
        let matchit_path = to_matchit_path(&raw.path);
        raw_table
            .insert(matchit_path.clone(), raw.handler)
            .map_err(|err| {
                crate::error::Error::Internal(format!(
                    "raw route conflict for {} ({matchit_path}): {err}",
                    raw.path
                ))
            })?;
    }

    let inner = Arc::new(InnerRouter {
        table,
        catchers: Arc::clone(&catcher_table),
    });
    let inner_dispatch: Handler = Arc::new(move |req| {
        let inner = Arc::clone(&inner);
        Box::pin(async move { inner.dispatch(req).await })
    });

    let dispatch = chain_from_entries(&root_mw, inner_dispatch);

    Ok(CompiledRouter {
        dispatch,
        raw_table,
        state: Arc::new(state),
        error_handler,
    })
}

/// Inject [`MatchedMeta`] / [`MatchedRoute`] (and apply MaxBody / RequestTimeout)
/// **before** the rest of the handler chain so route middleware can read meta.
fn inject_matched_meta(handler: Handler, meta: MetaMap, route_path: String) -> Handler {
    Arc::new(move |mut req| {
        let handler = Arc::clone(&handler);
        let meta = meta.clone();
        let route_path = route_path.clone();
        Box::pin(async move {
            if let Some(max) = meta.get::<crate::limits::MaxBody>() {
                req.body_limit = max.0;
            }
            let timeout = meta.get::<crate::limits::RequestTimeout>().map(|t| t.0);
            req.set(MatchedMeta(meta.clone()));
            req.set(MatchedRoute(route_path.clone()));
            if let Some(cap) = req.get::<MatchedMetaCapture>() {
                cap.set(meta);
            }
            if let Some(cap) = req.get::<MatchedRouteCapture>() {
                cap.set(route_path);
            }
            match timeout {
                Some(dur) => {
                    crate::limits::tighten_deadline(
                        &mut req,
                        std::time::Instant::now() + dur,
                    );
                    match tokio::time::timeout(dur, handler(req)).await {
                        Ok(res) => res,
                        Err(_) => Response::text("Request Timeout").status(504),
                    }
                }
                None => handler(req).await,
            }
        })
    })
}

/// Run `error_handler` on `Err`, then replace the response via a status catcher
/// when one is registered for the final status (request body already consumed).
fn wrap_with_catchers(
    handler: FallibleHandler,
    eh: Option<ErrorHandlerFn>,
    catchers: Arc<CatcherTable>,
) -> Handler {
    Arc::new(move |req| {
        let handler = Arc::clone(&handler);
        let eh = eh.clone();
        let catchers = Arc::clone(&catchers);
        Box::pin(async move {
            let snap = CatcherSnap::from_request(&req);
            let res = match handler(req).await {
                Ok(res) => res,
                Err(crate::error::Error::Response(res)) => *res,
                Err(err) => match &eh {
                    Some(hook) => hook(err).await,
                    None => err.into_response(),
                },
            };
            let status = res.status_code().as_u16();
            match catchers.find(&snap.path, status) {
                Some(catcher) => catcher(snap.into_request()).await.status(status),
                None => res,
            }
        })
    })
}

/// Snapshot of request fields needed to invoke a catcher after the leaf ran.
struct CatcherSnap {
    method: Method,
    path: String,
    headers: HeaderMap,
    query: HashMap<String, String>,
    scheme: String,
    host: String,
    raw_query: String,
    body_limit: usize,
    state: Arc<TypeMap>,
}

impl CatcherSnap {
    fn from_request(req: &Request) -> Self {
        Self {
            method: req.method.clone(),
            path: req.path.clone(),
            headers: req.headers.clone(),
            query: req.query.clone(),
            scheme: req.scheme.clone(),
            host: req.host.clone(),
            raw_query: req.raw_query.clone(),
            body_limit: req.body_limit(),
            state: Arc::clone(&req.state),
        }
    }

    fn into_request(self) -> Request {
        Request {
            method: self.method,
            path: self.path,
            headers: self.headers,
            params: HashMap::new(),
            query: self.query,
            scheme: self.scheme,
            host: self.host,
            raw_query: self.raw_query,
            body: crate::request::ReqBody::Taken { by: "catcher" },
            body_limit: self.body_limit,
            state: self.state,
            extensions: Extensions::new(),
        }
    }
}

fn allow_header<'a>(methods: impl Iterator<Item = &'a Method>) -> String {
    let mut set: HashSet<String> = methods.map(|m| m.to_string()).collect();
    set.insert(Method::OPTIONS.to_string());
    if set.contains(Method::GET.as_str()) {
        set.insert(Method::HEAD.to_string());
    }
    let mut list: Vec<_> = set.into_iter().collect();
    list.sort();
    list.join(", ")
}

fn allow_response<'a>(methods: impl Iterator<Item = &'a Method>) -> Response {
    Response::empty()
        .status(204)
        .header("allow", allow_header(methods))
}
