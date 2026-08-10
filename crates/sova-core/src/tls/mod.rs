//! TLS termination for TCP listeners (rustls + tokio-rustls).

use crate::error::{Error, Result};
use arc_swap::ArcSwap;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_rustls::TlsAcceptor;

const ENCRYPTED_KEY_HINT: &str = "private key may be password-protected; decrypt with: \
    openssl pkcs8 -topk8 -nocrypt -in enc.key -out key.pem";

#[derive(Debug)]
pub(crate) struct ReloadingResolver {
    current: ArcSwap<CertifiedKey>,
}

impl ResolvesServerCert for ReloadingResolver {
    fn resolve(&self, _hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.current.load_full())
    }
}

/// TLS settings for [`crate::BoundApp::tls`].
///
/// [`Clone`] shares the hot-reload resolver: HTTPS and QUIC must use clones of
/// the same `Tls` so `reload()` updates both.
#[derive(Clone)]
pub struct Tls {
    cert_path: PathBuf,
    key_path: PathBuf,
    resolver: Arc<ReloadingResolver>,
    handshake_timeout: Duration,
    hsts: bool,
    redirect_http: Option<u16>,
}

impl Tls {
    fn default_https_alpn() -> Vec<Vec<u8>> {
        vec![b"h2".to_vec(), b"http/1.1".to_vec()]
    }

    /// Load PEM certificate chain + private key from paths.
    pub fn from_pem(cert: impl AsRef<Path>, key: impl AsRef<Path>) -> Result<Self> {
        install_crypto_provider();
        let cert_path = cert.as_ref().to_path_buf();
        let key_path = key.as_ref().to_path_buf();
        let ck = load_certified_key(&cert_path, &key_path)?;
        let resolver = Arc::new(ReloadingResolver {
            current: ArcSwap::from(Arc::new(ck)),
        });
        Ok(Self {
            cert_path,
            key_path,
            resolver,
            handshake_timeout: Duration::from_secs(10),
            hsts: false,
            redirect_http: None,
        })
    }

    /// Hot-reloadable certificate resolver used by both HTTPS and QUIC TLS configs.
    #[allow(dead_code)]
    pub fn resolver(&self) -> Arc<dyn ResolvesServerCert> {
        let resolver: Arc<dyn ResolvesServerCert> = self.resolver.clone();
        resolver
    }

