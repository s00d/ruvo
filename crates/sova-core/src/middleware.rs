use crate::handler::{BoxFuture, Handler};
use crate::request::Request;
use crate::response::Response;
use std::future::Future;
use std::sync::Arc;

/// Call the rest of the middleware / handler chain.
pub type Next = Box<dyn FnOnce(Request) -> BoxFuture<Response> + Send>;

/// Type-erased middleware: `(Request, Next) -> Response`.
pub type Middleware = Arc<dyn Fn(Request, Next) -> BoxFuture<Response> + Send + Sync>;

/// Named middleware entry used for [`crate::Router::explain`].
#[derive(Clone)]
pub struct MwEntry {
    pub name: String,
    pub mw: Middleware,
}

/// Wrap middleware with an explicit explain label (e.g. `named("auth", …)`).
pub fn named(name: impl Into<String>, mw: impl IntoMiddleware) -> MwEntry {
    MwEntry {
        name: name.into(),
        mw: mw.into_middleware(),
    }
}

pub(crate) fn abbreviate_type_name(full: &str) -> String {
    let trimmed = full
        .trim_end_matches(">::{{closure}}")
        .trim_end_matches("::{{closure}}")
        .trim_end_matches("{{closure}}");
    let base = trimmed.rsplit("::").next().unwrap_or(trimmed);
    if base.is_empty() || base == "{{closure}}" {
        "closure".into()
    } else {
        base.to_string()
    }
}

pub trait IntoMiddleware {
    fn into_middleware(self) -> Middleware;
}

/// Convert into a named [`MwEntry`] (explicit name via [`named`], else type name).
pub trait IntoMwEntry {
    fn into_mw_entry(self) -> MwEntry;
}

impl<F, Fut> IntoMiddleware for F
where
    F: Fn(Request, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn into_middleware(self) -> Middleware {
        Arc::new(move |req, next| Box::pin(self(req, next)))
    }
}

impl IntoMiddleware for Middleware {
    fn into_middleware(self) -> Middleware {
        self
    }
}

impl IntoMwEntry for MwEntry {
    fn into_mw_entry(self) -> MwEntry {
        self
    }
}

impl IntoMwEntry for Middleware {
    fn into_mw_entry(self) -> MwEntry {
        MwEntry {
            name: "mw".into(),
            mw: self,
        }
    }
}

impl<F, Fut> IntoMwEntry for F
where
    F: Fn(Request, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn into_mw_entry(self) -> MwEntry {
        MwEntry {
            name: abbreviate_type_name(std::any::type_name::<F>()),
            mw: self.into_middleware(),
        }
    }
}

/// Middleware with owned state — hides the `Arc::clone` dance.
///
/// Prefer this over capturing many fields and cloning them into each future.
/// If a closure needs more than one `.clone()`, the state should be one `Arc`
/// (this helper) or process-lifetime `&'static` via [`with_leaked`].
pub fn with_state<S, F, Fut>(state: S, f: F) -> Middleware
where
    S: Send + Sync + 'static,
    F: Fn(Arc<S>, Request, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    let state = Arc::new(state);
    Arc::new(move |req, next| Box::pin(f(Arc::clone(&state), req, next)))
}

/// Immutable plugin config that lives for the process.
///
/// # ponytail
/// One-shot `Box::leak` at install — no Arc atomics on the hot path.
pub fn with_leaked<S, F, Fut>(state: S, f: F) -> Middleware
where
    S: Send + Sync + 'static,
    F: Fn(&'static S, Request, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    let state: &'static S = Box::leak(Box::new(state));
    Arc::new(move |req, next| Box::pin(f(state, req, next)))
}

/// Run `f` on the request before the rest of the chain.
pub fn before<F, Fut>(name: impl Into<String>, f: F) -> MwEntry
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Request> + Send + 'static,
{
    let f = Arc::new(f);
    named(name, move |req, next: Next| {
        let f = Arc::clone(&f);
        async move {
            let req = f(req).await;
            next(req).await
        }
    })
}

/// Run `f` on the response after the rest of the chain.
pub fn after<F, Fut>(name: impl Into<String>, f: F) -> MwEntry
where
    F: Fn(Response) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    let f = Arc::new(f);
    named(name, move |req, next: Next| {
        let f = Arc::clone(&f);
        async move {
            let res = next(req).await;
            f(res).await
        }
    })
}

/// `before` then chain then `after` under one explain name.
pub fn around<B, BF, A, AF>(name: impl Into<String>, before_fn: B, after_fn: A) -> MwEntry
where
    B: Fn(Request) -> BF + Send + Sync + 'static,
    BF: Future<Output = Request> + Send + 'static,
    A: Fn(Response) -> AF + Send + Sync + 'static,
    AF: Future<Output = Response> + Send + 'static,
{
    let before_fn = Arc::new(before_fn);
    let after_fn = Arc::new(after_fn);
    named(name, move |req, next: Next| {
        let before_fn = Arc::clone(&before_fn);
        let after_fn = Arc::clone(&after_fn);
        async move {
            let req = before_fn(req).await;
            let res = next(req).await;
            after_fn(res).await
        }
    })
}

/// After the handler: map buffered `text/html` bodies with `transform`.
///
/// Non-HTML / streamed responses are unchanged.
pub fn map_html<F>(name: impl Into<String>, transform: F) -> MwEntry
where
    F: Fn(&str) -> Option<String> + Send + Sync + 'static,
{
    let transform = Arc::new(transform);
    after(name, move |mut res| {
        let transform = Arc::clone(&transform);
        async move {
            res.map_buffered_html(|html| transform(html));
            res
        }
    })
}

