//! robots.txt generation.

use crate::defaults::MetaDefaults;
use crate::page::MetaPage;
use crate::sitemap::path_is_dynamic;
use http::Method;
use ruvo_core::extend::{RouteEntry, RouteTable};

pub fn render_robots(defaults: &MetaDefaults, table: Option<&RouteTable>) -> String {
    if defaults.robots_block_all {
        return "User-agent: *\nDisallow: /\n".into();
    }
    let mut out = String::from("User-agent: *\n");
    let mut has_disallow = false;
    if let Some(table) = table {
        for entry in &table.0 {
            if let RouteEntry::Http {
                method,
                path,
                meta,
            } = entry
            {
                if *method != Method::GET || path_is_dynamic(path) {
                    continue;
                }
                if meta.get::<MetaPage>().is_some_and(|p| p.noindex) {
                    out.push_str(&format!("Disallow: {path}\n"));
                    has_disallow = true;
                }
            }
        }
    }
    if !has_disallow {
        out.push_str("Allow: /\n");
    }
    if let Some(ref base) = defaults.public_url {
        let sitemap = format!("{}/sitemap.xml", base.trim_end_matches('/'));
        out.push_str(&format!("Sitemap: {sitemap}\n"));
    }
    out
}
