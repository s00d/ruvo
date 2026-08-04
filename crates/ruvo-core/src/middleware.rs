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

/// Request logger (`method path → status`).
pub fn logger() -> Middleware {
    Arc::new(|req, next| {
        Box::pin(async move {
            let method = req.method.clone();
            let path = req.path.clone();
            let res = next(req).await;
            tracing::info!("{method} {path} → {}", res.status_code());
            res
        })
    })
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
    fn abbreviate_closure() {
        let n = abbreviate_type_name("foo::bar::{{closure}}");
        assert_eq!(n, "bar");
    }
}
