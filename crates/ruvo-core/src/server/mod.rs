mod body;
mod conn;
mod forwarded;

#[allow(unused_imports)] // used via crate::server::collect_limited (tests / callers)
pub use body::collect_limited;

use crate::app::{App, ListenParts};
use crate::error::{Error, Result};
use std::net::SocketAddr;
use std::pin::Pin;
#[cfg(unix)]
use std::path::Path;
use tokio::net::TcpListener;

/// Peer address stored on each request for rate-limiting etc.
#[derive(Debug, Clone, Copy)]
pub struct ClientAddr(pub SocketAddr);

pub(crate) type ExternalShutdown = Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

#[cfg(feature = "tls")]
pub(crate) type TlsOpt = Option<crate::tls::TlsRuntime>;

#[cfg(not(feature = "tls"))]
pub async fn listen(
    app: App,
    port: Option<u16>,
    addr: Option<SocketAddr>,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    let ListenParts {
        inner,
        startups,
        shutdowns,
        services,
        start_services,
    } = app.into_listen_parts()?;
    let bind = addr
        .or_else(|| port.map(|p| SocketAddr::from(([0, 0, 0, 0], p))))
        .ok_or_else(|| Error::Internal("listen: port or address required".into()))?;
    let listener = bind_tcp(bind, inner.reuseport).await?;
    conn::run_tcp(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        listener,
        external_shutdown,
    )
    .await
}

#[cfg(feature = "tls")]
pub async fn listen(
    app: App,
    port: Option<u16>,
    addr: Option<SocketAddr>,
    external_shutdown: Option<ExternalShutdown>,
    tls: TlsOpt,
) -> Result<()> {
    let ListenParts {
        inner,
        startups,
        shutdowns,
        services,
        start_services,
    } = app.into_listen_parts()?;
    let bind = addr
        .or_else(|| port.map(|p| SocketAddr::from(([0, 0, 0, 0], p))))
        .ok_or_else(|| Error::Internal("listen: port or address required".into()))?;
    let listener = bind_tcp(bind, inner.reuseport).await?;
    conn::run_tcp(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        listener,
        external_shutdown,
        tls,
    )
    .await
}

async fn bind_tcp(bind: SocketAddr, reuseport: bool) -> Result<TcpListener> {
    if reuseport {
        #[cfg(feature = "listen-reuseport")]
        {
            return bind_reuseport(bind).await;
        }
        #[cfg(not(feature = "listen-reuseport"))]
        {
            return Err(Error::Internal(
                "BoundApp::reuseport(true) requires feature `listen-reuseport`".into(),
            ));
        }
    }
    TcpListener::bind(bind)
        .await
        .map_err(|e| Error::Internal(format!("bind {bind}: {e}")))
}

#[cfg(feature = "listen-reuseport")]
async fn bind_reuseport(bind: SocketAddr) -> Result<TcpListener> {
    use socket2::{Domain, Protocol, Socket, Type};
    let domain = if bind.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
        .map_err(|e| Error::Internal(format!("socket: {e}")))?;
    socket
        .set_reuse_address(true)
        .map_err(|e| Error::Internal(format!("SO_REUSEADDR: {e}")))?;
    #[cfg(all(unix, not(target_os = "solaris"), not(target_os = "illumos")))]
    socket
        .set_reuse_port(true)
        .map_err(|e| Error::Internal(format!("SO_REUSEPORT: {e}")))?;
    socket
        .set_nonblocking(true)
        .map_err(|e| Error::Internal(format!("nonblocking: {e}")))?;
    socket
        .bind(&bind.into())
        .map_err(|e| Error::Internal(format!("bind {bind}: {e}")))?;
    socket
        .listen(1024)
        .map_err(|e| Error::Internal(format!("listen: {e}")))?;
    let std_listener: std::net::TcpListener = socket.into();
    TcpListener::from_std(std_listener).map_err(|e| Error::Internal(format!("from_std: {e}")))
}

#[cfg(not(feature = "tls"))]
pub async fn listen_with_listener(
    app: App,
    listener: std::net::TcpListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    let ListenParts {
        inner,
        startups,
        shutdowns,
        services,
        start_services,
    } = app.into_listen_parts()?;
    let listener = TcpListener::from_std(listener)
        .map_err(|e| Error::Internal(format!("from_std: {e}")))?;
    conn::run_tcp(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        listener,
        external_shutdown,
    )
    .await
}

#[cfg(feature = "tls")]
pub async fn listen_with_listener(
    app: App,
    listener: std::net::TcpListener,
    external_shutdown: Option<ExternalShutdown>,
    tls: TlsOpt,
) -> Result<()> {
    let ListenParts {
        inner,
        startups,
        shutdowns,
        services,
        start_services,
    } = app.into_listen_parts()?;
    let listener = TcpListener::from_std(listener)
        .map_err(|e| Error::Internal(format!("from_std: {e}")))?;
    conn::run_tcp(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        listener,
        external_shutdown,
        tls,
    )
    .await
}

#[cfg(unix)]
pub async fn listen_uds(
    app: App,
    path: &Path,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    use tokio::net::UnixListener;

    let ListenParts {
        inner,
        startups,
        shutdowns,
        services,
        start_services,
    } = app.into_listen_parts()?;
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)
        .map_err(|e| Error::Internal(format!("bind uds {}: {e}", path.display())))?;
    tracing::info!("ruvo listening on unix:{}", path.display());
    conn::run_unix(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        listener,
        external_shutdown,
    )
    .await
}
