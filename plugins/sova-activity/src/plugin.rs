//! Activity plugin: state + optional read mount.

use crate::list::{list_activity, ActivityFilter};
use crate::record::ActivityLog;
use sova_core::extend::{MwEntry, IntoMwEntry};
use sova_core::{App, Json, Plugin, Request, Result, Router};
use sova_db::DbExt;

/// Activity / audit log plugin.
pub struct Activity {
    mount: Option<String>,
    guard: Option<MwEntry>,
}

impl Activity {
    pub fn new() -> Self {
        Self {
            mount: None,
            guard: None,
        }
    }

    /// Serve `GET {path}` list (query: `subject_type`, `subject_id`, `event`, `limit`).
    pub fn mount(mut self, path: impl Into<String>) -> Self {
        self.mount = Some(path.into());
        self
    }

    /// Optional auth middleware for the read mount.
    pub fn guard(mut self, mw: impl IntoMwEntry) -> Self {
        self.guard = Some(mw.into_mw_entry());
        self
    }
}

impl Default for Activity {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Activity {
    fn id(&self) -> &'static str {
        "activity"
    }

    fn requires(&self) -> &'static [&'static str] {
        &["db"]
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Activity")
            .description("Audit / activity log (who changed what)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.state(ActivityLog);

        if let Some(path) = self.mount {
            let mut r = Router::new();
            if let Some(g) = self.guard {
                r.use_middleware(g);
            }
            r.get("/", list_handler);
            app.mount(&path, r);
        }
    }
}

async fn list_handler(req: Request) -> Result<Json<Vec<crate::ActivityRow>>> {
    let limit = req
        .query("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let filter = ActivityFilter {
        subject_type: req.query("subject_type").map(str::to_string),
        subject_id: req.query("subject_id").map(str::to_string),
        event: req.query("event").map(str::to_string),
        actor_id: req.query("actor_id").and_then(|s| s.parse().ok()),
        limit,
    };
    let rows = list_activity(req.db(), filter).await?;
    Ok(Json(rows))
}
