use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use sova_core::{Error, Result};
use std::sync::Arc;

/// Insecure verifier for local/dev QUIC clients (tests, self-signed certs).
#[derive(Debug)]
pub(crate) struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

pub(crate) fn build_server_config(
    tls: &sova_core::Tls,
    alpn: Vec<Vec<u8>>,
    datagram_only: bool,
) -> Result<quinn::ServerConfig> {
    let rustls_config = tls.build_quic_server_config(alpn);
    let quic_crypto = QuicServerConfig::try_from(rustls_config)
        .map_err(|e| Error::Internal(format!("quic server TLS config build failed: {e}")))?;

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(quic_crypto));
    if datagram_only {
        if let Some(transport_config) = Arc::get_mut(&mut server_config.transport) {
            // Datagram-only service: disallow streams to reduce memory pressure.
            transport_config.max_concurrent_bidi_streams(0_u8.into());
            transport_config.max_concurrent_uni_streams(0_u8.into());
        }
    }
    Ok(server_config)
}

pub(crate) fn build_client_config(alpn: Vec<Vec<u8>>) -> Result<quinn::ClientConfig> {
    let mut client_crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();
    client_crypto.enable_early_data = false;
    client_crypto.alpn_protocols = alpn;

    let quic_crypto = QuicClientConfig::try_from(client_crypto)
        .map_err(|e| Error::Internal(format!("quic client TLS config build failed: {e}")))?;
    Ok(quinn::ClientConfig::new(Arc::new(quic_crypto)))
}
