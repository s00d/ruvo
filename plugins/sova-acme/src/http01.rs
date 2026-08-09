//! HTTP-01 challenge map + plain :80 listener.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::watch;

/// In-memory HTTP-01 token → key authorization.
#[derive(Clone, Default)]
pub struct ChallengeMap {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl ChallengeMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, token: impl Into<String>, key_auth: impl Into<String>) {
        self.inner
            .lock()
            .expect("challenge map")
            .insert(token.into(), key_auth.into());
    }

    pub fn remove(&self, token: &str) {
        self.inner.lock().expect("challenge map").remove(token);
    }

    pub fn get(&self, token: &str) -> Option<String> {
        self.inner.lock().expect("challenge map").get(token).cloned()
    }
}

pub async fn run_http01_listener(
    port: u16,
    challenges: ChallengeMap,
    redirect_https: bool,
    https_port: u16,
    mut shutdown: watch::Receiver<bool>,
) {
    let bind = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match TcpListener::bind(bind).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, port, "acme: HTTP-01 listener failed to bind");
            return;
        }
    };
    tracing::info!(port, "acme: HTTP-01 listening");

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() { break; }
            }
            accepted = listener.accept() => {
                let Ok((mut stream, _)) = accepted else { continue; };
                let challenges = challenges.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let n = match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => n,
                    };
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let line = req.lines().next().unwrap_or("");
                    let mut parts = line.split_whitespace();
                    let method = parts.next().unwrap_or("");
                    let path = parts.next().unwrap_or("");

                    let response = if method == "GET" {
                        if let Some(token) = path.strip_prefix("/.well-known/acme-challenge/") {
                            let token = token.split('?').next().unwrap_or(token);
                            if let Some(body) = challenges.get(token) {
                                format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                    body.len()
                                )
                            } else {
                                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
                            }
                        } else if redirect_https {
                            let host = req
                                .lines()
                                .find_map(|l| l.strip_prefix("Host:").or_else(|| l.strip_prefix("host:")))
                                .map(|h| h.trim())
                                .unwrap_or("localhost");
                            let host = host.split(':').next().unwrap_or(host);
                            let location = if https_port == 443 {
                                format!("https://{host}{path}")
                            } else {
                                format!("https://{host}:{https_port}{path}")
                            };
                            format!(
                                "HTTP/1.1 301 Moved Permanently\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            )
                        } else {
                            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
                        }
                    } else {
                        "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
                    };
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        }
    }
}
