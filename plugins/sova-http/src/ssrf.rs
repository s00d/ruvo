//! Block private / link-local / metadata addresses after DNS.

use crate::error::HttpError;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use url::Url;

const METADATA: &[&str] = &["169.254.169.254", "fd00:ec2::254"];

#[derive(Debug, Clone)]
pub struct SsrfPolicy {
    pub deny_private: bool,
    pub allow_hosts: Vec<String>,
}

impl Default for SsrfPolicy {
    fn default() -> Self {
        Self {
            deny_private: true,
            allow_hosts: Vec::new(),
        }
    }
}

impl SsrfPolicy {
    pub fn check_url(&self, raw: &str) -> Result<(), HttpError> {
        if !self.deny_private {
            return Ok(());
        }
        let url = Url::parse(raw).map_err(|e| HttpError::Ssrf(e.to_string()))?;
        let host = url
            .host_str()
            .ok_or_else(|| HttpError::Ssrf("missing host".into()))?;
        if self
            .allow_hosts
            .iter()
            .any(|h| h.eq_ignore_ascii_case(host))
        {
            return Ok(());
        }
        let port = url.port_or_known_default().unwrap_or(80);
        let addrs = (host, port)
            .to_socket_addrs()
            .map_err(|e| HttpError::Ssrf(format!("dns: {e}")))?;
        for addr in addrs {
            self.check_addr(addr)?;
        }
        Ok(())
    }

    fn check_addr(&self, addr: SocketAddr) -> Result<(), HttpError> {
        let ip = addr.ip();
        if is_blocked(ip) {
            return Err(HttpError::Ssrf(format!("blocked address {ip}")));
        }
        Ok(())
    }
}

fn is_blocked(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || (o[0] == 100 && (o[1] & 0b1100_0000) == 0b0100_0000) // 100.64/10
                || METADATA.iter().any(|m| m.parse() == Ok(IpAddr::V4(v4)))
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique local
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link-local
                || METADATA.iter().any(|m| m.parse() == Ok(IpAddr::V6(v6)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_loopback_literal() {
        let p = SsrfPolicy::default();
        assert!(p.check_url("http://127.0.0.1/").is_err());
        assert!(p.check_url("http://169.254.169.254/latest").is_err());
    }

    #[test]
    fn allow_hosts_bypasses() {
        let mut p = SsrfPolicy::default();
        p.allow_hosts.push("127.0.0.1".into());
        assert!(p.check_url("http://127.0.0.1/").is_ok());
    }
}
