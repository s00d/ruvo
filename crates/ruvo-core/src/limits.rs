//! Route-scoped limits: [`MaxBody`], [`RequestTimeout`], [`Deadline`].

use crate::human::{parse_bytes, parse_duration};
use crate::route_value::RouteValue;
use std::borrow::Cow;
use std::time::{Duration, Instant};

/// Max request body size for a route / router / app scope.
#[derive(Debug, Clone, Copy)]
pub struct MaxBody(pub usize);

impl MaxBody {
    pub fn bytes(n: usize) -> Self {
        Self(n)
    }

    pub fn kib(n: usize) -> Self {
        Self(n.saturating_mul(1024))
    }

    pub fn mib(n: usize) -> Self {
        Self(n.saturating_mul(1024 * 1024))
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        Ok(Self(parse_bytes(s)?))
    }
}

impl RouteValue for MaxBody {
    fn label(&self) -> Cow<'static, str> {
        Cow::Owned(format!("MaxBody({} bytes)", self.0))
    }
}

/// Per-route request timeout (inner; app-level timeout in serve still applies).
#[derive(Debug, Clone, Copy)]
pub struct RequestTimeout(pub Duration);

impl RequestTimeout {
    pub fn from_secs(secs: u64) -> Self {
        Self(Duration::from_secs(secs))
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        Ok(Self(parse_duration(s)?))
    }
}

impl RouteValue for RequestTimeout {
    fn label(&self) -> Cow<'static, str> {
        Cow::Owned(format!("RequestTimeout({:?})", self.0))
    }
}

/// Absolute instant when the request budget expires (app and/or route timeout).
///
/// Set by the server / router so outbound clients can use [`Self::remaining`].
#[derive(Debug, Clone, Copy)]
pub struct Deadline(pub Instant);

impl Deadline {
    pub fn at(instant: Instant) -> Self {
        Self(instant)
    }

    pub fn after(dur: Duration) -> Self {
        Self(Instant::now() + dur)
    }

    /// Time left until the deadline; `Duration::ZERO` if already past.
    pub fn remaining(&self) -> Duration {
        self.0.saturating_duration_since(Instant::now())
    }
}

/// Keep the earlier (stricter) deadline on `req`.
pub fn tighten_deadline(req: &mut crate::Request, until: Instant) {
    match req.get::<Deadline>() {
        Some(d) if d.0 <= until => {}
        _ => req.set(Deadline(until)),
    }
}
