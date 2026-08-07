//! `register_check("meta")`.

use crate::defaults::MetaDefaults;
use crate::page::MetaPage;
use crate::sitemap::{collect_entries, path_is_dynamic, SitemapRegistry};
use http::Method;
use ruvo_core::extend::{RouteEntry, RouteTable};
use ruvo_core::{App, Error};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

pub fn register_meta_check(app: &mut App, registry: Arc<SitemapRegistry>) {
    app.register_check("meta", move |state| {
        let registry = Arc::clone(&registry);
        async move {
            let defaults = state
                .get::<MetaDefaults>()
                .map(|d| (*d).clone())
                .unwrap_or_default();

            // staging profile must block robots
            let profile = std::env::var("RUVO_PROFILE").unwrap_or_default();
            if profile == "staging" && !defaults.robots_block_all {
                return Err(Error::Internal(
                    "meta: staging profile requires robots = block-all (set [staging.meta])".into(),
                ));
            }

            if let Some(table) = state.get::<RouteTable>() {
                check_titles(&table, &defaults)?;
                check_duplicate_canonicals(&table)?;
            }

            if let Some(ref img) = defaults.default_image {
                check_image_exists(img)?;
            }

            // providers
            let _ = collect_entries(&state, &registry)
                .await
                .map_err(Error::Internal)?;

            Ok(())
        }
    });
}

fn check_titles(table: &RouteTable, defaults: &MetaDefaults) -> Result<(), Error> {
    let mut missing = Vec::new();
    for entry in &table.0 {
        let RouteEntry::Http {
            method,
            path,
            meta,
        } = entry
        else {
            continue;
        };
        if *method != Method::GET || path_is_dynamic(path) {
            continue;
        }
        if path == "/sitemap.xml" || path == "/robots.txt" || path.starts_with("/sitemap-") {
            continue;
        }
        #[cfg(feature = "openapi")]
        {
            if meta.get::<ruvo_openapi::Doc>().is_some_and(|d| !d.is_skip()) {
                continue;
            }
        }
        let page = meta.get::<MetaPage>();
        if page.as_ref().is_some_and(|p| p.noindex) {
            continue;
        }
        let title = page.as_ref().and_then(|p| p.title.clone());
        let desc = page.as_ref().and_then(|p| p.description.clone());
        let has_title = title.is_some() || defaults.site_name.is_some();
        let has_desc = desc.is_some();
        if !has_title || !has_desc {
            missing.push(format!("{method} {path}"));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Internal(format!(
            "meta: indexable routes need title+description: {}",
            missing.join(", ")
        )))
    }
}

fn check_duplicate_canonicals(table: &RouteTable) -> Result<(), Error> {
    let mut seen = HashSet::new();
    let mut dups = Vec::new();
    for entry in &table.0 {
        let RouteEntry::Http { path, meta, method } = entry else {
            continue;
        };
        if *method != Method::GET {
            continue;
        }
        let key = meta
            .get::<MetaPage>()
            .and_then(|p| p.canonical_path.clone())
            .unwrap_or_else(|| path.clone());
        if !seen.insert(key.clone()) {
            dups.push(key);
        }
    }
    if dups.is_empty() {
        Ok(())
    } else {
        Err(Error::Internal(format!(
            "meta: duplicate canonical paths: {}",
            dups.join(", ")
        )))
    }
}

fn check_image_exists(img: &str) -> Result<(), Error> {
    if img.starts_with("http://") || img.starts_with("https://") {
        return Ok(());
    }
    let path = img.trim_start_matches('/');
    if Path::new(path).is_file() || Path::new(img).is_file() {
        Ok(())
    } else {
        Err(Error::Internal(format!(
            "meta: default_image not found on disk: {img}"
        )))
    }
}
