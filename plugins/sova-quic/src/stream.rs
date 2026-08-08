use bytes::{Buf, Bytes};
use sova_core::extend::{wait_shutdown, BoxFuture as SovaBoxFuture, StateMap};
use sova_core::{
    BackgroundService, Error, Request as SovaRequest, Result, Server as SovaServer, Shutdown,
};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::config::build_server_config;
use crate::endpoint::{accept_handshake, bind_server, prepare_incoming, PrepareIncoming};

/// HTTP/3 service over QUIC that dispatches requests through a compiled
/// `sova_core::Server` and writes responses back over h3.
pub struct Http3Service {
    bind_addr: SocketAddr,
    tls: Arc<sova_core::Tls>,
    alpn: Vec<Vec<u8>>,
    server: SovaServer,
    name: String,
}

impl Http3Service {
    /// Build from an existing [`sova_core::Tls`] (share with HTTPS via [`Clone`]).
    pub fn from_tls(
        bind_addr: SocketAddr,
        tls: sova_core::Tls,
        alpn: Vec<Vec<u8>>,
        server: SovaServer,
    ) -> Result<Self> {
        if alpn.is_empty() {
            return Err(Error::Internal("Http3Service: `alpn` must not be empty".into()));
        }
        Ok(Self {
            bind_addr,
            tls: Arc::new(tls),
            alpn,
            server,
            name: format!("h3:{bind_addr}"),
        })
    }

    pub fn from_pem(
        bind_addr: SocketAddr,
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
        alpn: Vec<Vec<u8>>,
        server: SovaServer,
    ) -> Result<Self> {
        let tls = sova_core::Tls::from_pem(cert_path, key_path)?;
        Self::from_tls(bind_addr, tls, alpn, server)
    }

    #[cfg(feature = "dev-tls")]
    pub fn self_signed(
        bind_addr: SocketAddr,
        hosts: &[&str],
        alpn: Vec<Vec<u8>>,
        server: SovaServer,
    ) -> Result<Self> {
        let tls = sova_core::Tls::self_signed(hosts)?;
        Self::from_tls(bind_addr, tls, alpn, server)
    }
}

impl BackgroundService for Http3Service {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(
        self: Box<Self>,
        _state: Arc<StateMap>,
        shutdown: Shutdown,
    ) -> SovaBoxFuture<()> {
        Box::pin(async move {
            let server_config = match build_server_config(&self.tls, self.alpn.clone(), false) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, "h3 server TLS config build failed");
                    return;
                }
            };

            let endpoint = match bind_server(server_config, self.bind_addr, "h3") {
                Some(ep) => ep,
                None => return,
            };

            tracing::info!(addr = %self.bind_addr, "h3 listening");

            let mut conn_tasks: JoinSet<()> = JoinSet::new();
            loop {
                tokio::select! {
                    _ = wait_shutdown(shutdown.clone()) => break,
                    incoming = endpoint.accept() => {
                        let Some(incoming) = incoming else { break; };
                        let incoming = match prepare_incoming(incoming) {
                            PrepareIncoming::RetrySent => continue,
                            PrepareIncoming::Ready(i) => *i,
                        };

                        let conn = match accept_handshake(incoming, "h3 quic handshake failed").await {
                            Some(c) => c,
                            None => continue,
                        };

                        let app_server = self.server.clone();
                        let shutdown = shutdown.clone();
                        conn_tasks.spawn(async move {
                            handle_h3_connection(conn, app_server, shutdown).await;
                        });
                    }
                }
            }

            endpoint.close(0u8.into(), b"shutdown");
            while let Some(res) = conn_tasks.join_next().await {
                if let Err(e) = res {
                    tracing::debug!(error = %e, "h3 conn task join error");
                }
            }
        })
    }
}

