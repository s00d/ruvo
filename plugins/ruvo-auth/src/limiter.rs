//! Thin in-memory rate limiters for Fortify auth POSTs.

use ruvo_core::extend::{named, MwEntry};
use ruvo_core::{with_state, ClientAddr, IntoResponse, Response};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Window {
    bucket: &'static str,
    max: usize,
    window: Duration,
    hits: Mutex<HashMap<String, Vec<Instant>>>,
}

/// Per-route limiter: key = `{bucket}:{ip}`.
pub fn limiter(bucket: &'static str, max: usize, window: Duration) -> MwEntry {
    let state = Arc::new(Window {
        bucket,
        max,
        window,
        hits: Mutex::new(HashMap::new()),
    });
    named(
        format!("fortify-limit:{bucket}"),
        with_state(state, |win, req, next| async move {
            let ip = req
                .get::<ClientAddr>()
                .map(|a| a.0.ip())
                .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));
            let key = format!("{}:{ip}", win.bucket);
            let now = Instant::now();
            let allowed = {
                let mut map = win.hits.lock().unwrap_or_else(|e| e.into_inner());
                let entry = map.entry(key).or_default();
                entry.retain(|t| now.duration_since(*t) < win.window);
                if entry.len() >= win.max {
                    false
                } else {
                    entry.push(now);
                    true
                }
            };
            if allowed {
                next(req).await
            } else {
                Response::text("Too Many Requests").status(429).into_response()
            }
        }),
    )
}

pub fn login_limiter() -> MwEntry {
    limiter("login", 5, Duration::from_secs(60))
}

pub fn forgot_limiter() -> MwEntry {
    limiter("forgot", 5, Duration::from_secs(60))
}

pub fn challenge_limiter() -> MwEntry {
    limiter("2fa", 5, Duration::from_secs(60))
}

pub fn resend_limiter() -> MwEntry {
    limiter("verify-resend", 6, Duration::from_secs(60))
}
