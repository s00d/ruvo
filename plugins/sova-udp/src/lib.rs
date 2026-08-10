//! UDP listeners as [`BackgroundService`](sova_core::BackgroundService).

use sova_core::extend::{wait_shutdown, BoxFuture, StateMap};
use sova_core::{BackgroundService, Shutdown};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

pub type UdpHandler =
    Arc<dyn Fn(SocketAddr, Vec<u8>, Arc<UdpSocket>) -> BoxFuture<()> + Send + Sync>;

/// Bind a UDP socket and invoke `handler` for each datagram until shutdown.
pub struct UdpService {
    addr: SocketAddr,
    handler: UdpHandler,
    name: String,
}

impl UdpService {
    pub fn new<F, Fut>(addr: SocketAddr, handler: F) -> Self
    where
        F: Fn(SocketAddr, Vec<u8>, Arc<UdpSocket>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        Self {
            addr,
            handler: Arc::new(move |a, b, s| Box::pin(handler(a, b, s))),
            name: format!("udp:{addr}"),
        }
    }

    pub fn echo(addr: SocketAddr) -> Self {
        Self::new(addr, |_peer, data, sock| async move {
            let _ = sock.send_to(&data, _peer).await;
        })
    }
}

impl BackgroundService for UdpService {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(self: Box<Self>, _state: Arc<StateMap>, shutdown: Shutdown) -> BoxFuture<()> {
        Box::pin(async move {
            let sock = match UdpSocket::bind(self.addr).await {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    tracing::error!(error = %e, addr = %self.addr, "udp bind failed");
                    return;
                }
            };
            tracing::info!(addr = %self.addr, "udp listening");
            let mut buf = vec![0u8; 65535];
            loop {
                tokio::select! {
                    _ = wait_shutdown(shutdown.clone()) => break,
                    res = sock.recv_from(&mut buf) => {
                        match res {
                            Ok((n, peer)) => {
                                let data = buf[..n].to_vec();
                                (self.handler)(peer, data, Arc::clone(&sock)).await;
                            }
                            Err(e) => {
                                tracing::debug!(error = %e, "udp recv");
                                break;
                            }
                        }
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn echo_roundtrip() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let sock = UdpSocket::bind(addr).await.unwrap();
        let addr = sock.local_addr().unwrap();
        drop(sock);

        let (tx, shutdown) = sova_core::shutdown_channel();
        let svc = Box::new(UdpService::echo(addr));
        let handle = tokio::spawn(svc.run(Arc::new(StateMap::new()), shutdown));

        tokio::time::sleep(Duration::from_millis(50)).await;
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(b"ping", addr).await.unwrap();
        let mut buf = [0u8; 16];
        let (n, _) = tokio::time::timeout(Duration::from_secs(1), client.recv_from(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&buf[..n], b"ping");
        let _ = tx.send(true);
        let _ = handle.await;
    }

    #[tokio::test]
    async fn shutdown_stops_recv_loop() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let sock = UdpSocket::bind(addr).await.unwrap();
        let addr = sock.local_addr().unwrap();
        drop(sock);

        let (tx, shutdown) = sova_core::shutdown_channel();
        let svc = Box::new(UdpService::echo(addr));
        let handle = tokio::spawn(svc.run(Arc::new(StateMap::new()), shutdown));

        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = tx.send(true);
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("service did not stop after shutdown")
            .expect("join error");
    }

    #[tokio::test]
    async fn max_datagram_size_accepted() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let sock = UdpSocket::bind(addr).await.unwrap();
        let addr = sock.local_addr().unwrap();
        drop(sock);

        let (tx, shutdown) = sova_core::shutdown_channel();
        let svc = Box::new(UdpService::echo(addr));
        let handle = tokio::spawn(svc.run(Arc::new(StateMap::new()), shutdown));

        tokio::time::sleep(Duration::from_millis(50)).await;
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let payload = vec![0xABu8; 8192];
        client.send_to(&payload, addr).await.unwrap();
        let mut buf = vec![0u8; 8192];
        let (n, _) = tokio::time::timeout(Duration::from_secs(1), client.recv_from(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&buf[..n], &payload[..]);
        let _ = tx.send(true);
        let _ = handle.await;
    }
}
