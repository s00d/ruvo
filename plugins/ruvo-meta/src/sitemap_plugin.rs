//! Plugin `Sitemap` — `/sitemap.xml` (+ pagination).

use crate::defaults::MetaDefaults;
use crate::sitemap::{
    build_sitemap_body, collect_entries_with, render_urlset, CollectOpts, Entry, HreflangOpts,
    SitemapCache, SitemapProvider, SitemapRegistry, MAX_URLS,
};
use crate::sitemap_config::SitemapConfig;
#[cfg(feature = "store")]
use crate::sitemap::SITEMAP_KV_KEY;
use ruvo_core::extend::{BoxFuture, RouteTable};
use ruvo_core::{App, Plugin, Request, Response};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "store")]
use ruvo_store::KvStore;

/// Optional shared sitemap cache (feature `store`).
#[cfg(feature = "store")]
#[derive(Clone)]
pub struct SitemapStore(pub Arc<dyn KvStore>);

/// Sitemap plugin: auto routes + exclude/include + providers.
pub struct Sitemap {
    public_url: Option<String>,
    path: String,
    ttl: Duration,
    from_routes: bool,
    excludes: Vec<String>,
    includes: Vec<Entry>,
    registry: SitemapRegistry,
    #[cfg(feature = "store")]
    cache_store: Option<Arc<dyn KvStore>>,
}

impl Sitemap {
    pub fn new() -> Self {
        Self {
            public_url: None,
            path: "/sitemap.xml".into(),
            ttl: Duration::from_secs(3600),
            from_routes: true,
            excludes: Vec::new(),
            includes: Vec::new(),
            registry: SitemapRegistry::default(),
            #[cfg(feature = "store")]
            cache_store: None,
        }
    }

    pub fn public_url(mut self, u: impl Into<String>) -> Self {
        self.public_url = Some(u.into());
        self
    }

    pub fn path(mut self, p: impl Into<String>) -> Self {
        self.path = p.into();
        self
    }

    pub fn ttl(mut self, d: Duration) -> Self {
        self.ttl = d;
        self
    }

    pub fn from_routes(mut self, yes: bool) -> Self {
        self.from_routes = yes;
        self
    }

    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.excludes.push(pattern.into());
        self
    }

    pub fn include(mut self, path: impl Into<String>) -> Self {
        self.includes.push(Entry::new(path));
        self
    }

    /// Shared KvStore for sitemap XML (L2 after in-process TTL cache).
    #[cfg(feature = "store")]
    pub fn cache_store(mut self, store: Arc<dyn KvStore>) -> Self {
        self.cache_store = Some(store);
        self
    }

    pub fn provider<F, Fut>(mut self, pattern: impl Into<String>, f: F) -> Self
    where
        F: Fn(crate::sitemap::SitemapCtx) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Vec<Entry>, String>> + Send + 'static,
    {
        let run = Arc::new(move |ctx| {
            let fut = f(ctx);
            Box::pin(fut) as BoxFuture<Result<Vec<Entry>, String>>
        });
        self.registry.providers.push(SitemapProvider {
            pattern: pattern.into(),
            run,
        });
        self
    }
}