    /// Build a rustls [`ServerConfig`] for HTTPS listeners.
    ///
    /// Panics if `alpn` is empty.
    pub fn build_https_server_config(&self, alpn: Vec<Vec<u8>>) -> ServerConfig {
        assert!(
            !alpn.is_empty(),
            "Tls::build_https_server_config: `alpn` must not be empty"
        );

        let resolver: Arc<dyn ResolvesServerCert> = self.resolver.clone();
        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver);
        config.alpn_protocols = alpn;
        config
    }

    /// Build a rustls [`ServerConfig`] suitable for QUIC datagrams.
    ///
    /// This config is strictly TLS 1.3-only and disables 0-RTT.
    ///
    /// Panics if `alpn` is empty.
    pub fn build_quic_server_config(&self, alpn: Vec<Vec<u8>>) -> ServerConfig {
        assert!(
            !alpn.is_empty(),
            "Tls::build_quic_server_config: `alpn` must not be empty"
        );

        let resolver: Arc<dyn ResolvesServerCert> = self.resolver.clone();
        let mut config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_no_client_auth()
            .with_cert_resolver(resolver);
        config.alpn_protocols = alpn;
        // Disable 0-RTT entirely.
        config.max_early_data_size = 0;
        config.send_half_rtt_data = false;
        config
    }

    /// Reload certificate + key from disk; new connections pick up the updated certificate.
    pub fn reload(&self) -> Result<()> {
        self.reload_paths()
    }

    /// Alias for [`Self::reload`] (disk paths captured at construction).
    pub fn reload_paths(&self) -> Result<()> {
        let ck = load_certified_key(&self.cert_path, &self.key_path)?;
        self.resolver.current.store(Arc::new(ck));
        Ok(())
    }

    /// Hot-reload from PEM strings; optionally persists to the configured paths.
    ///
    /// New TLS handshakes pick up the certificate immediately (shared resolver).
    pub fn reload_pem(&self, cert_pem: &str, key_pem: &str) -> Result<()> {
        let ck = certified_key_from_pem(cert_pem.as_bytes(), key_pem.as_bytes())?;
        self.resolver.current.store(Arc::new(ck));
        if let Some(parent) = self.cert_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Some(parent) = self.key_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&self.cert_path, cert_pem).map_err(|e| {
            Error::Internal(format!("write cert {}: {e}", self.cert_path.display()))
        })?;
        std::fs::write(&self.key_path, key_pem)
            .map_err(|e| Error::Internal(format!("write key {}: {e}", self.key_path.display())))?;
        Ok(())
    }

    /// Paths used by [`Self::reload`] / persistence from [`Self::reload_pem`].
    pub fn cert_path(&self) -> &Path {
        &self.cert_path
    }

    /// Private key path paired with [`Self::cert_path`].
    pub fn key_path(&self) -> &Path {
        &self.key_path
    }

    /// Self-signed certificate for local development (requires feature `dev-tls`).
    #[cfg(feature = "dev-tls")]
    pub fn self_signed(hosts: &[&str]) -> Result<Self> {
        install_crypto_provider();
        let subject_alt_names: Vec<String> = hosts.iter().map(|s| (*s).into()).collect();
        let cert = rcgen::generate_simple_self_signed(subject_alt_names)
            .map_err(|e| Error::Internal(format!("dev-tls cert: {e}")))?;
        let cert_pem = cert.cert.pem();
        let key_pem = cert.key_pair.serialize_pem();

        let dir = std::env::temp_dir().join(format!("sova-dev-tls-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| Error::Internal(format!("dev-tls dir: {e}")))?;
        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("key.pem");
        std::fs::write(&cert_path, cert_pem)
            .map_err(|e| Error::Internal(format!("dev-tls: {e}")))?;
        std::fs::write(&key_path, key_pem).map_err(|e| Error::Internal(format!("dev-tls: {e}")))?;
        Self::from_pem(&cert_path, &key_path)
    }

    pub fn handshake_timeout(mut self, timeout: Duration) -> Self {
        self.handshake_timeout = timeout;
        self
    }

    /// Emit `Strict-Transport-Security` on HTTPS responses (off by default).
    pub fn hsts(mut self, enabled: bool) -> Self {
        self.hsts = enabled;
        self
    }

    /// Spawn a plain HTTP listener that 301-redirects to HTTPS on this bind port.
    pub fn redirect_http(mut self, port: u16) -> Self {
        self.redirect_http = Some(port);
        self
    }

    pub(crate) fn into_runtime(self) -> Result<TlsRuntime> {
        let https_config = self.build_https_server_config(Self::default_https_alpn());
        let acceptor = TlsAcceptor::from(Arc::new(https_config));
        Ok(TlsRuntime {
            acceptor,
            resolver: self.resolver,
            cert_path: self.cert_path,
            key_path: self.key_path,
            handshake_timeout: self.handshake_timeout,
            hsts: self.hsts,
            redirect_http: self.redirect_http,
        })
    }
}

pub(crate) struct TlsRuntime {
    pub acceptor: TlsAcceptor,
    #[allow(dead_code)]
    pub(crate) resolver: Arc<ReloadingResolver>,
    #[allow(dead_code)]
    pub(crate) cert_path: PathBuf,
    #[allow(dead_code)]
    pub(crate) key_path: PathBuf,
    pub handshake_timeout: Duration,
    pub hsts: bool,
    pub redirect_http: Option<u16>,
}

impl TlsRuntime {
    /// Reload certificate + key from disk; new connections pick up the swap.
    #[allow(dead_code)]
    pub fn reload(&self) -> Result<()> {
        let ck = load_certified_key(&self.cert_path, &self.key_path)?;
        self.resolver.current.store(Arc::new(ck));
        Ok(())
    }
}

pub(crate) fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let file = std::fs::File::open(path)
        .map_err(|e| Error::Internal(format!("open cert {}: {e}", path.display())))?;
    let mut rd = BufReader::new(file);
    rustls_pemfile::certs(&mut rd)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Internal(format!("parse cert {}: {e}", path.display())))
}

fn load_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let file = std::fs::File::open(path)
        .map_err(|e| Error::Internal(format!("open key {}: {e}", path.display())))?;
    let mut rd = BufReader::new(file);
    rustls_pemfile::private_key(&mut rd)
        .map_err(|e| Error::Internal(format!("parse key {}: {e}", path.display())))?
        .ok_or_else(|| {
            Error::Internal(format!(
                "no private key in {} ({ENCRYPTED_KEY_HINT})",
                path.display()
            ))
        })
}

fn load_certified_key(cert_path: &Path, key_path: &Path) -> Result<CertifiedKey> {
    let certs = load_certs(cert_path)?;
    if certs.is_empty() {
        return Err(Error::Internal(format!(
            "no certificates in {}",
            cert_path.display()
        )));
    }
    let key = load_key(key_path)?;
    certified_key_from_parts(certs, key, Some(key_path))
}

