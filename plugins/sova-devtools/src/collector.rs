//! Per-request collector bag + finished snapshot.

use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize)]
pub struct LogLine {
    pub level: String,
    pub target: String,
    pub message: String,
    pub request_id: Option<String>,
    pub at_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryLine {
    pub sql: String,
    pub duration_ms: Option<f64>,
    pub rows: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HttpLine {
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub duration_ms: Option<f64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MailLine {
    pub to: Vec<String>,
    pub subject: String,
    pub backend: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct JobLine {
    pub name: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AuthSnap {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub session_keys: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RequestSnapshot {
    pub id: String,
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: f64,
    pub at_ms: u64,
    pub logs: Vec<LogLine>,
    pub queries: Vec<QueryLine>,
    pub http: Vec<HttpLine>,
    pub mail: Vec<MailLine>,
    pub jobs: Vec<JobLine>,
    pub auth: AuthSnap,
}

#[derive(Clone, Debug, Serialize)]
pub struct RequestMeta {
    pub id: String,
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: f64,
    pub at_ms: u64,
    pub sql_count: usize,
    pub log_errors: usize,
    pub http_count: usize,
    pub mail_count: usize,
}

impl From<&RequestSnapshot> for RequestMeta {
    fn from(s: &RequestSnapshot) -> Self {
        Self {
            id: s.id.clone(),
            request_id: s.request_id.clone(),
            method: s.method.clone(),
            path: s.path.clone(),
            status: s.status,
            duration_ms: s.duration_ms,
            at_ms: s.at_ms,
            sql_count: s.queries.len(),
            log_errors: s
                .logs
                .iter()
                .filter(|l| l.level.eq_ignore_ascii_case("ERROR") || l.level == "ERROR")
                .count(),
            http_count: s.http.len(),
            mail_count: s.mail.len(),
        }
    }
}

#[derive(Default)]
struct BagInner {
    logs: Vec<LogLine>,
    queries: Vec<QueryLine>,
    http: Vec<HttpLine>,
    mail: Vec<MailLine>,
    jobs: Vec<JobLine>,
    auth: AuthSnap,
}

/// Per-request collection bag (stored on Request extensions).
#[derive(Clone)]
pub struct DevToolsBag {
    pub id: String,
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub started: Instant,
    inner: Arc<Mutex<BagInner>>,
}

impl DevToolsBag {
    pub fn new(id: String, request_id: String, method: String, path: String) -> Self {
        Self {
            id,
            request_id,
            method,
            path,
            started: Instant::now(),
            inner: Arc::new(Mutex::new(BagInner::default())),
        }
    }

    pub fn push_log(&self, line: LogLine) {
        let mut g = self.inner.lock().unwrap();
        if g.logs.len() < 200 {
            g.logs.push(line);
        }
    }

    pub fn push_query(&self, q: QueryLine) {
        let mut g = self.inner.lock().unwrap();
        if g.queries.len() < 200 {
            g.queries.push(q);
        }
    }

    pub fn push_http(&self, h: HttpLine) {
        let mut g = self.inner.lock().unwrap();
        if g.http.len() < 100 {
            g.http.push(h);
        }
    }

    pub fn push_mail(&self, m: MailLine) {
        let mut g = self.inner.lock().unwrap();
        if g.mail.len() < 50 {
            g.mail.push(m);
        }
    }

    pub fn push_job(&self, j: JobLine) {
        let mut g = self.inner.lock().unwrap();
        if g.jobs.len() < 50 {
            g.jobs.push(j);
        }
    }

    pub fn set_auth(&self, auth: AuthSnap) {
        self.inner.lock().unwrap().auth = auth;
    }

    pub fn finish(self, status: u16) -> RequestSnapshot {
        let duration_ms = self.started.elapsed().as_secs_f64() * 1000.0;
        let at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;
        let inner = self.inner.lock().unwrap();
        RequestSnapshot {
            id: self.id,
            request_id: self.request_id,
            method: self.method,
            path: self.path,
            status,
            duration_ms,
            at_ms,
            logs: inner.logs.clone(),
            queries: inner.queries.clone(),
            http: inner.http.clone(),
            mail: inner.mail.clone(),
            jobs: inner.jobs.clone(),
            auth: inner.auth.clone(),
        }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}
