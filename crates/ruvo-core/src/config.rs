//! Load app-level settings from `ruvo.toml` (no per-route addressing).

use crate::app::App;
use crate::error::{Error, Result};
use crate::human::{parse_bytes, parse_duration};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Full parsed `ruvo.toml` (after profile merge of known keys; raw root for plugins).
#[derive(Debug, Clone)]
pub struct ConfigDoc {
    /// Original root table (`default`, profile names, …).
    pub root: toml::Value,
    /// Active profile name (`debug` / `release` / `RUVO_PROFILE`).
    pub profile: String,
}

impl ConfigDoc {
    /// Table at `default.<section>` merged with `<profile>.<section>` (shallow key merge).
    pub fn section(&self, section: &str) -> Option<toml::map::Map<String, toml::Value>> {
        let mut out = toml::map::Map::new();
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
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// Named client tables under `http` (`[default.http.payments]`).
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
        if let Some(toml::Value::Table(http)) = self.root.get("default").and_then(|d| d.get("http"))
        {
            merge_into(&mut out, http);
        }
        if let Some(toml::Value::Table(http)) =
            self.root.get(&self.profile).and_then(|d| d.get("http"))
        {
            merge_into(&mut out, http);
        }
        out
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
struct Profile {
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

#[derive(Debug, Default, Deserialize)]
struct RuvoFile {
    #[serde(default)]
    default: Profile,
    #[serde(flatten)]
    profiles: HashMap<String, Profile>,
}

fn active_profile() -> String {
    if let Ok(p) = std::env::var("RUVO_PROFILE") {
        return p;
    }
    if cfg!(debug_assertions) {
        "debug".into()
    } else {
        "release".into()
    }
}

fn merge(base: Profile, over: Profile) -> Profile {
    Profile {
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

fn apply_profile(app: &mut App, p: &Profile) -> Result<()> {
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
    if let Ok(s) = std::env::var("RUVO_REQUEST_TIMEOUT") {
        if s.eq_ignore_ascii_case("off") {
            app.request_timeout(None);
        } else {
            app.request_timeout(Some(parse_duration(&s).map_err(Error::Internal)?));
        }
    }
    if let Ok(s) = std::env::var("RUVO_TRUST_PROXY") {
        let v = matches!(s.as_str(), "1" | "true" | "TRUE" | "yes");
        app.trust_proxy(v);
    }
    Ok(())
}

impl App {
    /// Load `ruvo.toml` or `Ruvo.toml` from the current directory, then env overrides.
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

    /// Parse toml text and apply `[default]` + active profile (`RUVO_PROFILE` / debug|release).
    pub fn configure_from_str(&mut self, text: &str) -> Result<&mut Self> {
        let root: toml::Value =
            toml::from_str(text).map_err(|e| Error::Internal(format!("ruvo.toml: {e}")))?;
        let profile_name = active_profile();
        self.state(ConfigDoc {
            root: root.clone(),
            profile: profile_name.clone(),
        });

        let file: RuvoFile =
            toml::from_str(text).map_err(|e| Error::Internal(format!("ruvo.toml: {e}")))?;
        let mut merged = file.default;
        if let Some(over) = file.profiles.get(&profile_name) {
            merged = merge(merged, over.clone());
        }
        apply_profile(self, &merged)?;
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
    use std::time::Duration;

    #[test]
    fn parses_human_values() {
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
        // profile is debug under debug_assertions
        assert_eq!(app.max_connections, 10);
        assert_eq!(app.max_body_size, 2 * 1024 * 1024);
        assert_eq!(app.request_timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn stores_http_named_clients_in_config_doc() {
        let mut app = App::new();
        app.configure_from_str(
            r#"
[default]
request_timeout = "30s"

[default.http.payments]
base_url = "https://api.stripe.com"
timeout = "10s"

[debug.http.payments]
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
    }
}
