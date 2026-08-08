use std::net::SocketAddr;

pub(super) fn forwarded_addr(headers: &http::HeaderMap) -> Option<SocketAddr> {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        let first = xff.split(',').next()?.trim();
        if let Ok(ip) = first.parse::<std::net::IpAddr>() {
            return Some(SocketAddr::new(ip, 0));
        }
    }
    if let Some(fwd) = headers.get("forwarded").and_then(|v| v.to_str().ok()) {
        for part in fwd.split(';') {
            let part = part.trim();
            if let Some(rest) = part
                .strip_prefix("for=")
                .or_else(|| part.strip_prefix("For="))
            {
                let rest = rest.trim_matches('"');
                let host = rest.split(',').next()?.trim();
                let host = host.trim_start_matches('[').trim_end_matches(']');
                if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                    return Some(SocketAddr::new(ip, 0));
                }
            }
        }
    }
    None
}
