use bytes::Bytes;
use ruvo_core::extend::{wait_shutdown, BoxFuture as RuvoBoxFuture, StateMap};
use ruvo_core::{BackgroundService, Error, Result, Shutdown};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::config::{build_client_config, build_server_config};
use crate::endpoint::{accept_handshake, bind_server, prepare_incoming, PrepareIncoming};

/// Handler invoked for each received QUIC datagram.
pub type QuicDatagramHandler =
    Arc<dyn Fn(SocketAddr, Vec<u8>, quinn::Connection) -> RuvoBoxFuture<()> + Send + Sync>;

/// QUIC server-side datagram listener (`BackgroundService`).
pub struct QuicDatagramService {
    bind_addr: SocketAddr,
    tls: Arc<ruvo_core::Tls>,
    alpn: Vec<Vec<u8>>,
    handler: QuicDatagramHandler,
    preshared_token: Option<Vec<u8>>,
    name: String,
}

impl QuicDatagramService {
    /// Build from an existing [`ruvo_core::Tls`] (share with HTTPS via [`Clone`]).
    pub fn from_tls(
        bind_addr: SocketAddr,
        tls: ruvo_core::Tls,
        alpn: Vec<Vec<u8>>,
        retry: bool,
        handler: QuicDatagramHandler,
    ) -> Result<Self> {
        if alpn.is_empty() {
            return Err(Error::Internal(
                "QuicDatagramService: `alpn` must not be empty".into(),
            ));
        }
        if !retry {
            tracing::debug!(
                retry,
                "quic anti-amplification retry=false is ignored; retry is always enabled"
            );
        }
        Ok(Self {
            bind_addr,
            tls: Arc::new(tls),
            alpn,
            handler,
            preshared_token: None,
            name: format!("quic:{bind_addr}"),
        })
    }

    pub fn from_pem(
        bind_addr: SocketAddr,
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
        alpn: Vec<Vec<u8>>,
        retry: bool,
        handler: QuicDatagramHandler,
    ) -> Result<Self> {
        let tls = ruvo_core::Tls::from_pem(cert_path, key_path)?;
        Self::from_tls(bind_addr, tls, alpn, retry, handler)
    }

    /// Self-signed certificate for local development (requires feature `dev-tls`).
    #[cfg(feature = "dev-tls")]
    pub fn self_signed(
        bind_addr: SocketAddr,
        hosts: &[&str],
        alpn: Vec<Vec<u8>>,
        retry: bool,
        handler: QuicDatagramHandler,
    ) -> Result<Self> {
        let tls = ruvo_core::Tls::self_signed(hosts)?;
        Self::from_tls(bind_addr, tls, alpn, retry, handler)
    }

    /// Optional preshared token inside datagrams: `token + b':' + payload`.
    ///
    /// If `token` is `None`, no validation is performed.
    pub fn with_preshared_token(mut self, token: Option<String>) -> Self {
        self.preshared_token = token.map(|t| t.into_bytes());
        self
    }
}

impl BackgroundService for QuicDatagramService {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(
        self: Box<Self>,
        _state: Arc<StateMap>,
        shutdown: Shutdown,
    ) -> RuvoBoxFuture<()> {
        Box::pin(async move {
            let server_config = match build_server_config(&self.tls, self.alpn.clone(), true) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, "quic server TLS config build failed");
                    return;
                }
            };

            let endpoint = match bind_server(server_config, self.bind_addr, "quic datagram") {
                Some(ep) => ep,
                None => return,
            };

            tracing::info!(addr = %self.bind_addr, "quic datagram listening");

            let mut recv_tasks: JoinSet<()> = JoinSet::new();
            loop {
                tokio::select! {
                    _ = wait_shutdown(shutdown.clone()) => break,
                    incoming = endpoint.accept() => {
                        let Some(incoming) = incoming else { break; };

                        let incoming = match prepare_incoming(incoming) {
                            PrepareIncoming::RetrySent => continue,
                            PrepareIncoming::Ready(i) => *i,
                        };

                        let conn = match accept_handshake(incoming, "quic handshake failed").await {
                            Some(c) => c,
                            None => continue,
                        };

                        let handler = Arc::clone(&self.handler);
                        let token = self.preshared_token.clone();
                        let shutdown = shutdown.clone();
                        recv_tasks.spawn(async move {
                            recv_loop(conn, handler, token, shutdown).await
                        });
                    }
                }
            }

