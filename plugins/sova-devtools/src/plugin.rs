//! `DevTools` plugin entry.

use crate::collector::{HttpLine, LogLine, QueryLine, now_ms};
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
        // Drop access-log noise about the DevTools UI itself (even if logger skip missed it).
        if let Some(path) = field(&rec, "path") {
            if path_is_devtools(path.trim_matches('"')) {
                return;
            }
        }

        let request_id = rec
            .fields
            .iter()
            .find(|(k, _)| k == "request_id")
            .map(|(_, v)| v.trim_matches('"').to_string());

        let target = rec.target.as_str();
        let is_sql = target.starts_with("sqlx::query")
            || target.contains("sea_orm")
            || rec.message.contains("SELECT")
            || rec.message.contains("INSERT")
            || rec.message.contains("UPDATE")
            || rec.message.contains("DELETE");

        let is_http_client =
            target.contains("http.client") || rec.message.contains("http.client");

        if is_sql {
            let sql = redact_sql_bindings(&rec.message);
            let duration_ms = rec
                .fields
                .iter()
                .find(|(k, _)| k == "elapsed" || k.contains("time"))
                .and_then(|(_, v)| v.trim_matches('"').parse::<f64>().ok());
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
                &hub,
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
            let method = field(&rec, "method").unwrap_or_else(|| "?".into());
            let url = field(&rec, "uri")
                .or_else(|| field(&rec, "url"))
                .unwrap_or_else(|| rec.message.clone());
            let status = field(&rec, "status").and_then(|s| s.parse().ok());
            attach_http_to_open(
                &hub,
                request_id.as_deref(),
                HttpLine {
                    method,
                    url,
                    status,
                    duration_ms: None,
                    error: None,
                },
            );
        }

        // Format request access lines more readably in the panel.
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
        // Per-request Logs tab + site-wide /_devtools/logs feed.
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

fn attach_query_to_open(_hub: &DevToolsHub, request_id: Option<&str>, q: QueryLine) {
    open_bags::with_open(request_id, |bag| bag.push_query(q));
}

fn attach_http_to_open(_hub: &DevToolsHub, request_id: Option<&str>, h: HttpLine) {
    open_bags::with_open(request_id, |bag| bag.push_http(h));
}
