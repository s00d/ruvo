use crate::app::{AppInner, ShutdownHook, StartupHook};
use crate::error::{Error, Result};
use crate::request::{parse_query, Request};
use crate::response::{Response, ResponseBody};
use crate::service::BoxedService;
use crate::state::Extensions;
use crate::upgrade::PendingUpgrade;
use bytes::Bytes;
use futures_util::FutureExt;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::upgrade::OnUpgrade as HyperOnUpgrade;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;

use super::forwarded::forwarded_addr;
use super::{ClientAddr, ExternalShutdown};

enum AcceptKind {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(tokio::net::UnixListener),
}

#[cfg(not(feature = "tls"))]
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_tcp(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    listener: TcpListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    run_accept_loop(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        AcceptKind::Tcp(listener),
    )
    .await
}

#[cfg(feature = "tls")]
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_tcp(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    listener: TcpListener,
    external_shutdown: Option<ExternalShutdown>,
    tls: Option<crate::tls::TlsRuntime>,
) -> Result<()> {
    run_accept_loop(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        AcceptKind::Tcp(listener),
        tls,
    )
    .await
}

#[cfg(all(unix, not(feature = "tls")))]
pub(super) async fn run_unix(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    listener: tokio::net::UnixListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    run_accept_loop(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        AcceptKind::Unix(listener),
    )
    .await
}

#[cfg(all(unix, feature = "tls"))]
pub(super) async fn run_unix(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    listener: tokio::net::UnixListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    run_accept_loop(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        AcceptKind::Unix(listener),
        None,
    )
    .await
}

#[cfg(not(feature = "tls"))]
#[allow(clippy::too_many_arguments)]
async fn run_accept_loop(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    external_shutdown: Option<ExternalShutdown>,
    accept: AcceptKind,
) -> Result<()> {
    run_accept_loop_impl(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        accept,
        false,
    )
    .await
}

#[cfg(feature = "tls")]
#[allow(clippy::too_many_arguments)]
async fn run_accept_loop(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    external_shutdown: Option<ExternalShutdown>,
    accept: AcceptKind,
    tls: Option<crate::tls::TlsRuntime>,
) -> Result<()> {
    run_accept_loop_impl(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        accept,
        tls.is_some(),
        tls,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_accept_loop_impl(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    external_shutdown: Option<ExternalShutdown>,
    accept: AcceptKind,
    tls_enabled: bool,
    #[cfg(feature = "tls")] tls: Option<crate::tls::TlsRuntime>,
) -> Result<()> {
    let state = inner.state();
    for hook in &startups {
        hook(Arc::clone(&state)).await?;
    }

    let banner = match &accept {
        AcceptKind::Tcp(listener) => {
            let local = listener
                .local_addr()
                .map_err(|e| Error::Internal(format!("local_addr: {e}")))?;
            let scheme = if tls_enabled { "https" } else { "http" };
            format!("{scheme}://{local}")
        }
        #[cfg(unix)]
        AcceptKind::Unix(_) => "unix".into(),
    };
    log_startup_banner(&inner, &banner);

    let drain_timeout = inner.drain_timeout;
    let inner = Arc::new(inner);
    let conn_limit = Arc::new(Semaphore::new(inner.max_connections));
    let (accept_tx, accept_rx) = watch::channel(false);
    let (svc_tx, svc_rx) = watch::channel(false);
    spawn_shutdown_watchers(accept_tx, external_shutdown);

    #[cfg(feature = "tls")]
    if let (AcceptKind::Tcp(ref listener), Some(ref tls_cfg)) = (&accept, &tls) {
        if let Some(redirect_port) = tls_cfg.redirect_http {
            if let Ok(local) = listener.local_addr() {
                let rx = svc_rx.clone();
                tokio::spawn(crate::tls::spawn_http_redirect(
                    redirect_port, local, rx,
                ));
            }
        }
    }

    let mut service_tasks = JoinSet::new();
    if start_services {
        for svc in services {
            let name = svc.name().to_string();
            let state = Arc::clone(&state);
            let rx = svc_rx.clone();
            tracing::info!(service = %name, "starting background service");
            service_tasks.spawn(async move {
                svc.run(state, crate::service::Shutdown::new(rx)).await;
                tracing::info!(service = %name, "background service stopped");
            });
        }
    } else if !services.is_empty() {
        tracing::debug!(
            n = services.len(),
            "skipping BackgroundServices (CLI mode; use .service_in_cli(true))"
        );
        drop(services);
    }

    let mut tasks: JoinSet<()> = JoinSet::new();
    let mut accept_rx = accept_rx;

    match accept {
        AcceptKind::Tcp(listener) => {
            accept_tcp(
                listener,
                &inner,
                &conn_limit,
                &mut accept_rx,
                &mut tasks,
                #[cfg(feature = "tls")]
                tls,
            )
            .await?;
        }
        #[cfg(unix)]
        AcceptKind::Unix(listener) => {
            accept_unix(listener, &inner, &conn_limit, &mut accept_rx, &mut tasks).await?;
        }
    }

    drain_tasks(&mut tasks, drain_timeout).await;

    // Stop services after connection drain.
    let _ = svc_tx.send(true);
    while service_tasks.join_next().await.is_some() {}

    for hook in &shutdowns {
        hook().await;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn accept_tcp(
    listener: TcpListener,
    inner: &Arc<AppInner>,
    conn_limit: &Arc<Semaphore>,
    shutdown_rx: &mut watch::Receiver<bool>,
    tasks: &mut JoinSet<()>,
    #[cfg(feature = "tls")] tls: Option<crate::tls::TlsRuntime>,
) -> Result<()> {
    #[cfg(feature = "tls")]
    let tls_cfg = tls.map(|t| (t.acceptor, t.handshake_timeout));

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
                spawn_conn_tcp(
                    inner,
                    conn_limit,
                    shutdown_rx,
                    tasks,
                    stream,
                    peer,
                    #[cfg(feature = "tls")]
                    tls_cfg.clone(),
                );
            }
        }
    }
    Ok(())
}

#[cfg(feature = "tls")]
type TlsConnCfg = (tokio_rustls::TlsAcceptor, Duration);

fn spawn_conn_tcp(
    inner: &Arc<AppInner>,
    conn_limit: &Arc<Semaphore>,
    shutdown_rx: &watch::Receiver<bool>,
    tasks: &mut JoinSet<()>,
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    #[cfg(feature = "tls")] tls: Option<TlsConnCfg>,
) {
    let permit = match conn_limit.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            tracing::warn!("max connections reached, rejecting {peer}");
            drop(stream);
            return;
        }
    };

    let inner = Arc::clone(inner);
    let conn_shutdown = shutdown_rx.clone();
    tasks.spawn(async move {
        let _permit: OwnedSemaphorePermit = permit;
        #[cfg(feature = "tls")]
        if let Some((acceptor, handshake_timeout)) = tls {
            match tokio::time::timeout(handshake_timeout, acceptor.accept(stream)).await {
                Ok(Ok(tls_stream)) => {
                    serve_http1(inner, TokioIo::new(tls_stream), peer, conn_shutdown).await;
                }
                Ok(Err(e)) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("unexpected eof") {
                        tracing::debug!(%peer, "tls handshake: looks like plain HTTP on TLS port");
                    } else {
                        tracing::debug!(%peer, error = %e, "tls handshake failed");
                    }
                }
                Err(_) => {
                    tracing::debug!(%peer, "tls handshake timeout");
                }
            }
            return;
        }
        serve_http1(inner, TokioIo::new(stream), peer, conn_shutdown).await;
    });
}

