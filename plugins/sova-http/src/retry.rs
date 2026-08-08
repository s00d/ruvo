//! Exponential backoff with full jitter.

use std::time::Duration;

/// Delay before attempt `attempt` (1-based after first failure): `random(0, min(cap, base * 2^(n-1)))`.
pub fn full_jitter(base: Duration, cap: Duration, attempt: u32) -> Duration {
    let exp = attempt.saturating_sub(1).min(16);
    let max_ms = (base.as_millis() as u64)
        .saturating_mul(1u64 << exp)
        .min(cap.as_millis() as u64)
        .max(1);
    let jitter = fastrand_u64(max_ms);
    Duration::from_millis(jitter)
}

fn fastrand_u64(max: u64) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut h = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut h);
    max.hash(&mut h);
    h.finish() % max
}

pub fn parse_retry_after(headers: &http::HeaderMap) -> Option<Duration> {
    let raw = headers.get(http::header::RETRY_AFTER)?.to_str().ok()?;
    if let Ok(secs) = raw.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    None
}

pub fn method_idempotent(method: &http::Method) -> bool {
    matches!(
        *method,
        http::Method::GET | http::Method::HEAD | http::Method::PUT | http::Method::DELETE
    )
}
