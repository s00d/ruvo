use crate::app::{AppInner, ShutdownHook, StartupHook};
use crate::error::{Error, Result};
use crate::request::{parse_query, Request};
use crate::response::{Response, ResponseBody};
use crate::state::Extensions;
use bytes::Bytes;
use futures_util::FutureExt;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::{TokioIo, TokioTimer};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;

use super::forwarded::forwarded_addr;
use super::{ClientAddr, ExternalShutdown};

pub(super) async fn run_tcp(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    listener: TcpListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    let state = inner.state();
    for hook in startups {
        hook(Arc::clone(&state)).await?;
    }

    let local = listener
        .local_addr()
        .map_err(|e| Error::Internal(format!("local_addr: {e}")))?;
    log_startup_banner(&inner, &format!("http://{local}"));

    let drain_timeout = inner.drain_timeout;
    let inner = Arc::new(inner);
    let conn_limit = Arc::new(Semaphore::new(inner.max_connections));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    spawn_shutdown_watchers(shutdown_tx, external_shutdown);

    let mut tasks: JoinSet<()> = JoinSet::new();
    let mut shutdown_rx = shutdown_rx;

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("stopping accept loop");
                    break;
                }
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted
                    .map_err(|e| Error::Internal(format!("accept: {e}")))?;

                let permit = match conn_limit.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("max connections reached, rejecting {peer}");
                        drop(stream);
                        continue;
                    }
                };

                let io = TokioIo::new(stream);
                let inner = Arc::clone(&inner);
                let conn_shutdown = shutdown_rx.clone();

                tasks.spawn(async move {
                    let _permit = permit;
                    serve_http1(inner, io, peer, conn_shutdown).await;
                });
            }
        }
    }

    drain_tasks(&mut tasks, drain_timeout).await;
    for hook in shutdowns {
        hook().await;
    }
    Ok(())
}

#[cfg(unix)]
pub(super) async fn run_unix(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    listener: tokio::net::UnixListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    let state = inner.state();
    for hook in startups {
        hook(Arc::clone(&state)).await?;
    }

    log_startup_banner(&inner, "unix");

    let drain_timeout = inner.drain_timeout;
    let inner = Arc::new(inner);
    let conn_limit = Arc::new(Semaphore::new(inner.max_connections));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    spawn_shutdown_watchers(shutdown_tx, external_shutdown);

    let mut tasks: JoinSet<()> = JoinSet::new();
    let mut shutdown_rx = shutdown_rx;
    // UDS has no TCP peer — rate-limit / ClientAddr see unspecified.
    let peer = SocketAddr::from(([0, 0, 0, 0], 0));

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("stopping accept loop");
                    break;
                }
            }
            accepted = listener.accept() => {
                let (stream, _addr) = accepted
                    .map_err(|e| Error::Internal(format!("accept: {e}")))?;

                let permit = match conn_limit.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        tracing::warn!("max connections reached, rejecting uds client");
                        drop(stream);
                        continue;
                    }
                };

                let io = TokioIo::new(stream);
                let inner = Arc::clone(&inner);
                let conn_shutdown = shutdown_rx.clone();

                tasks.spawn(async move {
                    let _permit = permit;
                    serve_http1(inner, io, peer, conn_shutdown).await;
                });
            }
        }
    }

    drain_tasks(&mut tasks, drain_timeout).await;
    for hook in shutdowns {
        hook().await;
    }
    Ok(())
}

fn spawn_shutdown_watchers(
    shutdown_tx: watch::Sender<bool>,
    external_shutdown: Option<ExternalShutdown>,
) {
    let tx_signal = shutdown_tx.clone();
    tokio::spawn(async move {
        wait_shutdown_signal().await;
        tracing::info!("shutdown signal received");
        let _ = tx_signal.send(true);
    });

    if let Some(fut) = external_shutdown {
        tokio::spawn(async move {
            fut.await;
            tracing::info!("programmatic shutdown");
            let _ = shutdown_tx.send(true);
        });
    }
}

