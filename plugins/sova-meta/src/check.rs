//! `register_audit("meta")`.

use crate::defaults::MetaDefaults;
use crate::page::MetaPage;
use crate::robots::RobotsConfig;
use crate::sitemap::{collect_entries, collect_entries_with, path_is_dynamic, CollectOpts, SitemapRegistry};
use http::Method;
use sova_core::extend::{RouteEntry, RouteTable};
use sova_core::{App, Error};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

pub fn register_meta_check(app: &mut App) {
    app.register_audit("meta", move |state| {
        async move {
            let defaults = state
                .get::<MetaDefaults>()
                .map(|d| (*d).clone())
                .unwrap_or_default();

            let robots_blocked = state
                .get::<RobotsConfig>()
                .map(|c| c.block_all)
                .unwrap_or(false)
                || defaults.robots_block_all;

            // staging profile must block robots
            let profile = std::env::var("SOVA_PROFILE").unwrap_or_default();
            if profile == "staging" && !robots_blocked {
                return Err(Error::Internal(
                    "meta: staging profile requires robots block_all (Robots::block_all or [robots]/block_all = true)".into(),
                ));
            }

            if let Some(table) = state.get::<RouteTable>() {
                check_titles(&table, &defaults)?;
                check_duplicate_canonicals(&table)?;
            }

            if let Some(ref img) = defaults.default_image {
                check_image_exists(img)?;
            }

            if let Some(registry) = state.get::<Arc<SitemapRegistry>>() {
                if let Some(opts) = state.get::<Arc<CollectOpts>>() {
                    let _ = collect_entries_with(&state, registry.as_ref(), opts.as_ref())
                        .await
                        .map_err(Error::Internal)?;
                } else {
                    let _ = collect_entries(&state, registry.as_ref())
                        .await
                        .map_err(Error::Internal)?;
                }
            }

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
            if meta.get::<sova_openapi::Doc>().is_some_and(|d| !d.is_skip()) {
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
    } else if defaults.check_strict {
        Err(Error::Internal(format!(
            "meta: indexable routes need title+description: {}",
            missing.join(", ")
        )))
    } else {
        tracing::warn!(
            "meta: indexable routes missing title+description: {}",
            missing.join(", ")
        );
        Ok(())
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
