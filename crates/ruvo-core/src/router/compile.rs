use super::{to_matchit_path, Router};
use crate::handler::{wrap_errors, BoxFuture, ErrorHandlerFn, FallibleHandler, Handler};
use crate::middleware::{chain_from_entries, MwEntry};
use crate::raw::RawHandler;
use crate::request::{percent_decode, Request};
use crate::response::Response;
use crate::state::{MatchedMeta, TypeMap};
use http::{HeaderValue, Method};
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

struct InnerRouter {
    table: matchit::Router<HashMap<Method, Handler>>,
    not_found: Handler,
}

impl InnerRouter {
    async fn dispatch(&self, mut req: Request) -> Response {
        let path = req.path.clone();
        let method = req.method.clone();

        let matched = match self.table.at(&path) {
            Ok(m) => m,
            Err(_) => return (self.not_found)(req).await,
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

        let mut res = Response::text("Method Not Allowed").status(405);
        if let Ok(v) = HeaderValue::from_str(&allow_header(methods.keys())) {
            res.headers.insert(http::header::ALLOW, v);
        }
        res
    }
}

pub(crate) fn compile_router(router: Router) -> crate::error::Result<CompiledRouter> {
    let Router {
        routes,
        raw_routes,
        middleware: root_mw,
        mut state,
        not_found,
        error_handler,
    } = router;

    let mut entries = Vec::with_capacity(routes.len() + raw_routes.len());
    for r in &routes {
        entries.push(super::RouteEntry::Http {
            method: r.method.clone(),
            path: r.path.clone(),
            meta: r.meta.clone(),
        });
    }
    for r in &raw_routes {
        entries.push(super::RouteEntry::Raw {
            path: r.path.clone(),
        });
    }
    state.insert(super::RouteTable(entries));

    let eh = error_handler.clone();
    type MethodMap = HashMap<Method, (Vec<MwEntry>, FallibleHandler, TypeMap)>;
    let mut by_path: HashMap<String, MethodMap> = HashMap::new();

    for route in routes {
        by_path.entry(route.path.clone()).or_default().insert(
            route.method.clone(),
            (route.middleware, route.handler, route.meta),
        );
    }

    let mut table = matchit::Router::<HashMap<Method, Handler>>::new();

    for (path, methods) in by_path {
        let matchit_path = to_matchit_path(&path);
        let mut map = HashMap::new();
        for (method, (mw, fallible, meta)) in methods {
            let leaf = wrap_errors(with_matched_meta(fallible, meta), eh.clone());
            map.insert(method, chain_from_entries(&mw, leaf));
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

    let not_found: Handler = match not_found {
        Some(h) => wrap_errors(h, eh.clone()),
        None => Arc::new(|_req: Request| {
            Box::pin(async { Response::text("Not Found").status(404) }) as BoxFuture<Response>
        }),
    };

    let inner = Arc::new(InnerRouter { table, not_found });
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

/// Inject [`MatchedMeta`] so handlers/plugins can read typed route metadata.
fn with_matched_meta(handler: FallibleHandler, meta: TypeMap) -> FallibleHandler {
    Arc::new(move |mut req| {
        let handler = Arc::clone(&handler);
        let meta = meta.clone();
        Box::pin(async move {
            req.set(MatchedMeta(meta));
            handler(req).await
        })
    })
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
