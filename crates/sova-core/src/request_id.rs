//! Per-request correlation id (`x-request-id`).

use crate::middleware::{named, MwEntry, Next};
use crate::request::Request;
use tracing::Instrument;

tokio::task_local! {
    static CURRENT_REQUEST_ID: String;
}

/// Request id for the current async task (set by [`request_id`] middleware).
///
/// Plugins (store / redis / tasks) can attach this to tracing events so DevTools
/// correlates them with the open request bag.
pub fn current_request_id() -> Option<String> {
    CURRENT_REQUEST_ID.try_with(|s| s.clone()).ok()
}

/// Per-request correlation id (inbound `x-request-id` or generated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(pub String);

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for RequestId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Ensure [`RequestId`], echo `x-request-id`, wrap in an `http.server` span.
///
/// Quiet paths ([`crate::logger_should_skip`]) still get a request id, but use a
/// `debug` span so `/_devtools/*` / favicon do not flood the console.
pub fn request_id() -> MwEntry {
    named("request-id", |mut req: Request, next: Next| async move {
        ensure_request_id(&mut req);
        let id = req
            .get::<RequestId>()
            .map(|r| r.0.clone())
            .unwrap_or_default();
        let method = req.method.as_str().to_string();
        let path = req.path.clone();
        let quiet = crate::middleware::logger_should_skip(&path);
        let id_for_span = id.clone();
        let id_for_header = id.clone();

        let run = CURRENT_REQUEST_ID.scope(id, async move {
            let mut res = next(req).await;
            if !id_for_header.is_empty() {
                res = res.header("x-request-id", &id_for_header);
            }
            res
        });

        if quiet {
            let span = tracing::debug_span!(
                "http.server",
                request_id = %id_for_span,
                method = %method,
                path = %path,
                otel.kind = "server",
            );
            run.instrument(span).await
        } else {
            let span = tracing::info_span!(
                "http.server",
                request_id = %id_for_span,
                method = %method,
                path = %path,
                otel.kind = "server",
            );
            run.instrument(span).await
        }
    })
}

/// Set [`RequestId`] from `x-request-id` or generate one (idempotent).
pub fn ensure_request_id(req: &mut Request) {
    if req.get::<RequestId>().is_some() {
        return;
    }
    let id = req
        .header("x-request-id")
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(generate_request_id);
    req.set(RequestId(id));
}

fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut bytes = [0u8; 8];
    let _ = getrandom::getrandom(&mut bytes);
    let entropy = u64::from_le_bytes(bytes);
    format!("req-{entropy:016x}-{n:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::build_chain;
    use crate::response::Response;
    use http::Method;
    use std::sync::Arc;

    #[tokio::test]
    async fn echoes_and_reuses_inbound() {
        let leaf: crate::handler::Handler = Arc::new(|req: Request| {
            Box::pin(async move {
                let id = req.get::<RequestId>().unwrap().0.clone();
                Response::text(id)
            })
        });
        let chain = build_chain(&[request_id().mw], leaf);
        let mut req = Request::new(Method::GET, "/");
        req.headers
            .insert("x-request-id", "abc-123".parse().unwrap());
        let res = chain(req).await;
        assert_eq!(res.body_bytes(), Some(b"abc-123".as_slice()));
        assert_eq!(
            res.headers
                .get("x-request-id")
                .and_then(|v| v.to_str().ok()),
            Some("abc-123")
        );
    }

    #[tokio::test]
    async fn generates_when_missing() {
        let leaf: crate::handler::Handler = Arc::new(|req: Request| {
            Box::pin(async move {
                let id = req.get::<RequestId>().unwrap().0.clone();
                Response::text(id)
            })
        });
        let chain = build_chain(&[request_id().mw.clone()], leaf);
        let res = chain(Request::new(Method::GET, "/")).await;
        let body = String::from_utf8(res.body_bytes().unwrap().to_vec()).unwrap();
        assert!(body.starts_with("req-"), "{body}");
        assert_eq!(
            res.headers
                .get("x-request-id")
                .and_then(|v| v.to_str().ok()),
            Some(body.as_str())
        );
    }
}
