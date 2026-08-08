//! Sitemap generation from RouteTable + providers.

use crate::canonical::absolute_url;
use crate::page::MetaPage;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use http::Method;
use sova_core::extend::{BoxFuture, RouteEntry, RouteTable, StateMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_URLS_PER_FILE: usize = 50_000;

/// Cache key version — bump via crate version so deploy invalidates shared KvStore entries.
#[cfg(feature = "store")]
pub const SITEMAP_KV_KEY: &str = concat!("sitemap:xml:v", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Copy)]
pub enum ChangeFreq {
    Always,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}

impl ChangeFreq {
    fn as_str(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
            Self::Never => "never",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: String,
    pub lastmod: Option<DateTime<Utc>>,
    pub changefreq: Option<ChangeFreq>,
    pub priority: Option<f32>,
}

impl Entry {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }
    }

    pub fn lastmod(mut self, t: DateTime<Utc>) -> Self {
        self.lastmod = Some(t);
        self
    }

    pub fn changefreq(mut self, c: ChangeFreq) -> Self {
        self.changefreq = Some(c);
        self
    }

    pub fn priority(mut self, p: f32) -> Self {
        self.priority = Some(p);
        self
    }
}

pub struct SitemapCtx {
    pub state: Arc<StateMap>,
}

type ProviderFn =
    Arc<dyn Fn(SitemapCtx) -> BoxFuture<Result<Vec<Entry>, String>> + Send + Sync>;

#[derive(Clone)]
pub struct SitemapProvider {
    pub pattern: String,
    pub run: ProviderFn,
}

#[derive(Default)]
pub struct SitemapRegistry {
    pub providers: Vec<SitemapProvider>,
}

struct CacheSlot {
    body: Bytes,
    expires: Instant,
}

pub struct SitemapCache {
    inner: std::sync::Mutex<Option<CacheSlot>>,
    ttl: Duration,
}

impl SitemapCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: std::sync::Mutex::new(None),
            ttl,
        }
    }

    pub fn get(&self) -> Option<Bytes> {
        let g = self.inner.lock().unwrap();
        let slot = g.as_ref()?;
        if Instant::now() < slot.expires {
            Some(slot.body.clone())
        } else {
            None
        }
    }

    pub fn set(&self, body: Bytes) {
        *self.inner.lock().unwrap() = Some(CacheSlot {
            body,
            expires: Instant::now() + self.ttl,
        });
    }
}

/// Locale alternates for `xhtml:link` in urlset entries.
#[derive(Debug, Clone)]
pub struct HreflangOpts {
    pub default: String,
    pub path_prefix: bool,
    /// `(code, iso)` with `seo: true`.
    pub locales: Vec<(String, String)>,
}

pub fn path_is_dynamic(path: &str) -> bool {
    path.contains(':') || path.contains('*')
}

pub fn should_include_route(method: &Method, path: &str, meta: &sova_core::extend::MetaMap) -> bool {
    if *method != Method::GET {
        return false;
    }
    if path_is_dynamic(path) {
        return false;
    }
    if let Some(page) = meta.get::<MetaPage>() {
        if page.noindex {
            return false;
        }
    }
    #[cfg(feature = "openapi")]
    {
        if let Some(doc) = meta.get::<sova_openapi::Doc>() {
            if !doc.is_skip() {
                return false;
            }
        }
    }
    if path == "/sitemap.xml" || path.starts_with("/sitemap-") || path == "/robots.txt" {
        return false;
    }
    true
}

pub async fn collect_entries(
    state: &Arc<StateMap>,
    registry: &SitemapRegistry,
) -> Result<Vec<Entry>, String> {
    collect_entries_with(state, registry, &CollectOpts::default()).await
}

#[derive(Debug, Clone)]
pub struct CollectOpts {
    pub from_routes: bool,
    pub excludes: Vec<String>,
    pub includes: Vec<Entry>,
}

impl Default for CollectOpts {
    fn default() -> Self {
        Self {
            from_routes: true,
            excludes: Vec::new(),
            includes: Vec::new(),
        }
    }
}

/// Glob-ish match: `/admin`, `/admin/*`, `/api*`.
pub fn path_excluded(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| match_exclude(path, p))
}

