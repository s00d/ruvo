//! In-memory sliding-window rate limiter for Ruvo.

use ruvo_core::extend::named;
use ruvo_core::{with_state, App, ClientAddr, Plugin, Response};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Simple in-memory sliding-window rate limiter.
pub struct RateLimit {
    max: usize,
    window: Duration,
    max_entries: usize,
}

impl RateLimit {
    pub fn new(max: usize, window: Duration) -> Self {
        Self {
            max,
            window,
            max_entries: 10_000,
        }
    }

    pub fn per_minute(max: usize) -> Self {
        Self::new(max, Duration::from_secs(60))
    }

    /// Cap distinct client keys retained (default 10_000).
    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = n.max(1);
        self
    }
}

impl Plugin for RateLimit {
    fn install(self, app: &mut App) {
        let max_entries = self.max_entries;
        app.use_middleware(named(
            "rate-limit",
            with_state(
                SlidingWindow::new(self.max, self.window, max_entries),
                |state, req, next| async move {
                    let ip = req
                        .get::<ClientAddr>()
                        .map(|a| a.0.ip())
                        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));
                    if !state.allow(ip) {
                        return Response::text("Too Many Requests").status(429);
                    }
                    next(req).await
                },
            ),
        ));
    }
}

struct SlidingWindow {
    inner: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
    max: usize,
    window: Duration,
    max_entries: usize,
}

impl SlidingWindow {
    fn new(max: usize, window: Duration, max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max,
            window,
            max_entries,
        }
    }

    fn allow(&self, ip: IpAddr) -> bool {
        let mut map = self.inner.lock().unwrap();
        let now = Instant::now();

        if let Some(entries) = map.get_mut(&ip) {
            entries.retain(|t| now.duration_since(*t) < self.window);
            if entries.is_empty() {
                map.remove(&ip);
            }
        }

        if map.len() >= self.max_entries && !map.contains_key(&ip) {
            map.retain(|_, v| {
                v.retain(|t| now.duration_since(*t) < self.window);
                !v.is_empty()
            });
            while map.len() >= self.max_entries {
                if let Some(k) = map.keys().next().cloned() {
                    map.remove(&k);
                } else {
                    break;
                }
            }
        }

        let entries = map.entry(ip).or_default();
        entries.retain(|t| now.duration_since(*t) < self.window);
        if entries.len() >= self.max {
            return false;
        }
        entries.push(now);
        true
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn max_entries_caps_distinct_ips() {
        let w = SlidingWindow::new(100, Duration::from_secs(60), 3);
        assert!(w.allow(IpAddr::V4(Ipv4Addr::new(1, 0, 0, 1))));
        assert!(w.allow(IpAddr::V4(Ipv4Addr::new(1, 0, 0, 2))));
        assert!(w.allow(IpAddr::V4(Ipv4Addr::new(1, 0, 0, 3))));
        assert_eq!(w.len(), 3);
        assert!(w.allow(IpAddr::V4(Ipv4Addr::new(1, 0, 0, 4))));
        assert!(w.len() <= 3);
    }

    #[test]
    fn rate_blocks_after_max() {
        let w = SlidingWindow::new(2, Duration::from_secs(60), 10);
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        assert!(w.allow(ip));
        assert!(w.allow(ip));
        assert!(!w.allow(ip));
    }
}