impl Default for Sitemap {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Sitemap {
    fn id(&self) -> &'static str {
        "sitemap"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Sitemap")
            .description("Generate /sitemap.xml from routes, includes, and providers")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("sitemap") {
                if let Some(v) = section.get("public_url").and_then(|v| v.as_str()) {
                    self.public_url = Some(v.to_string());
                }
                if let Some(v) = section.get("path").and_then(|v| v.as_str()) {
                    self.path = v.to_string();
                }
                if let Some(v) = section.get("ttl").and_then(|v| v.as_integer()) {
                    if v > 0 {
                        self.ttl = Duration::from_secs(v as u64);
                    }
                }
                if let Some(arr) = section.get("exclude").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            self.excludes.push(s.to_string());
                        }
                    }
                }
                if let Some(arr) = section.get("include").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            self.includes.push(Entry::new(s));
                        }
                    }
                }
            }
        }

        if self.public_url.is_none() {
            if let Some(d) = app.try_state::<MetaDefaults>() {
                self.public_url = d.public_url.clone();
            }
        }

        let opts = CollectOpts {
            from_routes: self.from_routes,
            excludes: self.excludes.clone(),
            includes: self.includes.clone(),
        };
        let opts = Arc::new(opts);

        let cfg = SitemapConfig {
            public_url: self.public_url.clone(),
            path: self.path.clone(),
        };
        app.state(cfg);

        let registry = Arc::new(self.registry);
        let cache = Arc::new(SitemapCache::new(self.ttl));
        #[cfg(feature = "store")]
        let ttl = self.ttl;
        let public_url = self.public_url.clone();
        let mount = self.path.clone();

        #[cfg(feature = "store")]
        let kv: Option<Arc<dyn KvStore>> = self.cache_store.take().or_else(|| {
            app.try_state::<ruvo_store::AppStore>()
                .map(|s| s.namespaced("meta"))
        });
        #[cfg(feature = "store")]
        if let Some(ref store) = kv {
            app.state(SitemapStore(Arc::clone(store)));
        }

        app.state(Arc::clone(&registry));
        app.state(Arc::clone(&cache));
        app.state(Arc::clone(&opts));

        let paginated = sibling_sitemap_pattern(&mount);

        let reg_r = Arc::clone(&registry);
        let cache_r = Arc::clone(&cache);
        let opts_r = Arc::clone(&opts);
        let public_r = public_url.clone();
        #[cfg(feature = "store")]
        let kv_r = kv.clone();
        app.get(mount.as_str(), move |req: Request| {
            let registry = Arc::clone(&reg_r);
            let cache = Arc::clone(&cache_r);
            let opts = Arc::clone(&opts_r);
            let public = public_r.clone();
            #[cfg(feature = "store")]
            let kv = kv_r.clone();
            async move {
                if let Some(cached) = cache.get() {
                    return xml_response(String::from_utf8_lossy(&cached).into_owned());
                }
                #[cfg(feature = "store")]
                if let Some(ref store) = kv {
                    if let Some(cached) = store.get(SITEMAP_KV_KEY).await {
                        cache.set(cached.clone());
                        return xml_response(String::from_utf8_lossy(&cached).into_owned());
                    }
                }

                sitemap_response(
                    &req,
                    &registry,
                    public.as_deref(),
                    &cache,
                    &opts,
                    #[cfg(feature = "store")]
                    (ttl, kv.as_ref()),
                )
                .await
            }
        });

        let reg_p = Arc::clone(&registry);
        let opts_p = Arc::clone(&opts);
        let public_p = public_url;
        app.get(paginated.as_str(), move |req: Request| {
            let registry = Arc::clone(&reg_p);
            let opts = Arc::clone(&opts_p);
            let public = public_p.clone();
            async move {
                let n: usize = req
                    .params
                    .get("n")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1)
                    .max(1);
                let public = match public.or_else(|| {
                    req.try_state::<MetaDefaults>()
                        .and_then(|d| d.public_url.clone())
                }) {
                    Some(u) => u,
                    None => return Response::text("public_url required").status(500),
                };
                let hreflang = hreflang_from_req(&req);
                match collect_entries_with(&state_from_req(&req), &registry, &opts).await {
                    Ok(entries) => {
                        let start = (n - 1) * MAX_URLS;
                        let slice: Vec<_> =
                            entries.into_iter().skip(start).take(MAX_URLS).collect();
                        xml_response(render_urlset(&public, &slice, hreflang.as_ref()))
                    }
                    Err(e) => Response::text(e).status(500),
                }
            }
        });
    }
}

fn sibling_sitemap_pattern(mount: &str) -> String {
    // `/sitemap.xml` → `/sitemap-:n.xml`
    if let Some(stem) = mount.strip_suffix(".xml") {
        format!("{stem}-:n.xml")
    } else {
        format!("{mount}-:n.xml")
    }
}

fn xml_response(body: String) -> Response {
    Response::text(body).header("content-type", "application/xml; charset=utf-8")
}

async fn sitemap_response(
    req: &Request,
    registry: &SitemapRegistry,
    public_url: Option<&str>,
    cache: &SitemapCache,
    opts: &CollectOpts,
    #[cfg(feature = "store")] kv: (Duration, Option<&Arc<dyn KvStore>>),
) -> Response {
    let public = match public_url
        .map(|s| s.to_string())
        .or_else(|| {
            req.try_state::<MetaDefaults>()
                .and_then(|d| d.public_url.clone())
        })
        .or_else(|| {
            req.try_state::<SitemapConfig>()
                .and_then(|c| c.public_url.clone())
        }) {
        Some(u) => u,
        None => return Response::text("public_url required").status(500),
    };
    let hreflang = hreflang_from_req(req);
    let state = state_from_req(req);
    match build_sitemap_body(&state, registry, &public, hreflang.as_ref(), opts).await {
        Ok(bytes) => {
            let xml = String::from_utf8_lossy(&bytes).into_owned();
            cache.set(bytes.clone());
            #[cfg(feature = "store")]
            if let Some(store) = kv.1 {
                store.set(SITEMAP_KV_KEY, bytes, Some(kv.0)).await;
            }
            xml_response(xml)
        }
        Err(e) => {
            tracing::error!("sitemap: {e}");
            Response::text("sitemap unavailable").status(500)
        }
    }
}

fn hreflang_from_req(req: &Request) -> Option<HreflangOpts> {
    #[cfg(feature = "i18n")]
    {
        let state = req.try_state::<ruvo_i18n::I18nState>()?;
        let cfg = crate::i18n_meta::sitemap_hreflang_from_state(&state)?;
        Some(HreflangOpts {
            default: cfg.default,
            path_prefix: cfg.path_prefix,
            locales: cfg.locales,
        })
    }
    #[cfg(not(feature = "i18n"))]
    {
        let _ = req;
        None
    }
}

fn state_from_req(req: &Request) -> Arc<ruvo_core::extend::StateMap> {
    let mut map = ruvo_core::extend::StateMap::new();
    if let Some(table) = req.try_state::<RouteTable>() {
        map.insert((*table).clone());
    }
    if let Some(d) = req.try_state::<MetaDefaults>() {
        map.insert((*d).clone());
    }
    #[cfg(feature = "i18n")]
    if let Some(i18n) = req.try_state::<ruvo_i18n::I18nState>() {
        map.insert((*i18n).clone());
    }
    Arc::new(map)
}
