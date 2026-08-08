//! Load app-level settings from `ruvo.toml` (no per-route addressing).
//!
//! # Document shape
//!
//! ```toml
//! [server]
//! max_body = "2mb"
//! trust_proxy = false
//!
//! [mail]
//! from = "App <noreply@example.com>"
//!
//! [development.server]
//! # profile overlay
//!
//! [production.server]
//! trust_proxy = true
//! ```
//!
//! Active profile: `RUVO_PROFILE` → else `RUVO_ENV` → else
//! `development` (debug builds) / `production` (release). Aliases:
//! `debug`→`development`, `release`→`production`.
//!
//! Legacy: server keys under `[default]` / `[debug]` / `[release]`, and
//! `[default.mail]` instead of top-level `[mail]`, still work.

use crate::app::App;
use crate::error::{Error, Result};
use crate::human::{parse_bytes, parse_duration};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// Full parsed `ruvo.toml` (raw root for plugins; profile name for overlays).
#[derive(Debug, Clone)]
pub struct ConfigDoc {
    /// Original root table (`server`, `mail`, profile names, …).
    pub root: toml::Value,
    /// Active profile (`development` / `production` / `test` / custom).
    pub profile: String,
}

impl ConfigDoc {
    /// Merge `[section]` + `[default.section]` + `[<profile>.section]` (shallow).
    pub fn section(&self, section: &str) -> Option<toml::map::Map<String, toml::Value>> {
        let mut out = toml::map::Map::new();
        if let Some(toml::Value::Table(t)) = self.root.get(section) {
            // Top-level `[mail]` etc. — skip if this table looks like a profile
            // container with only nested section tables and no scalar keys for
            // leaf sections. For leaf sections (mail, storage, …) take as-is.
            // `[server]` is a leaf; `[http]` is nested clients (handled separately).
            if section != "http" {
                out.extend(t.clone());
            }
        }
        if let Some(toml::Value::Table(t)) = self
            .root
            .get("default")
            .and_then(|d| d.get(section))
        {
            out.extend(t.clone());
        }
        if let Some(toml::Value::Table(t)) = self
            .root
            .get(&self.profile)
            .and_then(|d| d.get(section))
        {
            out.extend(t.clone());
        }
        // Legacy profile aliases: also merge debug/release overlays when active
        // profile is development/production (in case file still uses old names).
        for alias in profile_aliases_for_merge(&self.profile) {
            if let Some(toml::Value::Table(t)) =
                self.root.get(alias).and_then(|d| d.get(section))
            {
                out.extend(t.clone());
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// Named client tables under `http` (`[http.payments]` / `[default.http.payments]`).
    pub fn http_clients(&self) -> HashMap<String, toml::map::Map<String, toml::Value>> {
        let mut out = HashMap::new();
        let merge_into = |dest: &mut HashMap<String, toml::map::Map<String, toml::Value>>,
                          table: &toml::map::Map<String, toml::Value>| {
            for (name, val) in table {
                if let toml::Value::Table(t) = val {
                    dest.entry(name.clone())
                        .and_modify(|e| e.extend(t.clone()))
                        .or_insert_with(|| t.clone());
                }
            }
        };
        if let Some(toml::Value::Table(http)) = self.root.get("http") {
            merge_into(&mut out, http);
        }
        if let Some(toml::Value::Table(http)) = self.root.get("default").and_then(|d| d.get("http"))
        {
            merge_into(&mut out, http);
        }
        if let Some(toml::Value::Table(http)) =
            self.root.get(&self.profile).and_then(|d| d.get("http"))
        {
            merge_into(&mut out, http);
        }
        for alias in profile_aliases_for_merge(&self.profile) {
            if let Some(toml::Value::Table(http)) =
                self.root.get(alias).and_then(|d| d.get("http"))
            {
                merge_into(&mut out, http);
            }
        }
        out
    }
}

/// Extra profile table names to merge when the active profile is the modern name.
fn profile_aliases_for_merge(profile: &str) -> &'static [&'static str] {
    match profile {
        "development" => &["debug"],
        "production" => &["release"],
        _ => &[],
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
struct ServerProfile {
    max_body: Option<String>,
    max_connections: Option<usize>,
    max_upgraded_connections: Option<usize>,
    max_concurrent_streams: Option<usize>,
    max_headers: Option<usize>,
    max_buf_size: Option<String>,
    request_timeout: Option<String>,
    header_read_timeout: Option<String>,
    idle_timeout: Option<String>,
    drain_timeout: Option<String>,
    keep_alive: Option<bool>,
    trust_proxy: Option<bool>,
}

fn normalize_profile_name(raw: &str) -> String {
    match raw.trim() {
        "debug" => "development".into(),
        "release" => "production".into(),
        other => other.to_string(),
    }
}

fn active_profile() -> String {
    if let Ok(p) = std::env::var("RUVO_PROFILE") {
        return normalize_profile_name(&p);
    }
    if let Ok(p) = std::env::var("RUVO_ENV") {
        return normalize_profile_name(&p);
    }
    if cfg!(debug_assertions) {
        "development".into()
    } else {
        "production".into()
    }
}

fn merge_server(base: ServerProfile, over: ServerProfile) -> ServerProfile {
    ServerProfile {
        max_body: over.max_body.or(base.max_body),
        max_connections: over.max_connections.or(base.max_connections),
        max_upgraded_connections: over
            .max_upgraded_connections
            .or(base.max_upgraded_connections),
        max_concurrent_streams: over
            .max_concurrent_streams
            .or(base.max_concurrent_streams),
        max_headers: over.max_headers.or(base.max_headers),
        max_buf_size: over.max_buf_size.or(base.max_buf_size),
        request_timeout: over.request_timeout.or(base.request_timeout),
        header_read_timeout: over.header_read_timeout.or(base.header_read_timeout),
        idle_timeout: over.idle_timeout.or(base.idle_timeout),
        drain_timeout: over.drain_timeout.or(base.drain_timeout),
        keep_alive: over.keep_alive.or(base.keep_alive),
        trust_proxy: over.trust_proxy.or(base.trust_proxy),
    }
}

fn table_to_server(table: &toml::map::Map<String, toml::Value>) -> ServerProfile {
    // Re-serialize subset so we can reuse Deserialize (ignore unknown keys).
    let mut filtered = toml::map::Map::new();
    for key in [
        "max_body",
        "max_connections",
        "max_upgraded_connections",
        "max_concurrent_streams",
        "max_headers",
        "max_buf_size",
        "request_timeout",
        "header_read_timeout",
        "idle_timeout",
        "drain_timeout",
        "keep_alive",
        "trust_proxy",
    ] {
        if let Some(v) = table.get(key) {
            filtered.insert(key.to_string(), v.clone());
        }
    }
    toml::Value::Table(filtered)
        .try_into()
        .unwrap_or_default()
}

/// Resolve merged `[server]` from canon + legacy layouts.
fn resolve_server(root: &toml::Value, profile: &str) -> ServerProfile {
    let mut merged = ServerProfile::default();

    // Canon: [server]
    if let Some(toml::Value::Table(t)) = root.get("server") {
        merged = merge_server(merged, table_to_server(t));
    }

    // Legacy: flat server keys under [default]
    if let Some(toml::Value::Table(t)) = root.get("default") {
        // Prefer nested [default.server] if present; else treat flat keys as server.
        if let Some(toml::Value::Table(server)) = t.get("server") {
            merged = merge_server(merged, table_to_server(server));
        } else {
            merged = merge_server(merged, table_to_server(t));
        }
    }

    // Profile overlays: [development.server], [production.server], …
    let mut names = vec![profile];
    names.extend(profile_aliases_for_merge(profile).iter().copied());
    for name in names {
        if let Some(toml::Value::Table(prof)) = root.get(name) {
            if let Some(toml::Value::Table(server)) = prof.get("server") {
                merged = merge_server(merged, table_to_server(server));
            } else {
                // Legacy: flat keys under [debug] / [release] / [development]
                merged = merge_server(merged, table_to_server(prof));
            }
        }
    }

    merged
}

fn apply_server(app: &mut App, p: &ServerProfile) -> Result<()> {
    if let Some(ref s) = p.max_body {
        app.max_body_size(parse_bytes(s).map_err(Error::Internal)?);
    }
    if let Some(n) = p.max_connections {
        app.max_connections(n);
    }
    if let Some(n) = p.max_upgraded_connections {
        app.max_upgraded_connections(n);
    }
    if let Some(n) = p.max_concurrent_streams {
        app.max_concurrent_streams(n);
    }
    if let Some(n) = p.max_headers {
        app.max_headers(n);
    }
    if let Some(ref s) = p.max_buf_size {
        app.max_buf_size(parse_bytes(s).map_err(Error::Internal)?);
    }
    if let Some(ref s) = p.request_timeout {
        if s.eq_ignore_ascii_case("off") || s.eq_ignore_ascii_case("none") {
            app.request_timeout(None);
        } else {
            app.request_timeout(Some(parse_duration(s).map_err(Error::Internal)?));
        }
    }
    if let Some(ref s) = p.header_read_timeout {
        app.header_read_timeout(parse_duration(s).map_err(Error::Internal)?);
    }
    if let Some(ref s) = p.idle_timeout {
        app.idle_timeout(parse_duration(s).map_err(Error::Internal)?);
    }
    if let Some(ref s) = p.drain_timeout {
        app.drain_timeout(parse_duration(s).map_err(Error::Internal)?);
    }
    if let Some(v) = p.keep_alive {
        app.keep_alive(v);
    }
    if let Some(v) = p.trust_proxy {
        app.trust_proxy(v);
    }
    Ok(())
}

fn parse_timeout_env(s: &str) -> Result<Option<Duration>> {
    if s.eq_ignore_ascii_case("off") || s.eq_ignore_ascii_case("none") {
        Ok(None)
    } else {
        Ok(Some(parse_duration(s).map_err(Error::Internal)?))
    }
}

fn env_override(app: &mut App) -> Result<()> {
    if let Ok(s) = std::env::var("RUVO_MAX_BODY") {
        app.max_body_size(parse_bytes(&s).map_err(Error::Internal)?);
    }
    if let Ok(s) = std::env::var("RUVO_MAX_CONNECTIONS") {
        let n: usize = s
            .parse()
            .map_err(|_| Error::Internal(format!("RUVO_MAX_CONNECTIONS: {s}")))?;
        app.max_connections(n);
    }
    if let Ok(s) = std::env::var("RUVO_MAX_UPGRADED_CONNECTIONS") {
        let n: usize = s.parse().map_err(|_| {
            Error::Internal(format!("RUVO_MAX_UPGRADED_CONNECTIONS: {s}"))
        })?;
        app.max_upgraded_connections(n);
    }
    if let Ok(s) = std::env::var("RUVO_MAX_CONCURRENT_STREAMS") {
        let n: usize = s.parse().map_err(|_| {
            Error::Internal(format!("RUVO_MAX_CONCURRENT_STREAMS: {s}"))
        })?;
        app.max_concurrent_streams(n);
    }
    if let Ok(s) = std::env::var("RUVO_MAX_HEADERS") {
        let n: usize = s
            .parse()
            .map_err(|_| Error::Internal(format!("RUVO_MAX_HEADERS: {s}")))?;
        app.max_headers(n);
    }
    if let Ok(s) = std::env::var("RUVO_MAX_BUF_SIZE") {
        app.max_buf_size(parse_bytes(&s).map_err(Error::Internal)?);
    }
    if let Ok(s) = std::env::var("RUVO_REQUEST_TIMEOUT") {
        app.request_timeout(parse_timeout_env(&s)?);
    }
    if let Ok(s) = std::env::var("RUVO_HEADER_READ_TIMEOUT") {
        app.header_read_timeout(parse_duration(&s).map_err(Error::Internal)?);
    }
    if let Ok(s) = std::env::var("RUVO_IDLE_TIMEOUT") {
        app.idle_timeout(parse_duration(&s).map_err(Error::Internal)?);
    }
    if let Ok(s) = std::env::var("RUVO_DRAIN_TIMEOUT") {
        app.drain_timeout(parse_duration(&s).map_err(Error::Internal)?);
    }
    if let Ok(s) = std::env::var("RUVO_KEEP_ALIVE") {
        let v = matches!(s.as_str(), "1" | "true" | "TRUE" | "yes");
        app.keep_alive(v);
    }
    if let Ok(s) = std::env::var("RUVO_TRUST_PROXY") {
        let v = matches!(s.as_str(), "1" | "true" | "TRUE" | "yes");
        app.trust_proxy(v);
    }
    Ok(())
}

impl App {
    /// Load `ruvo.toml` or `Ruvo.toml` from the current directory, then env overrides.
    ///
    /// Missing file is not an error — only `RUVO_*` env overrides apply.
    pub fn configure(&mut self) -> Result<&mut Self> {
        for name in ["ruvo.toml", "Ruvo.toml"] {
            let path = Path::new(name);
            if path.is_file() {
                return self.configure_from_path(path);
            }
        }
        env_override(self)?;
        Ok(self)
    }

    /// Load settings from a toml file (app-level only), then `RUVO_*` env overrides.
    pub fn configure_from_path(&mut self, path: impl AsRef<Path>) -> Result<&mut Self> {
        let text = std::fs::read_to_string(path.as_ref()).map_err(Error::Io)?;
        self.configure_from_str(&text)?;
        Ok(self)
    }

    /// Parse toml and apply `[server]` (+ legacy) for the active profile, then env overrides.
    pub fn configure_from_str(&mut self, text: &str) -> Result<&mut Self> {
        let root: toml::Value =
            toml::from_str(text).map_err(|e| Error::Internal(format!("ruvo.toml: {e}")))?;
        let profile_name = active_profile();
        self.state(ConfigDoc {
            root: root.clone(),
            profile: profile_name.clone(),
        });

        let server = resolve_server(&root, &profile_name);
        apply_server(self, &server)?;
        env_override(self)?;
        Ok(self)
    }

    /// `App::new()` + [`Self::configure_from_path`].
    pub fn from_toml(path: impl AsRef<Path>) -> Result<Self> {
        let mut app = App::new();
        app.configure_from_path(path)?;
        Ok(app)
    }

    /// Shared [`ConfigDoc`] from the last successful [`Self::configure_from_str`], if any.
    pub fn config_doc(&self) -> Option<Arc<ConfigDoc>> {
        self.state.get::<ConfigDoc>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_profile(profile: &str, f: impl FnOnce()) {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev_profile = std::env::var("RUVO_PROFILE").ok();
        let prev_env = std::env::var("RUVO_ENV").ok();
        std::env::set_var("RUVO_PROFILE", profile);
        std::env::remove_var("RUVO_ENV");
        f();
        match prev_profile {
            Some(v) => std::env::set_var("RUVO_PROFILE", v),
            None => std::env::remove_var("RUVO_PROFILE"),
        }
        match prev_env {
            Some(v) => std::env::set_var("RUVO_ENV", v),
            None => std::env::remove_var("RUVO_ENV"),
        }
    }

    #[test]
    fn parses_canon_server_and_development_overlay() {
        with_profile("development", || {
            let mut app = App::new();
            app.configure_from_str(
                r#"
[server]
max_body = "2 MiB"
request_timeout = "30s"
max_connections = 100

[development.server]
max_connections = 10
"#,
            )
            .unwrap();
            assert_eq!(app.max_connections, 10);
            assert_eq!(app.max_body_size, 2 * 1024 * 1024);
            assert_eq!(app.request_timeout, Some(Duration::from_secs(30)));
            assert_eq!(app.config_doc().unwrap().profile, "development");
        });
    }

    #[test]
    fn legacy_default_debug_still_works() {
        with_profile("development", || {
            let mut app = App::new();
            app.configure_from_str(
                r#"
[default]
max_body = "2 MiB"
request_timeout = "30s"
max_connections = 100

[debug]
max_connections = 10
"#,
            )
            .unwrap();
            assert_eq!(app.max_connections, 10);
            assert_eq!(app.max_body_size, 2 * 1024 * 1024);
        });
    }

    #[test]
    fn section_merges_top_level_and_profile() {
        with_profile("production", || {
            let mut app = App::new();
            app.configure_from_str(
                r#"
[mail]
from = "base@example.com"

[production.mail]
from = "prod@example.com"
"#,
            )
            .unwrap();
            let doc = app.config_doc().unwrap();
            assert_eq!(doc.profile, "production");
            let mail = doc.section("mail").unwrap();
            assert_eq!(
                mail.get("from").and_then(|v| v.as_str()),
                Some("prod@example.com")
            );
        });
    }

    #[test]
    fn stores_http_named_clients_in_config_doc() {
        with_profile("development", || {
            let mut app = App::new();
            app.configure_from_str(
                r#"
[server]
request_timeout = "30s"

[http.payments]
base_url = "https://api.stripe.com"
timeout = "10s"

[development.http.payments]
timeout = "5s"
"#,
            )
            .unwrap();
            let doc = app.config_doc().unwrap();
            let clients = doc.http_clients();
            let p = clients.get("payments").unwrap();
            assert_eq!(
                p.get("base_url").and_then(|v| v.as_str()),
                Some("https://api.stripe.com")
            );
            assert_eq!(p.get("timeout").and_then(|v| v.as_str()), Some("5s"));
        });
    }

    #[test]
    fn debug_alias_normalizes_to_development() {
        assert_eq!(normalize_profile_name("debug"), "development");
        assert_eq!(normalize_profile_name("release"), "production");
    }
}