#[cfg(unix)]
async fn accept_unix(
    listener: tokio::net::UnixListener,
    inner: &Arc<AppInner>,
    conn_limit: &Arc<Semaphore>,
    shutdown_rx: &mut watch::Receiver<bool>,
    tasks: &mut JoinSet<()>,
) -> Result<()> {
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
                spawn_conn(inner, conn_limit, shutdown_rx, tasks, TokioIo::new(stream), peer);
            }
        }
    }
    Ok(())
}

fn spawn_conn<I>(
    inner: &Arc<AppInner>,
    conn_limit: &Arc<Semaphore>,
    shutdown_rx: &watch::Receiver<bool>,
    tasks: &mut JoinSet<()>,
    io: TokioIo<I>,
    peer: SocketAddr,
) where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let permit = match conn_limit.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            tracing::warn!("max connections reached, rejecting {peer}");
            drop(io);
            return;
        }
    };

    let inner = Arc::clone(inner);
    let conn_shutdown = shutdown_rx.clone();
    tasks.spawn(async move {
        let _permit: OwnedSemaphorePermit = permit;
        serve_http1(inner, io, peer, conn_shutdown).await;
    });
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
    let is_http2 = req.version() == http::Version::HTTP_2;
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
            match to_ruvo_request(req, &app, peer).await {
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

        let mut res = res;
        if is_http2
            && (res.headers.contains_key(http::header::CONNECTION)
                || res.headers.contains_key(http::header::TRANSFER_ENCODING)
                || res.headers.contains_key(http::HeaderName::from_static("keep-alive"))
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

async fn to_ruvo_request(
    req: HyperRequest<Incoming>,
    app: &AppInner,
    peer: SocketAddr,
) -> Result<Request> {
    use crate::request::{resolve_scheme_host, ReqBody};

    let max_body = app.max_body_size;
    let state = app.state();
    let trust_proxy = app.trust_proxy;

    let (mut parts, incoming) = req.into_parts();
    let on_upgrade = parts.extensions.remove::<HyperOnUpgrade>();

    let method = parts.method;
    let uri = parts.uri;
    let path = uri.path().to_string();
    let raw_query = uri.query().unwrap_or("").to_string();
    let query = if raw_query.is_empty() {
        Default::default()
    } else {
        parse_query(&raw_query)
    };
    let headers = parts.headers;

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
    if let Some(on_upgrade) = on_upgrade {
        extensions.insert(PendingUpgrade {
            on_upgrade,
            budget: app.max_upgraded.clone(),
        });
    }

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

fn to_hyper_response(res: Response, hsts: bool, alt_svc: Option<&str>) -> HyperResponse<ResponseBody> {
    let status = res.status;
    let headers = res.headers.clone();
    let body = res.into_http_body();
    let mut builder = HyperResponse::builder().status(status);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    if hsts {
        builder = builder.header("strict-transport-security", "max-age=31536000");
    }
    if let Some(alt_svc) = alt_svc {
        builder = builder.header("alt-svc", alt_svc);
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
