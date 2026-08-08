//! Per-host circuit breaker.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BreakerConfig {
    pub failure_threshold: u32,
    pub open_for: Duration,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            open_for: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Default)]
struct HostState {
    failures: u32,
    open_until: Option<Instant>,
}

#[derive(Debug, Default)]
pub struct CircuitBreaker {
    inner: Mutex<HashMap<String, HostState>>,
    cfg: BreakerConfig,
}

impl CircuitBreaker {
    pub fn new(cfg: BreakerConfig) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            cfg,
        }
    }

    pub fn guard(&self, host: &str) -> bool {
        let mut g = self.inner.lock().unwrap();
        let st = g.entry(host.to_string()).or_default();
        if let Some(until) = st.open_until {
            if Instant::now() < until {
                return false;
            }
            st.open_until = None;
            st.failures = 0;
        }
        true
    }

    pub fn record_success(&self, host: &str) {
        let mut g = self.inner.lock().unwrap();
        if let Some(st) = g.get_mut(host) {
            st.failures = 0;
            st.open_until = None;
        }
    }

    pub fn record_failure(&self, host: &str) {
        let mut g = self.inner.lock().unwrap();
        let st = g.entry(host.to_string()).or_default();
        st.failures = st.failures.saturating_add(1);
        if st.failures >= self.cfg.failure_threshold {
            st.open_until = Some(Instant::now() + self.cfg.open_for);
        }
    }
}
