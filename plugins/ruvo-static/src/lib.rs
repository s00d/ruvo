//! Static file routes as a regular [`Plugin`] — public `Router::get` + conditional headers.

mod serve;

use ruvo_core::extend::normalize_path;
use ruvo_core::{App, Plugin, Request, Router};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Mount a directory behind Express-style routes (`mount` + `mount/*path`).
pub struct Static {
    mount: String,
    dir: PathBuf,
    max_age: Duration,
    max_age_explicit: bool,
    immutable: bool,
    allow_dotfiles: bool,
    index: bool,
}

impl Static {
    pub fn new(mount: impl Into<String>, dir: impl Into<PathBuf>) -> Self {
        let mut mount = normalize_path(&mount.into());
        while mount.len() > 1 && mount.ends_with('/') {
            mount.pop();
        }
        Self {
            mount,
            dir: dir.into(),
            max_age: Duration::from_secs(3600),
            max_age_explicit: false,
            immutable: false,
            allow_dotfiles: false,
            index: true,
        }
    }

    /// `Cache-Control: public, max-age=N` (default 1 hour).
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self.max_age_explicit = true;
        self
    }

    /// Append `immutable` to Cache-Control.
    pub fn immutable(mut self, on: bool) -> Self {
        self.immutable = on;
        self
    }

    /// Allow paths with `.`-prefixed segments (default: deny → 403).
    pub fn dotfiles_allow(mut self) -> Self {
        self.allow_dotfiles = true;
        self
    }

    /// Serve `index.html` for the mount path (default true).
    pub fn index(mut self, on: bool) -> Self {
        self.index = on;
        self
    }

    /// Register on any [`Router`]. Module middleware is applied when that router is mounted.
    pub fn register(self, router: &mut Router) {
        let opts = Arc::new(serve::StaticOpts {
            max_age: self.max_age,
            immutable: self.immutable,
            allow_dotfiles: self.allow_dotfiles,
        });
        let dir = Arc::new(self.dir);
        let mount = self.mount;

        if self.index {
            let dir_index = Arc::clone(&dir);
            let opts_index = Arc::clone(&opts);
            router.get(&mount, move |req: Request| {
                let dir = Arc::clone(&dir_index);
                let opts = Arc::clone(&opts_index);
                async move {
                    serve::serve_in(
                        dir.as_path(),
                        Path::new("index.html"),
                        serve::FileOptions::from_request(&req),
                        &opts,
                    )
                    .await
                }
            });
        }

        let wildcard = if mount == "/" {
            "/*path".to_string()
        } else {
            format!("{mount}/*path")
        };

        let dir_files = Arc::clone(&dir);
        router.get(&wildcard, move |req: Request| {
            let dir = Arc::clone(&dir_files);
            let opts = Arc::clone(&opts);
            async move {
                let rel = req
                    .param("path")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("index.html"));
                serve::serve_in(
                    dir.as_path(),
                    &rel,
                    serve::FileOptions::from_request(&req),
                    &opts,
                )
                .await
            }
        });
    }
}

impl Plugin for Static {
    fn id(&self) -> &'static str {
        "static"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Static files")
            .description("Serve files from a directory under a mount path")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if !self.max_age_explicit {
            if let Some(doc) = app.config_doc() {
                if let Some(section) = doc.section("static") {
                    if let Some(s) = section.get("max_age").and_then(|v| v.as_str()) {
                        if let Ok(d) = ruvo_core::extend::parse_duration(s) {
                            self.max_age = d;
                        }
                    } else if let Some(n) = section.get("max_age").and_then(|v| v.as_integer()) {
                        self.max_age = Duration::from_secs(n as u64);
                    }
                }
            }
        }
        self.register(app);
    }
}