            // Stop accepting new connections; existing recv loops exit via shutdown.
            endpoint.close(0u8.into(), b"shutdown");

            while let Some(res) = recv_tasks.join_next().await {
                if let Err(e) = res {
                    tracing::debug!(error = %e, "quic recv task join error");
                }
            }
        })
    }
}

async fn recv_loop(
    conn: quinn::Connection,
    handler: QuicDatagramHandler,
    preshared_token: Option<Vec<u8>>,
    shutdown: Shutdown,
) {
    let peer = conn.remote_address();
    loop {
        tokio::select! {
            _ = wait_shutdown(shutdown.clone()) => break,
            res = conn.read_datagram() => {
                let Ok(data) = res else {
                    break;
                };
                let data = data.to_vec();

                if let Some(max) = conn.max_datagram_size() {
                    if data.len() > max {
                        tracing::error!(
                            peer = %peer,
                            payload_len = data.len(),
                            max_datagram_size = max,
                            "quic datagram payload exceeds max_datagram_size; dropping"
                        );
                        continue;
                    }
                } else {
                    tracing::debug!("quic connection has no max_datagram_size; skipping datagram");
                    continue;
                }

                let payload = if let Some(expected) = preshared_token.as_deref() {
                    if let Some(colon) = data.iter().position(|b| *b == b':') {
                        if data[..colon] == expected[..] {
                            data[colon + 1..].to_vec()
                        } else {
                            tracing::debug!("quic datagram token mismatch; dropping");
                            continue;
                        }
                    } else {
                        data
                    }
                } else {
                    data
                };

                (handler)(peer, payload, conn.clone()).await;
            }
        }
    }
}

/// QUIC client for sending datagrams and receiving a response.
pub struct QuicDatagramClient {
    _endpoint: quinn::Endpoint,
    conn: quinn::Connection,
}

impl QuicDatagramClient {
    /// Connect to a QUIC datagram server.
    pub async fn connect(
        bind_addr: SocketAddr,
        server_addr: SocketAddr,
        host: &str,
        alpn: Vec<Vec<u8>>,
    ) -> Result<Self> {
        if alpn.is_empty() {
            return Err(Error::Internal(
                "QuicDatagramClient: `alpn` must not be empty".into(),
            ));
        }

        let client_config = build_client_config(alpn)?;

        let mut endpoint = quinn::Endpoint::client(bind_addr)
            .map_err(|e| Error::Internal(format!("quic endpoint client bind failed: {e}")))?;
        endpoint.set_default_client_config(client_config);

        let connecting = endpoint
            .connect(server_addr, host)
            .map_err(|e| Error::Internal(format!("quic endpoint connect failed: {e}")))?;
        let conn = connecting
            .await
            .map_err(|e| Error::Internal(format!("quic connect failed: {e}")))?;

        Ok(Self {
            _endpoint: endpoint,
            conn,
        })
    }

    pub fn max_datagram_size(&self) -> Option<usize> {
        self.conn.max_datagram_size()
    }

    /// Returns peer certificate chain (leaf first) as raw DER bytes.
    pub fn peer_certificates(&self) -> Option<Vec<Vec<u8>>> {
        let identity = self.conn.peer_identity()?;
        identity
            .downcast::<Vec<rustls::pki_types::CertificateDer<'static>>>()
            .ok()
            .map(|certs| certs.into_iter().map(|c| c.as_ref().to_vec()).collect())
    }

    pub async fn send_datagram(&self, data: &[u8]) -> Result<()> {
        self.conn
            .send_datagram(Bytes::copy_from_slice(data))
            .map_err(|e| Error::Internal(format!("send_datagram failed: {e}")))
    }

    pub async fn recv_datagram(&self) -> Result<Vec<u8>> {
        let bytes = self
            .conn
            .read_datagram()
            .await
            .map_err(|e| Error::Internal(format!("recv_datagram failed: {e}")))?;
        Ok(bytes.to_vec())
    }

