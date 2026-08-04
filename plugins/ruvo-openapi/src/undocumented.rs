use crate::doc::Doc;
use ruvo_core::extend::RouteEntry;
use ruvo_core::App;

/// Routes that lack a [`Doc`] (and are not [`Doc::skip`] / under `docs_prefix`).
pub fn undocumented(app: &App) -> Vec<String> {
    undocumented_with_prefix(app, "/docs")
}

pub fn undocumented_with_prefix(app: &App, docs_prefix: &str) -> Vec<String> {
    let prefix = docs_prefix.trim_end_matches('/');
    let mut out = Vec::new();
    for entry in app.route_entries() {
        let RouteEntry::Http {
            method,
            path,
            meta,
        } = entry
        else {
            continue;
        };
        if path == prefix || path.starts_with(&format!("{prefix}/")) {
            continue;
        }
        match meta.get::<Doc>() {
            Some(d) if d.is_skip() => continue,
            Some(_) => continue,
            None => out.push(format!("{method} {path}")),
        }
    }
    out.sort();
    out
}
