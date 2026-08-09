//! `DevTools` plugin entry.

use crate::collector::{CacheLine, HttpLine, JobLine, LogLine, QueryLine, now_ms, truncate_key};
use crate::hub::DevToolsHub;
use crate::middleware;
use crate::redact::redact_sql_bindings;
use crate::routes;
use sova_core::{add_log_event_hook, App, LogRecord, Plugin, PluginMeta};
use std::sync::Arc;

/// In-app DevTools (HTML inject + SSE timeline). Disabled unless development / env.
pub struct DevTools {
    enabled: Option<bool>,
    request_cap: usize,
    log_cap: usize,
}

impl Default for DevTools {
    fn default() -> Self {
        Self::new()
    }
}

impl DevTools {
    pub fn new() -> Self {
        Self {
            enabled: None,
            request_cap: 100,
            log_cap: 500,
        }
    }

    /// Force enable/disable in **debug** builds (overrides toml).
    ///
    /// Ignored in release binaries — use `SOVA_DEVTOOLS=1` there if you must.
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = Some(on);
        self
    }

    pub fn request_cap(mut self, n: usize) -> Self {
        self.request_cap = n;
        self
    }

    pub fn log_cap(mut self, n: usize) -> Self {
        self.log_cap = n;
        self
    }
}

fn env_devtools_flag() -> Option<bool> {
    let Ok(v) = std::env::var("SOVA_DEVTOOLS") else {
        return None;
    };
    let v = v.to_ascii_lowercase();
    if matches!(v.as_str(), "1" | "true" | "yes" | "on") {
        return Some(true);
    }
    if matches!(v.as_str(), "0" | "false" | "no" | "off") {
        return Some(false);
    }
    None
}

fn is_production_profile(profile: &str) -> bool {
    matches!(
        profile.to_ascii_lowercase().as_str(),
        "production" | "release" | "prod"
    )
}

fn current_profile(app: &App) -> String {
    if let Some(p) = app
        .config_doc()
        .map(|d| d.profile.clone())
        .filter(|p| !p.is_empty())
    {
        return p;
    }
    std::env::var("SOVA_PROFILE")
        .or_else(|_| std::env::var("SOVA_ENV"))
        .unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "development".into()
            } else {
                "production".into()
            }
        })
}

/// Enable only in development by default.
///
/// **Release binaries** (`cargo build --release`): always off unless
/// `SOVA_DEVTOOLS=1` (ops escape hatch). Toml / `.enabled(true)` cannot turn
/// it on in release — avoids shipping a debug surface by accident.
///
/// **Debug binaries** with production profile: off unless `SOVA_DEVTOOLS=1`
/// or `.enabled(true)`.
fn resolve_enabled(app: &App, explicit: Option<bool>) -> bool {
    match env_devtools_flag() {
        Some(false) => return false,
        Some(true) => return true,
        None => {}
    }

    if !cfg!(debug_assertions) {
        return false;
    }

    if is_production_profile(&current_profile(app)) {
        return explicit == Some(true);
    }

    if let Some(v) = explicit {
        return v;
    }
    if let Some(section) = app.config_doc().and_then(|d| d.section("devtools")) {
        if let Some(v) = section.get("enabled").and_then(|v| v.as_bool()) {
            return v;
        }
    }
    true
}

impl Plugin for DevTools {
    fn id(&self) -> &'static str {
        "devtools"
    }

    fn meta(&self) -> PluginMeta {
        PluginMeta::new("DevTools")
            .description("In-app debug bar (HTML inject, SSE timeline, request snapshots)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        if !resolve_enabled(app, self.enabled) {
            tracing::debug!("devtools: disabled");
            return;
        }

        let hub = DevToolsHub::new(self.request_cap, self.log_cap);

        // Don't spam access logs / DevTools feed with the panel's own polling.
        sova_core::logger_skip_path("/_devtools");

        let profile = current_profile(app);
        // Plugin list is not public on App — leave empty / fill from toml later.
        hub.set_config_info(Vec::new(), profile);

        wire_log_hook(hub.clone());

        crate::hub::wire_event_bus(app, hub.clone());
        crate::hub::spawn_memory_sampler(hub.clone(), std::time::Duration::from_secs(2));

        app.state(hub.clone());
        middleware::install(app, hub.clone());
        routes::mount(app, hub);

        // CSRF / security: document that /_devtools/* is GET-only debug surface.
        tracing::info!("devtools: enabled (bar on text/html, SSE /_devtools/events)");
    }
}