/// Build Express-style onion once; returns a reusable [`Handler`].
pub fn build_chain(middleware: &[Middleware], handler: Handler) -> Handler {
    let mut next = handler;

    for mw in middleware.iter().rev() {
        let mw = Arc::clone(mw);
        let inner = next;
        next = Arc::new(move |req| {
            let inner = Arc::clone(&inner);
            let mw = Arc::clone(&mw);
            mw(
                req,
                Box::new(move |r| {
                    let inner = Arc::clone(&inner);
                    inner(r)
                }),
            )
        });
    }

    next
}

pub(crate) fn chain_from_entries(entries: &[MwEntry], handler: Handler) -> Handler {
    let mws: Vec<Middleware> = entries.iter().map(|e| Arc::clone(&e.mw)).collect();
    build_chain(&mws, handler)
}

/// Paths registered via [`logger_skip_path`] / [`logger_skip_paths`] are not logged
/// (useful for health checks and `/_devtools/*`).
pub fn logger() -> MwEntry {
    named("logger", |req: Request, next: Next| async move {
        let method = req.method.as_str().to_string();
        let path = req.path.clone();
        let quiet = logger_should_skip(&path);
        let request_id = req
            .get::<crate::request_id::RequestId>()
            .map(|r| r.0.clone())
            .unwrap_or_default();
        let start = std::time::Instant::now();
        let res = next(req).await;
        if quiet {
            return res;
        }
        let status = res.status_code().as_u16();
        let latency_ms = start.elapsed().as_millis() as u64;
        if request_id.is_empty() {
            tracing::info!(
                method = %method,
                path = %path,
                status,
                latency_ms,
                "request"
            );
        } else {
            tracing::info!(
                request_id = %request_id,
                method = %method,
                path = %path,
                status,
                latency_ms,
                "request"
            );
        }
        res
    })
}

static LOGGER_SKIP_PREFIXES: std::sync::OnceLock<std::sync::Mutex<Vec<String>>> =
    std::sync::OnceLock::new();

fn logger_skip_list() -> &'static std::sync::Mutex<Vec<String>> {
    LOGGER_SKIP_PREFIXES.get_or_init(|| {
        // Common browser noise — apps can still log these by not matching exact paths.
        std::sync::Mutex::new(vec!["/favicon.ico".into()])
    })
}

/// Skip access-log lines for paths that equal or start with `prefix`
/// (e.g. `"/_devtools"` matches `/_devtools/config`).
pub fn logger_skip_path(prefix: impl Into<String>) {
    let p = prefix.into();
    if p.is_empty() {
        return;
    }
    let mut g = logger_skip_list().lock().unwrap();
    if !g.iter().any(|x| x == &p) {
        g.push(p);
    }
}

/// Register several skip prefixes (see [`logger_skip_path`]).
pub fn logger_skip_paths(prefixes: impl IntoIterator<Item = impl Into<String>>) {
    for p in prefixes {
        logger_skip_path(p);
    }
}

/// Whether [`logger`] (and quiet [`crate::request_id`] spans) should skip this path.
pub fn logger_should_skip(path: &str) -> bool {
    let g = logger_skip_list().lock().unwrap();
    g.iter()
        .any(|p| path == p.as_str() || path.starts_with(&format!("{p}/")))
}

#[cfg(any(test, feature = "testing"))]
pub fn logger_clear_skip_paths() {
    *logger_skip_list().lock().unwrap() = vec!["/favicon.ico".into()];
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;

    fn ok_handler() -> Handler {
        Arc::new(|_req: Request| Box::pin(async { Response::text("ok") }))
    }

    #[tokio::test]
    async fn onion_order() {
        let leaf = ok_handler();
        let outer = |req: Request, next: Next| async move {
            let mut res = next(req).await;
            res = res.header("x-outer", "1");
            res
        };
        let inner = |req: Request, next: Next| async move {
            let mut res = next(req).await;
            res = res.header("x-inner", "1");
            res
        };
        let chain = build_chain(
            &[outer.into_middleware(), inner.into_middleware()],
            leaf,
        );
        let res = chain(Request::new(Method::GET, "/")).await;
        assert_eq!(res.body_bytes(), Some(b"ok".as_slice()));
        assert_eq!(res.headers.get("x-outer").map(|v| v.to_str().unwrap()), Some("1"));
        assert_eq!(res.headers.get("x-inner").map(|v| v.to_str().unwrap()), Some("1"));
    }

    #[tokio::test]
    async fn with_state_runs() {
        let leaf = ok_handler();
        let mw = with_state(7u32, |n, req, next| async move {
            assert_eq!(*n, 7);
            next(req).await
        });
        let chain = build_chain(&[mw], leaf);
        assert_eq!(
            chain(Request::new(Method::GET, "/")).await.body_bytes(),
            Some(b"ok".as_slice())
        );
    }

    #[test]
    fn logger_skip_favicon_by_default() {
        logger_clear_skip_paths();
        assert!(logger_should_skip("/favicon.ico"));
        assert!(!logger_should_skip("/favicon"));
        logger_clear_skip_paths();
    }

    #[test]
    fn logger_skip_matches_prefixes() {
        logger_clear_skip_paths();
        logger_skip_path("/_devtools");
        logger_skip_path("/healthz");
        assert!(logger_should_skip("/_devtools"));
        assert!(logger_should_skip("/_devtools/config"));
        assert!(logger_should_skip("/_devtools/requests/dt-1"));
        assert!(logger_should_skip("/healthz"));
        assert!(!logger_should_skip("/api/users"));
        assert!(!logger_should_skip("/health"));
        logger_clear_skip_paths();
    }

    #[test]
    fn abbreviate_closure() {
        let n = abbreviate_type_name("foo::bar::{{closure}}");
        assert_eq!(n, "bar");
    }
}
