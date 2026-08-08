//! robots.txt generation.

use crate::defaults::MetaDefaults;
use crate::page::MetaPage;
use crate::sitemap::path_is_dynamic;
use crate::sitemap_config::SitemapConfig;
use http::Method;
use sova_core::extend::{RouteEntry, RouteTable};

#[derive(Debug, Clone)]
pub enum RobotsDirective {
    Allow(String),
    Disallow(String),
    CrawlDelay(f32),
    Raw(String),
}

#[derive(Debug, Clone)]
pub struct RobotsGroup {
    pub user_agent: String,
    pub directives: Vec<RobotsDirective>,
}

/// Builder state for robots.txt.
#[derive(Debug, Clone)]
pub struct RobotsConfig {
    pub path: String,
    pub block_all: bool,
    pub from_noindex: bool,
    pub sitemap_url: Option<String>,
    pub sitemap_from_plugin: bool,
    pub crawl_delay: Option<f32>,
    pub groups: Vec<RobotsGroup>,
    pub trailing_raw: Vec<String>,
}

impl Default for RobotsConfig {
    fn default() -> Self {
        Self {
            path: "/robots.txt".into(),
            block_all: false,
            from_noindex: true,
            sitemap_url: None,
            sitemap_from_plugin: true,
            crawl_delay: None,
            groups: vec![RobotsGroup {
                user_agent: "*".into(),
                directives: vec![RobotsDirective::Allow("/".into())],
            }],
            trailing_raw: Vec::new(),
        }
    }
}

pub fn render_robots(
    cfg: &RobotsConfig,
    table: Option<&RouteTable>,
    sitemap_cfg: Option<&SitemapConfig>,
    meta: Option<&MetaDefaults>,
) -> String {
    if cfg.block_all {
        return "User-agent: *\nDisallow: /\n".into();
    }

    let mut out = String::new();
    for group in &cfg.groups {
        out.push_str(&format!("User-agent: {}\n", group.user_agent));
        let mut wrote = false;
        for d in &group.directives {
            match d {
                RobotsDirective::Allow(p) => {
                    out.push_str(&format!("Allow: {p}\n"));
                    wrote = true;
                }
                RobotsDirective::Disallow(p) => {
                    out.push_str(&format!("Disallow: {p}\n"));
                    wrote = true;
                }
                RobotsDirective::CrawlDelay(n) => {
                    out.push_str(&format!("Crawl-delay: {n}\n"));
                    wrote = true;
                }
                RobotsDirective::Raw(line) => {
                    out.push_str(line);
                    if !line.ends_with('\n') {
                        out.push('\n');
                    }
                    wrote = true;
                }
            }
        }
        if group.user_agent == "*" && cfg.from_noindex {
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
                            wrote = true;
                        }
                    }
                }
            }
        }
        if !wrote {
            out.push_str("Allow: /\n");
        }
        if let Some(delay) = cfg.crawl_delay {
            if group.user_agent == "*"
                && !group
                    .directives
                    .iter()
                    .any(|d| matches!(d, RobotsDirective::CrawlDelay(_)))
            {
                out.push_str(&format!("Crawl-delay: {delay}\n"));
            }
        }
        out.push('\n');
    }

    for line in &cfg.trailing_raw {
        out.push_str(line);
        if !line.ends_with('\n') {
            out.push('\n');
        }
    }

    let sitemap = cfg
        .sitemap_url
        .clone()
        .or_else(|| {
            if cfg.sitemap_from_plugin {
                sitemap_cfg.and_then(|c| c.sitemap_url()).or_else(|| {
                    let base = meta.and_then(|m| m.public_url.as_ref())?;
                    let path = sitemap_cfg
                        .map(|c| c.path.as_str())
                        .unwrap_or("/sitemap.xml");
                    Some(format!("{}{}", base.trim_end_matches('/'), path))
                })
            } else {
                None
            }
        });
    if let Some(url) = sitemap {
        out.push_str(&format!("Sitemap: {url}\n"));
    }
    out
}
