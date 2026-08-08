/// Anti-amplification RETRY was sent; caller should accept again.
pub(crate) enum PrepareIncoming {
    Ready(Box<quinn::Incoming>),
    RetrySent,
}

pub(crate) fn prepare_incoming(incoming: quinn::Incoming) -> PrepareIncoming {
    if incoming.may_retry() {
        match incoming.retry() {
            Ok(()) => PrepareIncoming::RetrySent,
            Err(e) => PrepareIncoming::Ready(Box::new(e.into_incoming())),
        }
    } else {
        PrepareIncoming::Ready(Box::new(incoming))
    }
}

pub(crate) async fn accept_handshake(
    incoming: quinn::Incoming,
    fail_msg: &'static str,
) -> Option<quinn::Connection> {
    match incoming.await {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::debug!(error = %e, "{fail_msg}");
            None
        }
    }
}

pub(crate) fn bind_server(
    server_config: quinn::ServerConfig,
    bind_addr: std::net::SocketAddr,
    label: &str,
) -> Option<quinn::Endpoint> {
    match quinn::Endpoint::server(server_config, bind_addr) {
        Ok(ep) => Some(ep),
        Err(e) => {
            tracing::error!(error = %e, addr = %bind_addr, "{label} endpoint bind failed");
            None
        }
    }
}
