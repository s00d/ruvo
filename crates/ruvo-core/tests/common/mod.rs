//! Shared helpers for integration tests (real TCP listener).

use bytes::Bytes;
use ruvo_core::{App, Bind};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

pub struct LiveServer {
    pub addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<tokio::task::JoinHandle<ruvo_core::Result<()>>>,
}

impl LiveServer {
    pub async fn spawn(app: App) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = oneshot::channel::<()>();
        let join = tokio::spawn(async move {
            app.bind(Bind::Listener(listener))
            .shutdown(async move {
                let _ = rx.await;
            })
            .serve()
            .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        Self {
            addr,
            shutdown: Some(tx),
            join: Some(join),
        }
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), join).await;
        }
    }
}

/// Minimal HTTP/1.1 GET over TCP (Connection: close).
pub async fn http_get(addr: SocketAddr, path: &str) -> (u16, Bytes) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    parse_response(&buf)
}

fn parse_response(buf: &[u8]) -> (u16, Bytes) {
    let text = String::from_utf8_lossy(buf);
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = match find_body(buf) {
        Some(b) => Bytes::copy_from_slice(b),
        None => Bytes::new(),
    };
    (status, body)
}

fn find_body(buf: &[u8]) -> Option<&[u8]> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| &buf[i + 4..])
}
