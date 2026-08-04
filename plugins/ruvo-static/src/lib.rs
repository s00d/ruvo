//! Static file routes as a regular [`Plugin`] — public `Router::get` + conditional headers.

mod serve;

use ruvo_core::extend::normalize_path;
use ruvo_core::{App, Plugin, Request, Router};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Mount a directory behind Express-style routes (`mount` + `mount/*path`).
pub struct Static {
    mount: String,
    dir: PathBuf,
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
        }
    }

    /// Register on any [`Router`]. Module middleware is applied when that router is mounted.
    pub fn register(self, router: &mut Router) {
        let dir = Arc::new(self.dir);
        let mount = self.mount;

        let wildcard = if mount == "/" {
            "/*path".to_string()
        } else {
            format!("{mount}/*path")
        };

        let dir_index = Arc::clone(&dir);
        router.get(&mount, move |req: Request| {
            let dir = Arc::clone(&dir_index);
            async move {
                serve::serve_in(
                    dir.as_path(),
                    Path::new("index.html"),
                    serve::FileOptions::from_request(&req),
                )
                .await
            }
        });

        let dir_files = Arc::clone(&dir);
        router.get(&wildcard, move |req: Request| {
            let dir = Arc::clone(&dir_files);
            async move {
                let rel = req
                    .param("path")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("index.html"));
                serve::serve_in(dir.as_path(), &rel, serve::FileOptions::from_request(&req)).await
            }
        });
    }
}

impl Plugin for Static {
    fn install(self, app: &mut App) {
        self.register(app);
    }
}
