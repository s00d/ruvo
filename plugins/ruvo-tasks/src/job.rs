//! Job definition: handler + optional schedule / queue / priority.

use crate::{BoxFuture, Handler};
use cron::Schedule as CronSchedule;
use ruvo_tasks_store::Task;
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

/// Common priority levels (higher is claimed first).
pub mod priority {
    pub const LOW: i32 = -100;
    pub const NORMAL: i32 = 0;
    pub const HIGH: i32 = 100;
}

/// When a [`Job`] should be enqueued by the scheduler.
#[derive(Clone, Debug)]
pub enum Schedule {
    Every(Duration),
    Cron(CronSchedule),
}

/// Named handler with optional recurring schedule, queue, and priority.
pub struct Job {
    pub(crate) name: String,
    pub(crate) handler: Handler,
    pub(crate) schedule: Option<Schedule>,
    pub(crate) payload: Value,
    pub(crate) queue: Option<String>,
    pub(crate) priority: i32,
}

impl Job {
    pub fn new<F, Fut>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(Task) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        Self {
            name: name.into(),
            handler: Arc::new(move |t| Box::pin(f(t)) as BoxFuture<Result<(), String>>),
            schedule: None,
            payload: Value::Object(Default::default()),
            queue: None,
            priority: priority::NORMAL,
        }
    }

    /// Named queue for schedule enqueue and default dispatch routing.
    pub fn queue(mut self, q: impl Into<String>) -> Self {
        self.queue = Some(q.into());
        self
    }

    /// Claim priority within the queue (higher first). Default [`priority::NORMAL`].
    pub fn priority(mut self, p: i32) -> Self {
        self.priority = p;
        self
    }

    /// Run approximately every `period` (slot-based dedup across instances).
    pub fn every(mut self, period: Duration) -> Self {
        assert!(!period.is_zero(), "Job::every period must be non-zero");
        self.schedule = Some(Schedule::Every(period));
        self
    }

    /// Cron expression: 5 fields (`min hour day month dow`) or 6 (`sec min …`).
    pub fn cron(mut self, expr: impl AsRef<str>) -> Self {
        let sched = parse_cron(expr.as_ref())
            .unwrap_or_else(|e| panic!("Job::cron({}): {e}", expr.as_ref()));
        self.schedule = Some(Schedule::Cron(sched));
        self
    }

    /// JSON `data` field used when the scheduler enqueues this job.
    pub fn payload(mut self, data: Value) -> Self {
        self.payload = data;
        self
    }
}

/// Pad 5-field crontab to 6-field (seconds = 0) for the `cron` crate.
pub fn parse_cron(expr: &str) -> Result<CronSchedule, String> {
    let trimmed = expr.trim();
    let fields = trimmed.split_whitespace().count();
    let padded = if fields == 5 {
        format!("0 {trimmed}")
    } else {
        trimmed.to_string()
    };
    CronSchedule::from_str(&padded).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_five_field_cron() {
        let s = parse_cron("0 * * * *").unwrap();
        assert!(s.upcoming(chrono::Utc).next().is_some());
    }

    #[test]
    #[should_panic(expected = "Job::cron")]
    fn bad_cron_panics() {
        let _ = Job::new("x", |_| async { Ok(()) }).cron("not a cron");
    }
}
