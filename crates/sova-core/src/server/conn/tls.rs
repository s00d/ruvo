#[cfg(feature = "tls")]
use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::sync::Arc;
#[cfg(feature = "tls")]
use std::time::Duration;

#[cfg(feature = "tls")]
use crate::app::AppInner;
#[cfg(feature = "tls")]
use hyper_util::rt::TokioIo;
#[cfg(feature = "tls")]
use tokio::sync::watch;

#[cfg(feature = "tls")]
use super::serve::serve_http1;

#[cfg(feature = "tls")]
pub(super) type TlsConnCfg = (tokio_rustls::TlsAcceptor, Duration);

#[cfg(feature = "tls")]
pub(super) async fn accept_tls_stream(
    inner: Arc<AppInner>,
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    conn_shutdown: watch::Receiver<bool>,
    tls: TlsConnCfg,
) {
    let (acceptor, handshake_timeout) = tls;
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
}
