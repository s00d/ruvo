use crate::doc::Doc;
use crate::validate_meta::OpenApiValidate;
use sova_core::extend::{RouteEntry, RouteTable};
use sova_core::App;

/// Routes that lack a [`Doc`] / [`OpenApiValidate`] (and are not skip / under docs).
pub fn undocumented(app: &App) -> Vec<String> {
    undocumented_with_prefix(app, "/docs")
}

pub fn undocumented_with_prefix(app: &App, docs_prefix: &str) -> Vec<String> {
    let entries = app.route_entries();
    undocumented_entries(entries.iter(), docs_prefix)
}

/// Same as [`undocumented`], using a compiled [`RouteTable`] (e.g. from `check` state).
pub fn undocumented_from_table(table: &RouteTable, docs_prefix: &str) -> Vec<String> {
    undocumented_entries(table.0.iter(), docs_prefix)
}

fn undocumented_entries<'a>(
    entries: impl IntoIterator<Item = &'a RouteEntry>,
    docs_prefix: &str,
) -> Vec<String> {
    let prefix = docs_prefix.trim_end_matches('/');
    let mut out = Vec::new();
    for entry in entries {
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
        if meta.get::<OpenApiValidate>().is_some() {
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