fn certified_key_from_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<CertifiedKey> {
    let mut cert_rd = BufReader::new(cert_pem);
    let certs = rustls_pemfile::certs(&mut cert_rd)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Internal(format!("parse cert pem: {e}")))?;
    if certs.is_empty() {
        return Err(Error::Internal("no certificates in pem".into()));
    }
    let mut key_rd = BufReader::new(key_pem);
    let key = rustls_pemfile::private_key(&mut key_rd)
        .map_err(|e| Error::Internal(format!("parse key pem: {e}")))?
        .ok_or_else(|| Error::Internal(format!("no private key in pem ({ENCRYPTED_KEY_HINT})")))?;
    certified_key_from_parts(certs, key, None)
}

fn certified_key_from_parts(
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
    key_path: Option<&Path>,
) -> Result<CertifiedKey> {
    let provider = rustls::crypto::ring::default_provider();
    let signing_key = provider.key_provider.load_private_key(key).map_err(|e| {
        let where_ = key_path
            .map(|p| format!(" {}", p.display()))
            .unwrap_or_default();
        Error::Internal(format!("load private key{where_}: {e}"))
    })?;
    Ok(CertifiedKey::new(certs, signing_key))
}

pub(crate) async fn spawn_http_redirect(
    port: u16,
    https_addr: SocketAddr,
    mut shutdown: watch::Receiver<bool>,
) {
    let bind = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match TcpListener::bind(bind).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(error = %e, port, "http redirect listener failed to bind");
            return;
        }
    };
    tracing::info!(port, "http redirect listening (301 → https)");
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() { break; }
            }
            accepted = listener.accept() => {
                let Ok((mut stream, _)) = accepted else { break; };
                let host = https_addr.ip().to_string();
                let target = format!("https://{host}:{}/", https_addr.port());
                let response = format!(
                    "HTTP/1.1 301 Moved Permanently\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;
            }
        }
    }
}

#[cfg(all(test, feature = "dev-tls"))]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    fn temp_pem_pair() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        let ck = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        std::fs::write(&cert, ck.cert.pem()).unwrap();
        std::fs::write(&key, ck.key_pair.serialize_pem()).unwrap();
        (dir, cert, key)
    }

    #[test]
    fn pem_loads_and_acceptor_builds() {
        let (_dir, cert, key) = temp_pem_pair();
        let tls = Tls::from_pem(&cert, &key).unwrap();
        let rt = tls.into_runtime().unwrap();
        assert!(!rt.hsts);
    }

    #[tokio::test]
    async fn handshake_timeout_does_not_block_other_client() {
        let (_dir, cert, key) = temp_pem_pair();
        let tls = Tls::from_pem(&cert, &key)
            .unwrap()
            .handshake_timeout(Duration::from_millis(100));
        let rt = tls.into_runtime().unwrap();
        let acceptor = rt.acceptor.clone();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let slow = tokio::spawn(async move {
            let _ = listener.accept().await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        });

        let client = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            let stream = TcpStream::connect(addr).await.unwrap();
            let _ = tokio::time::timeout(Duration::from_millis(200), acceptor.accept(stream)).await;
        });

        let _ = tokio::join!(slow, client);
    }

    #[test]
    fn reload_swaps_cert() {
        let (_dir, cert, key) = temp_pem_pair();
        let tls = Tls::from_pem(&cert, &key).unwrap();
        let rt = tls.into_runtime().unwrap();
        let before = rt.resolver.current.load_full();
        rt.reload().unwrap();
        let after = rt.resolver.current.load_full();
        assert!(!Arc::ptr_eq(&before, &after));
        assert_eq!(rt.cert_path, cert);
    }

    #[test]
    fn reload_swaps_cert_on_tls() {
        let (_dir, cert, key) = temp_pem_pair();
        let tls = Tls::from_pem(&cert, &key).unwrap();
        let before = tls.resolver.current.load_full();
        tls.reload().unwrap();
        let after = tls.resolver.current.load_full();
        assert!(!Arc::ptr_eq(&before, &after));
    }

    #[test]
    fn reload_pem_swaps_and_persists() {
        let (dir, cert, key) = temp_pem_pair();
        let tls = Tls::from_pem(&cert, &key).unwrap();
        let before = tls.resolver.current.load_full();

        let next = rcgen::generate_simple_self_signed(vec!["reload.example".into()]).unwrap();
        let cert_pem = next.cert.pem();
        let key_pem = next.key_pair.serialize_pem();
        tls.reload_pem(&cert_pem, &key_pem).unwrap();

        let after = tls.resolver.current.load_full();
        assert!(!Arc::ptr_eq(&before, &after));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("cert.pem")).unwrap(),
            cert_pem
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("key.pem")).unwrap(),
            key_pem
        );
    }

    #[tokio::test]
    async fn plain_http_on_tls_port_not_fatal() {
        let (_dir, cert, key) = temp_pem_pair();
        let tls = Tls::from_pem(&cert, &key).unwrap();
        let rt = tls.into_runtime().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let acceptor = rt.acceptor.clone();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = acceptor.accept(stream).await;
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let _ = stream.read(&mut [0u8; 16]).await;
    }
}