fn path_is_devtools(path: &str) -> bool {
    path == "/_devtools" || path.starts_with("/_devtools/")
}

fn wire_log_hook(hub: DevToolsHub) {
    let hub = Arc::new(hub);
    add_log_event_hook(Arc::new(move |rec: LogRecord| {
        if let Some(path) = field(&rec, "path") {
            if path_is_devtools(path.trim_matches('"')) {
                return;
            }
        }

        let request_id = field(&rec, "request_id").or_else(|| {
            sova_core::current_request_id()
        });

        let target = rec.target.as_str();

        if target.starts_with("sova.store") || target.starts_with("sova.redis") {
            let op = field(&rec, "op")
                .or_else(|| field(&rec, "cmd"))
                .unwrap_or_else(|| "op".into());
            let key = field(&rec, "key")
                .or_else(|| field(&rec, "channel"))
                .or_else(|| field(&rec, "queue"))
                .unwrap_or_default();
            let hit = field(&rec, "hit").and_then(|s| match s.as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            });
            let bytes = field(&rec, "bytes").and_then(|s| s.parse().ok());
            let duration_ms = field(&rec, "duration_ms").and_then(|s| s.parse().ok());
            let ok = field(&rec, "ok").and_then(|s| match s.as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            });
            let backend = if target.starts_with("sova.redis") {
                "redis".into()
            } else {
                field(&rec, "backend").unwrap_or_else(|| "kv".into())
            };
            let line = CacheLine {
                op,
                key: truncate_key(&key, 120),
                hit,
                bytes,
                duration_ms,
                backend,
                ok,
            };
            open_bags::with_open(request_id.as_deref(), |bag| bag.push_cache(line));
            // Also mirror as a short log line.
            let msg = format!(
                "[{}] {} {}",
                target.trim_start_matches("sova."),
                field(&rec, "op").or_else(|| field(&rec, "cmd")).unwrap_or_default(),
                truncate_key(&key, 80)
            );
            let log = LogLine {
                level: rec.level.clone(),
                target: rec.target.clone(),
                message: msg,
                request_id: request_id.clone(),
                at_ms: now_ms(),
            };
            open_bags::with_open(request_id.as_deref(), |bag| bag.push_log(log.clone()));
            hub.push_log(log);
            return;
        }

        if target.starts_with("sova.tasks") {
            let name = field(&rec, "name").unwrap_or_else(|| "job".into());
            let status = field(&rec, "status").unwrap_or_else(|| rec.message.clone());
            let detail = field(&rec, "id")
                .map(|id| {
                    let q = field(&rec, "queue").unwrap_or_default();
                    if q.is_empty() {
                        id
                    } else {
                        format!("queue={q} id={id}")
                    }
                })
                .or_else(|| field(&rec, "queue"));
            let duration_ms = field(&rec, "duration_ms").and_then(|s| s.parse().ok());
            let job = JobLine {
                name,
                status,
                detail,
                duration_ms,
            };
            open_bags::with_open(request_id.as_deref(), |bag| bag.push_job(job));
            let log = LogLine {
                level: rec.level.clone(),
                target: rec.target.clone(),
                message: format!("[tasks] {}", rec.message),
                request_id: request_id.clone(),
                at_ms: now_ms(),
            };
            open_bags::with_open(request_id.as_deref(), |bag| bag.push_log(log.clone()));
            hub.push_log(log);
            return;
        }

        let is_sql = target.starts_with("sqlx::query")
            || target.contains("sea_orm")
            || rec.message.contains("SELECT")
            || rec.message.contains("INSERT")
            || rec.message.contains("UPDATE")
            || rec.message.contains("DELETE");

        let is_http_client = target.contains("http.client")
            || rec.message.contains("http.client")
            || rec.message == "http.client done"
            || rec.message == "http.client error";

        if is_sql {
            let sql = redact_sql_bindings(&rec.message);
            let duration_ms = field(&rec, "elapsed")
                .or_else(|| field(&rec, "duration_ms"))
                .and_then(|v| v.trim_matches('"').parse::<f64>().ok());
            let line = LogLine {
                level: rec.level.clone(),
                target: rec.target.clone(),
                message: format!("[sql] {sql}"),
                request_id: request_id.clone(),
                at_ms: now_ms(),
            };
            open_bags::with_open(request_id.as_deref(), |bag| bag.push_log(line.clone()));
            hub.push_log(line);
            attach_query_to_open(
                request_id.as_deref(),
                QueryLine {
                    sql,
                    duration_ms,
                    rows: None,
                },
            );
            return;
        }

        if is_http_client {
            let method = field(&rec, "http.method")
                .or_else(|| field(&rec, "method"))
                .unwrap_or_else(|| "?".into());
            let url = field(&rec, "http.url")
                .or_else(|| field(&rec, "url"))
                .or_else(|| field(&rec, "uri"))
                .unwrap_or_else(|| rec.message.clone());
            let status = field(&rec, "status").and_then(|s| s.parse().ok());
            let duration_ms = field(&rec, "duration_ms").and_then(|s| s.parse().ok());
            let error = field(&rec, "error");
            attach_http_to_open(
                request_id.as_deref(),
                HttpLine {
                    method,
                    url,
                    status,
                    duration_ms,
                    error,
                },
            );
            // Still fall through to logs for visibility.
        }

        let message = if rec.message == "request" {
            let method = field(&rec, "method").unwrap_or_else(|| "?".into());
            let path = field(&rec, "path").unwrap_or_else(|| "?".into());
            let status = field(&rec, "status").unwrap_or_else(|| "?".into());
            let ms = field(&rec, "latency_ms").unwrap_or_else(|| "?".into());
            format!("{method} {path} → {status} ({ms}ms)")
        } else {
            rec.message.clone()
        };

        let line = LogLine {
            level: rec.level,
            target: rec.target,
            message,
            request_id: request_id.clone(),
            at_ms: now_ms(),
        };
        open_bags::with_open(request_id.as_deref(), |bag| bag.push_log(line.clone()));
        hub.push_log(line);
    }));
}

