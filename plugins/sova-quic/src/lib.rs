//! QUIC datagrams (QUIC + TLS 1.3). No DTLS and no HTTP/3 streams — only
//! unreliable/unordered application datagrams.

mod config;
mod datagram;
mod endpoint;
mod stream;

pub use datagram::{QuicDatagramClient, QuicDatagramHandler, QuicDatagramService};
pub use stream::Http3Service;
