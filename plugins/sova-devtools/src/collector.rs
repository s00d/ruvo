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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphqlLine {
    pub operation: String,
    pub kind: String,
    pub duration_ms: f64,
    pub errors: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GrpcLine {
    pub method: String,
    pub base: String,
    pub direction: String,
    pub duration_ms: f64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_out: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RabbitLine {
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    pub duration_ms: f64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CacheLine {
    /// `get` / `set` / `remember` / `remove` / `incr` / redis cmd
    pub op: String,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    /// `cache` | `kv` | `redis`
    pub backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RouteSnap {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub captures: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RateLimitSnap {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AuthSnap {
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
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
    pub cache: Vec<CacheLine>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graphql: Vec<GraphqlLine>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grpc: Vec<GrpcLine>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rabbit: Vec<RabbitLine>,
    pub auth: AuthSnap,
    pub route: RouteSnap,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csrf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitSnap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
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
    pub cache_count: usize,
    pub graphql_count: usize,
    pub grpc_count: usize,
    pub rabbit_count: usize,
    pub job_count: usize,
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
            cache_count: s.cache.len(),
            graphql_count: s.graphql.len(),
            grpc_count: s.grpc.len(),
            rabbit_count: s.rabbit.len(),
            job_count: s.jobs.len(),
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
    cache: Vec<CacheLine>,
    graphql: Vec<GraphqlLine>,
    grpc: Vec<GrpcLine>,
    rabbit: Vec<RabbitLine>,
    auth: AuthSnap,
    route: RouteSnap,
    locale: Option<String>,
    csrf: Option<bool>,
    rate_limit: Option<RateLimitSnap>,
    encoding: Option<String>,
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
            path: path.clone(),
            started: Instant::now(),
            inner: Arc::new(Mutex::new(BagInner {
                route: RouteSnap {
                    path,
                    ..Default::default()
                },
                ..Default::default()
            })),
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

    pub fn push_cache(&self, c: CacheLine) {
        let mut g = self.inner.lock().unwrap();
        if g.cache.len() < 200 {
            g.cache.push(c);
        }
    }

    pub fn push_graphql(&self, gql: GraphqlLine) {
        let mut g = self.inner.lock().unwrap();
        if g.graphql.len() < 50 {
            g.graphql.push(gql);
        }
    }

    pub fn push_grpc(&self, line: GrpcLine) {
        let mut g = self.inner.lock().unwrap();
        if g.grpc.len() < 50 {
            g.grpc.push(line);
        }
    }

    pub fn push_rabbit(&self, line: RabbitLine) {
        let mut g = self.inner.lock().unwrap();
        if g.rabbit.len() < 50 {
            g.rabbit.push(line);
        }
    }

    pub fn set_auth(&self, auth: AuthSnap) {
        self.inner.lock().unwrap().auth = auth;
    }

    pub fn set_route(&self, route: RouteSnap) {
        self.inner.lock().unwrap().route = route;
    }

    pub fn set_locale(&self, locale: Option<String>) {
        self.inner.lock().unwrap().locale = locale;
    }

    pub fn set_csrf(&self, present: Option<bool>) {
        self.inner.lock().unwrap().csrf = present;
    }

    pub fn set_rate_limit(&self, rl: Option<RateLimitSnap>) {
        self.inner.lock().unwrap().rate_limit = rl;
    }

    pub fn set_encoding(&self, encoding: Option<String>) {
        self.inner.lock().unwrap().encoding = encoding;
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
            cache: inner.cache.clone(),
            graphql: inner.graphql.clone(),
            grpc: inner.grpc.clone(),
            rabbit: inner.rabbit.clone(),
            auth: inner.auth.clone(),
            route: inner.route.clone(),
            locale: inner.locale.clone(),
            csrf: inner.csrf,
            rate_limit: inner.rate_limit.clone(),
            encoding: inner.encoding.clone(),
        }
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

/// Truncate keys for safe display / logs.
pub fn truncate_key(key: &str, max: usize) -> String {
    if key.len() <= max {
        key.to_string()
    } else {
        format!("{}…", &key[..max.saturating_sub(1)])
    }
}