    pub async fn send_and_recv(
        &self,
        data: &[u8],
        timeout: std::time::Duration,
    ) -> Result<Vec<u8>> {
        self.send_datagram(data).await?;
        tokio::time::timeout(timeout, self.recv_datagram())
            .await
            .map_err(|_| Error::Internal("timed out waiting for datagram".into()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruvo_core::extend::StateMap;
    use rcgen::generate_simple_self_signed;
    use rustls::pki_types::CertificateDer;
    use std::fs;
    use tempfile::TempDir;

    fn unused_local_addr() -> SocketAddr {
        let sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind udp");
        sock.local_addr().expect("local addr")
    }

    fn write_self_signed_pem(tmp: &TempDir, subject: &[&str]) -> (std::path::PathBuf, std::path::PathBuf, Vec<u8>) {
        let subject_alt_names: Vec<String> = subject.iter().map(|s| (*s).into()).collect();
        let cert = generate_simple_self_signed(subject_alt_names).expect("rcgen");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.key_pair.serialize_pem();

        let cert_path = tmp.path().join("cert.pem");
        let key_path = tmp.path().join("key.pem");
        fs::write(&cert_path, cert_pem).expect("write cert");
        fs::write(&key_path, key_pem).expect("write key");

        let mut rd = std::io::BufReader::new(fs::File::open(&cert_path).expect("open cert pem"));
        let certs = rustls_pemfile::certs(&mut rd).collect::<std::result::Result<Vec<_>, _>>().expect("parse certs");
        let leaf: CertificateDer<'static> = certs.into_iter().next().expect("at least 1 cert");

        (cert_path, key_path, leaf.as_ref().to_vec())
    }

    #[tokio::test]
    async fn echo_roundtrip() {
        let bind_addr = unused_local_addr();
        let client_bind = unused_local_addr();
        let server_alpn = vec![b"ruvo-quic-udp".to_vec()];

        let tmp = TempDir::new().unwrap();
        let (cert_path, key_path, _cert_der) = write_self_signed_pem(&tmp, &["localhost"]);

        let handler: QuicDatagramHandler = Arc::new(|_peer, data, conn| {
            Box::pin(async move {
                let _ = conn.send_datagram(Bytes::from(data));
            })
        });

        let svc = QuicDatagramService::from_pem(
            bind_addr,
            &cert_path,
            &key_path,
            server_alpn.clone(),
            true,
            handler,
        )
        .unwrap();

        let (tx, shutdown) = ruvo_core::shutdown_channel();
        let handle = tokio::spawn(Box::new(svc).run(Arc::new(StateMap::new()), shutdown));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = QuicDatagramClient::connect(client_bind, bind_addr, "localhost", server_alpn)
            .await
            .unwrap();

        let resp = client
            .send_and_recv(b"ping", std::time::Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(resp, b"ping");

        let _ = tx.send(true);
        let _ = handle.await;
    }

    #[tokio::test]
    async fn handshake_requires_alpn() {
        let bind_addr = unused_local_addr();
        let client_bind = unused_local_addr();
        let server_alpn = vec![b"A".to_vec()];
        let client_alpn = vec![b"B".to_vec()];

        let tmp = TempDir::new().unwrap();
        let (cert_path, key_path, _cert_der) = write_self_signed_pem(&tmp, &["localhost"]);

        let handler: QuicDatagramHandler = Arc::new(|_peer, _data, _conn| Box::pin(async {}));

        let svc = QuicDatagramService::from_pem(
            bind_addr,
            &cert_path,
            &key_path,
            server_alpn.clone(),
            true,
            handler,
        )
        .unwrap();

        let (tx, shutdown) = ruvo_core::shutdown_channel();
        let handle = tokio::spawn(Box::new(svc).run(Arc::new(StateMap::new()), shutdown));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let res = QuicDatagramClient::connect(client_bind, bind_addr, "localhost", client_alpn).await;
        assert!(res.is_err());

        let _ = tx.send(true);
        let _ = handle.await;
    }

    #[tokio::test]
    async fn max_datagram_size_accepted() {
        let bind_addr = unused_local_addr();
        let client_bind = unused_local_addr();
        let server_alpn = vec![b"ruvo-quic-udp".to_vec()];

        let tmp = TempDir::new().unwrap();
        let (cert_path, key_path, _cert_der) = write_self_signed_pem(&tmp, &["localhost"]);

        let handler: QuicDatagramHandler = Arc::new(|_peer, data, conn| {
            Box::pin(async move {
                let _ = conn.send_datagram(Bytes::from(data));
            })
        });

        let svc = QuicDatagramService::from_pem(
            bind_addr,
            &cert_path,
            &key_path,
            server_alpn.clone(),
            true,
            handler,
        )
        .unwrap();

        let (tx, shutdown) = ruvo_core::shutdown_channel();
        let handle = tokio::spawn(Box::new(svc).run(Arc::new(StateMap::new()), shutdown));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = QuicDatagramClient::connect(client_bind, bind_addr, "localhost", server_alpn)
            .await
            .unwrap();

        let max = client.max_datagram_size().expect("max_datagram_size");
        let len = max.saturating_sub(64).max(1);
        let payload = vec![0xABu8; len];

        let resp = client
            .send_and_recv(&payload, std::time::Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(resp, payload);

        let _ = tx.send(true);
        let _ = handle.await;
    }

    #[tokio::test]
    async fn shutdown_stops_recv_loop() {
        let bind_addr = unused_local_addr();
        let client_bind = unused_local_addr();
        let server_alpn = vec![b"ruvo-quic-udp".to_vec()];

        let tmp = TempDir::new().unwrap();
        let (cert_path, key_path, _cert_der) = write_self_signed_pem(&tmp, &["localhost"]);

        let handler: QuicDatagramHandler = Arc::new(|_peer, _data, _conn| Box::pin(async {}));

        let svc = QuicDatagramService::from_pem(
            bind_addr,
            &cert_path,
            &key_path,
            server_alpn.clone(),
            true,
            handler,
        )
        .unwrap();

        let (tx, shutdown) = ruvo_core::shutdown_channel();
        let handle = tokio::spawn(Box::new(svc).run(Arc::new(StateMap::new()), shutdown));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _client =
            QuicDatagramClient::connect(client_bind, bind_addr, "localhost", server_alpn)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = tx.send(true);

        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("service did not stop after shutdown")
            .expect("join error");
    }

    #[tokio::test]
    async fn tls_reload_affects_new_connections() {
        let bind_addr = unused_local_addr();
        let client_bind1 = unused_local_addr();
        let client_bind2 = unused_local_addr();
        let server_alpn = vec![b"ruvo-quic-udp".to_vec()];

        let tmp = TempDir::new().unwrap();
        let (cert_path, key_path, cert1_der) = write_self_signed_pem(&tmp, &["localhost"]);
        let svc = QuicDatagramService::from_pem(
            bind_addr,
            &cert_path,
            &key_path,
            server_alpn.clone(),
            true,
            Arc::new(|_peer, _data, _conn| Box::pin(async {})),
        )
        .unwrap();

        let tls = svc.tls.clone();
        let (tx, shutdown) = ruvo_core::shutdown_channel();
        let handle = tokio::spawn(Box::new(svc).run(Arc::new(StateMap::new()), shutdown));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let client1 = QuicDatagramClient::connect(client_bind1, bind_addr, "localhost", server_alpn.clone())
            .await
            .unwrap();
        let peer1 = client1
            .peer_certificates()
            .and_then(|mut v| v.pop())
            .expect("peer cert");

        let (_cert_path2, _key_path2, cert2_der) = write_self_signed_pem(&tmp, &["localhost", "127.0.0.1"]);
        fs::write(&cert_path, fs::read(_cert_path2).unwrap()).unwrap();
        fs::write(&key_path, fs::read(_key_path2).unwrap()).unwrap();
        tls.reload().unwrap();

        let client2 = QuicDatagramClient::connect(client_bind2, bind_addr, "localhost", server_alpn)
            .await
            .unwrap();
        let peer2 = client2
            .peer_certificates()
            .and_then(|mut v| v.pop())
            .expect("peer cert");

        assert_eq!(peer1, cert1_der);
        assert_eq!(peer2, cert2_der);

        let _ = tx.send(true);
        let _ = handle.await;
    }
}