async fn handle_h3_connection(conn: quinn::Connection, app_server: SovaServer, shutdown: Shutdown) {
    let mut h3_conn = match h3::server::builder()
        .build(h3_quinn::Connection::new(conn))
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(error = %e, "h3 connection build failed");
            return;
        }
    };

    loop {
        tokio::select! {
            _ = wait_shutdown(shutdown.clone()) => break,
            accepted = h3_conn.accept() => {
                match accepted {
                    Ok(Some(resolver)) => {
                        let app_server = app_server.clone();
                        tokio::spawn(async move {
                            match resolver.resolve_request().await {
                                Ok((req, mut stream)) => {
                                    let mut body = Vec::new();
                                    loop {
                                        match stream.recv_data().await {
                                            Ok(Some(mut chunk)) => {
                                                while chunk.has_remaining() {
                                                    let bytes = chunk.chunk();
                                                    body.extend_from_slice(bytes);
                                                    let n = bytes.len();
                                                    chunk.advance(n);
                                                }
                                            }
                                            Ok(None) => break,
                                            Err(e) => {
                                                tracing::debug!(error = %e, "h3 recv_data failed");
                                                return;
                                            }
                                        }
                                    }

                                    let path = if let Some(q) = req.uri().query() {
                                        format!("{}?{q}", req.uri().path())
                                    } else {
                                        req.uri().path().to_string()
                                    };
                                    let mut sova_req = SovaRequest::builder()
                                        .method(req.method().clone())
                                        .path(path)
                                        .scheme("https")
                                        .host(
                                            req.headers()
                                                .get(http::header::HOST)
                                                .and_then(|v| v.to_str().ok())
                                                .unwrap_or("localhost"),
                                        )
                                        .body(Bytes::from(body))
                                        .build();
                                    for (name, value) in req.headers() {
                                        sova_req.headers.insert(name.clone(), value.clone());
                                    }

                                    let mut res = app_server.handle(sova_req).await;
                                    let status = res.status_code();
                                    let headers = res.headers().clone();
                                    let body = match res.take_body().collect().await {
                                        Ok(b) => b,
                                        Err(e) => {
                                            tracing::error!(error = %e, "h3 response body collect failed");
                                            Bytes::from_static(b"internal error")
                                        }
                                    };

                                    let mut resp_builder = http::Response::builder().status(status);
                                    for (name, value) in &headers {
                                        if name == http::header::CONNECTION
                                            || name == http::header::TRANSFER_ENCODING
                                            || name == http::header::UPGRADE
                                            || name.as_str().eq_ignore_ascii_case("keep-alive")
                                        {
                                            continue;
                                        }
                                        resp_builder = resp_builder.header(name, value);
                                    }
                                    let response = match resp_builder.body(()) {
                                        Ok(r) => r,
                                        Err(e) => {
                                            tracing::error!(error = %e, "h3 response build failed");
                                            return;
                                        }
                                    };

                                    if stream.send_response(response).await.is_err() {
                                        return;
                                    }
                                    if !body.is_empty() && stream.send_data(body).await.is_err() {
                                        return;
                                    }
                                    let _ = stream.finish().await;
                                }
                                Err(e) => tracing::debug!(error = %e, "h3 resolve request failed"),
                            }
                        });
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::debug!(error = %e, "h3 accept failed");
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SkipServerVerification;
    use h3::client;
    use sova_core::extend::StateMap;
    use sova_core::{App, Request, Response};
    use rcgen::generate_simple_self_signed;
    use std::fs;
    use tempfile::TempDir;

    fn unused_local_addr() -> std::net::SocketAddr {
        let sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind udp");
        sock.local_addr().expect("local addr")
    }

    fn write_self_signed_pem(tmp: &TempDir, subject: &[&str]) -> (std::path::PathBuf, std::path::PathBuf) {
        let subject_alt_names: Vec<String> = subject.iter().map(|s| (*s).into()).collect();
        let cert = generate_simple_self_signed(subject_alt_names).expect("rcgen");
        let cert_pem = cert.cert.pem();
        let key_pem = cert.key_pair.serialize_pem();

        let cert_path = tmp.path().join("cert.pem");
        let key_path = tmp.path().join("key.pem");
        fs::write(&cert_path, cert_pem).expect("write cert");
        fs::write(&key_path, key_pem).expect("write key");

        (cert_path, key_path)
    }

    #[tokio::test]
    async fn h3_service_dispatches_sova_routes() {
        let bind_addr = unused_local_addr();
        let client_bind = unused_local_addr();
        let h3_alpn = vec![b"h3".to_vec()];

        let tmp = TempDir::new().unwrap();
        let (cert_path, key_path) = write_self_signed_pem(&tmp, &["localhost"]);

        let mut app = App::new();
        app.get("/h3", |_r: Request| async { Response::text("h3-ok") });
        let server = app.build().unwrap();

        let svc = Http3Service::from_pem(
            bind_addr,
            &cert_path,
            &key_path,
            h3_alpn.clone(),
            server,
        )
        .unwrap();

        let (tx, shutdown) = sova_core::shutdown_channel();
        let handle = tokio::spawn(Box::new(svc).run(Arc::new(StateMap::new()), shutdown));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut client_crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();
        client_crypto.alpn_protocols = h3_alpn;
        client_crypto.enable_early_data = false;
        let client_cfg = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto).unwrap(),
        ));

        let mut endpoint = quinn::Endpoint::client(client_bind).unwrap();
        endpoint.set_default_client_config(client_cfg);
        let conn = endpoint.connect(bind_addr, "localhost").unwrap().await.unwrap();

        let (mut driver, mut send_request) = client::new(h3_quinn::Connection::new(conn))
            .await
            .unwrap();
        let drive = tokio::spawn(async move {
            let _ = std::future::poll_fn(|cx| driver.poll_close(cx)).await;
        });

        let req = http::Request::builder()
            .method(http::Method::GET)
            .uri("https://localhost/h3")
            .body(())
            .unwrap();
        let mut stream = send_request.send_request(req).await.unwrap();
        stream.finish().await.unwrap();
        let _resp = stream.recv_response().await.unwrap();
        let mut body = Vec::new();
        while let Some(mut chunk) = stream.recv_data().await.unwrap() {
            while chunk.has_remaining() {
                let bytes = chunk.chunk();
                body.extend_from_slice(bytes);
                let n = bytes.len();
                chunk.advance(n);
            }
        }
        assert_eq!(body, b"h3-ok");

        drop(send_request);
        let _ = drive.await;
        let _ = tx.send(true);
        let _ = handle.await;
    }
}