fn match_exclude(path: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/*") {
        path == prefix || path.starts_with(&format!("{prefix}/"))
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        path.starts_with(prefix)
    } else {
        path == pattern
    }
}

pub async fn collect_entries_with(
    state: &Arc<StateMap>,
    registry: &SitemapRegistry,
    opts: &CollectOpts,
) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if opts.from_routes {
        if let Some(table) = state.get::<RouteTable>() {
            for entry in &table.0 {
                if let RouteEntry::Http {
                    method,
                    path,
                    meta,
                } = entry
                {
                    if !should_include_route(method, path.as_str(), meta) {
                        continue;
                    }
                    if path_excluded(path, &opts.excludes) {
                        continue;
                    }
                    if seen.insert(path.clone()) {
                        entries.push(Entry::new(path.clone()));
                    }
                }
            }
        }
    }

    for e in &opts.includes {
        if path_excluded(&e.path, &opts.excludes) {
            continue;
        }
        if seen.insert(e.path.clone()) {
            entries.push(e.clone());
        }
    }

    for p in &registry.providers {
        let ctx = SitemapCtx {
            state: Arc::clone(state),
        };
        let extra = (p.run)(ctx)
            .await
            .map_err(|e| format!("sitemap provider `{}`: {e}", p.pattern))?;
        for e in extra {
            if path_excluded(&e.path, &opts.excludes) {
                continue;
            }
            if seen.insert(e.path.clone()) {
                entries.push(e);
            }
        }
    }
    Ok(entries)
}

pub fn render_urlset(
    public_url: &str,
    entries: &[Entry],
    hreflang: Option<&HreflangOpts>,
) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    if hreflang.is_some() {
        out.push_str(
            "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
             xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
        );
    } else {
        out.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    }
    for e in entries.iter().take(MAX_URLS_PER_FILE) {
        let loc = absolute_url(public_url, &e.path);
        out.push_str("  <url>\n");
        out.push_str(&format!("    <loc>{}</loc>\n", xml_escape(&loc)));
        if let Some(t) = e.lastmod {
            out.push_str(&format!(
                "    <lastmod>{}</lastmod>\n",
                t.format("%Y-%m-%d")
            ));
        }
        if let Some(c) = e.changefreq {
            out.push_str(&format!(
                "    <changefreq>{}</changefreq>\n",
                c.as_str()
            ));
        }
        if let Some(p) = e.priority {
            out.push_str(&format!("    <priority>{p:.1}</priority>\n"));
        }
        if let Some(h) = hreflang {
            append_xhtml_links(&mut out, public_url, &e.path, h);
        }
        out.push_str("  </url>\n");
    }
    out.push_str("</urlset>\n");
    out
}

fn append_xhtml_links(out: &mut String, public_url: &str, entry_path: &str, h: &HreflangOpts) {
    #[cfg(feature = "i18n")]
    {
        use sova_i18n::{localized_url, strip_locale_prefix};
        let codes: Vec<&str> = h.locales.iter().map(|(c, _)| c.as_str()).collect();
        let bare = strip_locale_prefix(entry_path, &codes);
        for (code, iso) in &h.locales {
            let href = localized_url(public_url, &bare, code, &h.default, h.path_prefix);
            out.push_str(&format!(
                "    <xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{}\"/>\n",
                xml_escape(iso),
                xml_escape(&href)
            ));
        }
        let href = localized_url(
            public_url,
            &bare,
            &h.default,
            &h.default,
            h.path_prefix,
        );
        out.push_str(&format!(
            "    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{}\"/>\n",
            xml_escape(&href)
        ));
    }
    #[cfg(not(feature = "i18n"))]
    {
        let _ = (out, public_url, entry_path, h);
    }
}

pub fn render_sitemap_index(public_url: &str, count: usize) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
    );
    let files = count.div_ceil(MAX_URLS_PER_FILE).max(1);
    for i in 1..=files {
        let loc = absolute_url(public_url, &format!("/sitemap-{i}.xml"));
        out.push_str("  <sitemap>\n");
        out.push_str(&format!("    <loc>{}</loc>\n", xml_escape(&loc)));
        out.push_str("  </sitemap>\n");
    }
    out.push_str("</sitemapindex>\n");
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub async fn build_sitemap_body(
    state: &Arc<StateMap>,
    registry: &SitemapRegistry,
    public_url: &str,
    hreflang: Option<&HreflangOpts>,
    opts: &CollectOpts,
) -> Result<Bytes, String> {
    let entries = collect_entries_with(state, registry, opts).await?;
    let xml = if entries.len() > MAX_URLS_PER_FILE {
        render_sitemap_index(public_url, entries.len())
    } else {
        render_urlset(public_url, &entries, hreflang)
    };
    Ok(Bytes::from(xml))
}

pub const MAX_URLS: usize = MAX_URLS_PER_FILE;