fn field(rec: &LogRecord, name: &str) -> Option<String> {
    rec.fields
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.trim_matches('"').to_string())
}

/// Registry of in-flight bags by request_id for tracing hooks.
pub(crate) mod open_bags {
    use crate::collector::DevToolsBag;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static MAP: OnceLock<Mutex<HashMap<String, DevToolsBag>>> = OnceLock::new();

    fn map() -> &'static Mutex<HashMap<String, DevToolsBag>> {
        MAP.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn insert(bag: &DevToolsBag) {
        if bag.request_id == "-" {
            return;
        }
        map()
            .lock()
            .unwrap()
            .insert(bag.request_id.clone(), bag.clone());
    }

    pub fn remove(request_id: &str) {
        map().lock().unwrap().remove(request_id);
    }

    pub fn with_open(request_id: Option<&str>, f: impl FnOnce(&DevToolsBag)) {
        let Some(id) = request_id else {
            return;
        };
        let g = map().lock().unwrap();
        if let Some(bag) = g.get(id) {
            f(bag);
        }
    }
}

fn attach_query_to_open(request_id: Option<&str>, q: QueryLine) {
    open_bags::with_open(request_id, |bag| bag.push_query(q));
}

fn attach_http_to_open(request_id: Option<&str>, h: HttpLine) {
    open_bags::with_open(request_id, |bag| bag.push_http(h));
}