fn log_startup_banner(inner: &AppInner, addr: &str) {
    tracing::info!(
        addr = %addr,
        routes = inner.route_count,
        "ruvo listening"
    );
    #[cfg(debug_assertions)]
    tracing::debug!("routes:\n{}", inner.explain.trim_end());
}

async fn drain_tasks(tasks: &mut JoinSet<()>, drain_timeout: Duration) {
    let drain = async {
        while tasks.join_next().await.is_some() {}
    };
    match tokio::time::timeout(drain_timeout, drain).await {
        Ok(()) => tracing::info!("connections drained"),
        Err(_) => {
            tracing::warn!("drain timeout; aborting remaining connections");
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        }
    }
}

async fn serve_http1<I>(
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

    let service = service_fn(move |req| {
        let inner = Arc::clone(&inner);
        async move { Ok::<_, Infallible>(handle_hyper(inner, req, peer).await) }
    });

    let mut builder = hyper::server::conn::http1::Builder::new();
    builder
        .timer(TokioTimer::new())
        .keep_alive(keep_alive)
        .header_read_timeout(header_timeout)
        .max_headers(max_headers);
    if let Some(max) = max_buf_size {
        builder.max_buf_size(max);
    }

    let conn = builder.serve_connection(io, service);
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

async fn wait_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn handle_hyper(
    app: Arc<AppInner>,
    req: HyperRequest<Incoming>,
    peer: SocketAddr,
) -> HyperResponse<ResponseBody> {
    let timeout = app.request_timeout;
    let fut = async {
        let path = req.uri().path().to_string();
        if let Some(raw) = app.compiled.lookup_raw(&path) {
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

        let fut = AssertUnwindSafe(async {
            match to_ruvo_request(req, app.state(), app.max_body_size, peer, app.trust_proxy).await
            {
                Ok(r) => app.handle(r).await,
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

        to_hyper_response(res)
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

async fn to_ruvo_request(
    req: HyperRequest<Incoming>,
    state: Arc<crate::state::StateMap>,
    max_body: usize,
    peer: SocketAddr,
    trust_proxy: bool,
) -> Result<Request> {
    use crate::request::{resolve_scheme_host, ReqBody};
    use http_body_util::BodyExt;

    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let raw_query = uri.query().unwrap_or("").to_string();
    let query = if raw_query.is_empty() {
        Default::default()
    } else {
        parse_query(&raw_query)
    };
    let headers = req.headers().clone();

    if let Some(cl) = headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
    {
        if cl > max_body {
            return Err(Error::PayloadTooLarge);
        }
    }

    let (scheme, host) = resolve_scheme_host(&headers, uri.scheme_str(), trust_proxy);

    // Lazy body: keep as stream until text/json/form/collect.
    let incoming = req.into_body();
    let stream = incoming
        .map_err(|e| -> crate::response::BoxError { Box::new(e) })
        .boxed();

    let client = if trust_proxy {
        forwarded_addr(&headers).unwrap_or(peer)
    } else {
        peer
    };

    let mut extensions = Extensions::new();
    extensions.insert(ClientAddr(client));

    Ok(Request {
        method,
        path,
        headers,
        params: Default::default(),
        query,
        scheme,
        host,
        raw_query,
        body: ReqBody::Stream(stream),
        body_limit: max_body,
        state,
        extensions,
    })
}

fn to_hyper_response(res: Response) -> HyperResponse<ResponseBody> {
    let status = res.status;
    let headers = res.headers.clone();
    let body = res.into_http_body();
    let mut builder = HyperResponse::builder().status(status);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    builder.body(body).unwrap_or_else(|_| {
        HyperResponse::builder()
            .status(500)
            .body(
                Full::new(Bytes::from_static(b"internal error"))
                    .map_err(|_: Infallible| unreachable!())
                    .boxed(),
            )
            .expect("fallback")
    })
}
