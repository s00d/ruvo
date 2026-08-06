use crate::app::{AppInner, ShutdownHook, StartupHook};
use crate::error::{Error, Result};
use crate::service::BoxedService;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;

use super::ExternalShutdown;

use super::serve::serve_http1;
use hyper_util::rt::TokioIo;
#[cfg(feature = "tls")]
use super::tls::{accept_tls_stream, TlsConnCfg};

pub(super) enum AcceptKind {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(tokio::net::UnixListener),
}

#[cfg(not(feature = "tls"))]
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_accept_loop(
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
pub(super) async fn run_accept_loop(
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
pub(super) async fn accept_tcp(
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

pub(super) fn spawn_conn_tcp(
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
        if let Some(cfg) = tls {
            accept_tls_stream(inner, stream, peer, conn_shutdown, cfg).await;
            return;
        }
        serve_http1(inner, TokioIo::new(stream), peer, conn_shutdown).await;
    });
}

#[cfg(unix)]
pub(super) async fn accept_unix(
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

pub(super) fn spawn_conn<I>(
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

pub(super) fn spawn_shutdown_watchers(
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

pub(super) fn log_startup_banner(inner: &AppInner, addr: &str) {
    tracing::info!(
        addr = %addr,
        routes = inner.route_count,
        "ruvo listening"
    );
    #[cfg(debug_assertions)]
    tracing::debug!("routes:\n{}", inner.explain.trim_end());
}

pub(super) async fn drain_tasks(tasks: &mut JoinSet<()>, drain_timeout: Duration) {
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

pub(super) async fn wait_shutdown_signal() {
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
