use crate::app::AppInner;
use crate::response::{Response, ResponseBody};
use bytes::Bytes;
use futures_util::FutureExt;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::sync::watch;

use super::convert::{to_hyper_response, to_sova_request};

pub(super) async fn serve_http1<I>(
    inner: Arc<AppInner>,
    io: TokioIo<I>,
    peer: SocketAddr,
    mut conn_shutdown: watch::Receiver<bool>,
) where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let header_timeout = inner.conn_header_timeout();
    let keep_alive = inner.keep_alive;
    let max_headers = inner.max_headers;
    let max_buf_size = inner.max_buf_size;
    let max_concurrent_streams = inner.max_concurrent_streams.min(u32::MAX as usize) as u32;

    let service = service_fn(move |req| {
        let inner = Arc::clone(&inner);
        async move { Ok::<_, Infallible>(handle_hyper(inner, req, peer).await) }
    });

    let mut builder = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
    builder
        .http1()
        .timer(TokioTimer::new())
        .keep_alive(keep_alive)
        .header_read_timeout(header_timeout)
        .max_headers(max_headers);
    if let Some(max) = max_buf_size {
        builder.http1().max_buf_size(max);
    }

    // HTTP/2 hardening: limit concurrency, tune flow-control windows,
    // and mitigate Rapid Reset style attacks.
    builder
        .http2()
        .max_concurrent_streams(Some(max_concurrent_streams))
        .adaptive_window(true)
        .initial_stream_window_size(Some(1024 * 1024))
        .initial_connection_window_size(Some(1024 * 1024))
        .max_local_error_reset_streams(Some(1024));

    let conn = builder.serve_connection_with_upgrades(io, service);
    tokio::pin!(conn);

    tokio::select! {
        res = &mut conn => {
            if let Err(err) = res {
                tracing::debug!("connection error: {err}");
            }
        }
        _ = conn_shutdown.changed() => {
            if *conn_shutdown.borrow() {
                conn.as_mut().graceful_shutdown();
                let _ = conn.await;
            }
        }
    }
}

pub(super) async fn handle_hyper(
    app: Arc<AppInner>,
    req: HyperRequest<Incoming>,
    peer: SocketAddr,
) -> HyperResponse<ResponseBody> {
    use crate::limits::{tighten_deadline, Deadline};
    use std::time::Instant;

    let is_http2 = req.version() == http::Version::HTTP_2;
    let timeout = app.request_timeout;
    let app_deadline = timeout.map(|d| Instant::now() + d);
    let fut = async {
        if app.compiled.has_raw {
            let path = req.uri().path();
            if let Some(raw) = app.compiled.lookup_raw(path) {
                let fut = AssertUnwindSafe(raw(req));
                return match fut.catch_unwind().await {
                    Ok(res) => res,
                    Err(_) => {
                        tracing::error!("raw handler panicked");
                        HyperResponse::builder()
                            .status(500)
                            .body(
                                Full::new(Bytes::from_static(b"Internal Server Error"))
                                    .map_err(|_: Infallible| unreachable!())
                                    .boxed(),
                            )
                            .expect("fallback")
                    }
                };
            }
        }

        let fut = AssertUnwindSafe(async {
            match to_sova_request(req, &app, peer).await {
                Ok(mut r) => {
                    if let Some(until) = app_deadline {
                        // Seed deadline; route RequestTimeout may tighten further.
                        if r.get::<Deadline>().is_none() {
                            r.set(Deadline(until));
                        } else {
                            tighten_deadline(&mut r, until);
                        }
                    }
                    app.handle(r).await
                }
                Err(err) => match &app.compiled.error_handler {
                    Some(handler) => handler(err).await,
                    None => err.into_response(),
                },
            }
        });

        let res = match fut.catch_unwind().await {
            Ok(res) => res,
            Err(_) => {
                tracing::error!("handler panicked");
                Response::text("Internal Server Error").status(500)
            }
        };

        let mut res = res;
        if is_http2
            && (res.headers.contains_key(http::header::CONNECTION)
                || res.headers.contains_key(http::header::TRANSFER_ENCODING)
                || res
                    .headers
                    .contains_key(http::HeaderName::from_static("keep-alive"))
                || res.headers.contains_key(http::header::UPGRADE))
        {
            // Special-case: `426 Upgrade Required` is our explicit, user-facing
            // websocket-over-h2 refusal. HTTP/2 forbids hop-by-hop `Upgrade`,
            // so strip it instead of turning the response into a 500.
            if res.status == http::StatusCode::UPGRADE_REQUIRED {
                res.headers.remove(http::header::CONNECTION);
                res.headers.remove(http::header::TRANSFER_ENCODING);
                res.headers
                    .remove(http::HeaderName::from_static("keep-alive"));
                res.headers.remove(http::header::UPGRADE);
            } else {
                res = Response::text("Internal Server Error").status(500);
            }
        }

        to_hyper_response(res, app.hsts, app.alt_svc.as_deref())
    };

    match timeout {
        Some(d) => match tokio::time::timeout(d, fut).await {
            Ok(res) => res,
            Err(_) => HyperResponse::builder()
                .status(408)
                .body(
                    Full::new(Bytes::from_static(b"Request Timeout"))
                        .map_err(|_: Infallible| unreachable!())
                        .boxed(),
                )
                .expect("timeout response"),
        },
        None => fut.await,
    }
}
