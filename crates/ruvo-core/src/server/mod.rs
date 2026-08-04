mod body;
mod conn;
mod forwarded;

#[allow(unused_imports)] // used via crate::server::collect_limited (tests / callers)
pub use body::collect_limited;

use crate::app::App;
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

pub async fn listen(
    app: App,
    port: Option<u16>,
    addr: Option<SocketAddr>,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    let (inner, startups, shutdowns) = app.into_listen_parts()?;
    let bind = addr
        .or(inner.listen_addr)
        .or_else(|| port.map(|p| SocketAddr::from(([0, 0, 0, 0], p))))
        .ok_or_else(|| Error::Internal("listen: port or address required".into()))?;

    let listener = TcpListener::bind(bind)
        .await
        .map_err(|e| Error::Internal(format!("bind {bind}: {e}")))?;

    conn::run_tcp(inner, startups, shutdowns, listener, external_shutdown).await
}

pub async fn listen_with_listener(
    app: App,
    listener: TcpListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    let (inner, startups, shutdowns) = app.into_listen_parts()?;
    conn::run_tcp(inner, startups, shutdowns, listener, external_shutdown).await
}

#[cfg(unix)]
pub async fn listen_uds(
    app: App,
    path: &Path,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    use tokio::net::UnixListener;

    let (inner, startups, shutdowns) = app.into_listen_parts()?;
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)
        .map_err(|e| Error::Internal(format!("bind uds {}: {e}", path.display())))?;
    tracing::info!("ruvo listening on unix:{}", path.display());
    conn::run_unix(inner, startups, shutdowns, listener, external_shutdown).await
}
