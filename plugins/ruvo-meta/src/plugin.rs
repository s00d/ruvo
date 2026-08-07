//! Plugin builder and install.

use crate::check::register_meta_check;
use crate::defaults::{MetaDefaults, TrailingSlash};
use crate::headers::install_headers_middleware;
use crate::page::MetaPage;
use crate::robots::render_robots;
use crate::sitemap::{
    build_sitemap_body, collect_entries, render_urlset, HreflangOpts, SitemapCache, SitemapProvider,
    SitemapRegistry, MAX_URLS,
};
#[cfg(feature = "store")]
use crate::sitemap::SITEMAP_KV_KEY;
use crate::slash::install_slash_middleware;
use ruvo_core::extend::{BoxFuture, RouteTable};
use ruvo_core::{App, Plugin, Request, Response};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "store")]
use ruvo_store::KvStore;

/// Optional shared sitemap cache (feature `store`).
#[cfg(feature = "store")]
#[derive(Clone)]
pub struct MetaSitemapStore(pub Arc<dyn KvStore>);

/// Outbound document-meta plugin.
pub struct Meta {
    defaults: MetaDefaults,
    registry: SitemapRegistry,
    #[cfg(feature = "store")]
    cache_store: Option<Arc<dyn KvStore>>,
}

impl Meta {
    pub fn new() -> Self {
        Self {
            defaults: MetaDefaults::default(),
            registry: SitemapRegistry::default(),
            #[cfg(feature = "store")]
            cache_store: None,
        }
    }

    pub fn site_name(mut self, name: impl Into<String>) -> Self {
        self.defaults.site_name = Some(name.into());
        self
    }

    pub fn title_template(mut self, t: impl Into<String>) -> Self {
        self.defaults.title_template = Some(t.into());
        self
    }

    pub fn default_image(mut self, img: impl Into<String>) -> Self {
        self.defaults.default_image = Some(img.into());
        self
    }

    pub fn twitter_site(mut self, s: impl Into<String>) -> Self {
        self.defaults.twitter_site = Some(s.into());
        self
    }

    pub fn public_url(mut self, u: impl Into<String>) -> Self {
        self.defaults.public_url = Some(u.into());
        self
    }

    pub fn trailing_slash(mut self, p: TrailingSlash) -> Self {
        self.defaults.trailing_slash = p;
        self
    }

    pub fn sitemap_ttl(mut self, d: Duration) -> Self {
        self.defaults.sitemap_ttl = d;
        self
    }

    /// Shared KvStore for sitemap XML (L2 after in-process TTL cache).
    #[cfg(feature = "store")]
    pub fn cache_store(mut self, store: Arc<dyn KvStore>) -> Self {
        self.cache_store = Some(store);
        self
    }

    pub fn page() -> MetaPage {
        MetaPage::new()
    }

    pub fn noindex() -> MetaPage {
        MetaPage::new().noindex()
    }

    pub fn provider<F, Fut>(mut self, pattern: impl Into<String>, f: F) -> Self
    where
        F: Fn(crate::sitemap::SitemapCtx) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Vec<crate::sitemap::Entry>, String>>
            + Send
            + 'static,
    {
        let run = Arc::new(move |ctx| {
            let fut = f(ctx);
            Box::pin(fut) as BoxFuture<Result<Vec<crate::sitemap::Entry>, String>>
        });
        self.registry.providers.push(SitemapProvider {
            pattern: pattern.into(),
            run,
        });
        self
    }
}

impl Default for Meta {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Meta {
    fn id(&self) -> &'static str {
        "meta"
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("meta") {
                if section
                    .get("robots")
                    .and_then(|v| v.as_str())
                    == Some("block-all")
                {
                    self.defaults.robots_block_all = true;
                }
                if let Some(v) = section.get("public_url").and_then(|v| v.as_str()) {
                    self.defaults.public_url = Some(v.to_string());
                }
                if let Some(v) = section.get("site_name").and_then(|v| v.as_str()) {
                    self.defaults.site_name = Some(v.to_string());
                }
            }
        }

        let defaults = self.defaults.clone();
        let registry = Arc::new(self.registry);
        let cache = Arc::new(SitemapCache::new(defaults.sitemap_ttl));
        #[cfg(feature = "store")]
        let ttl = defaults.sitemap_ttl;

        #[cfg(feature = "store")]
        let kv: Option<Arc<dyn KvStore>> = self.cache_store.take();
        #[cfg(feature = "store")]
        if let Some(ref store) = kv {
            app.state(MetaSitemapStore(Arc::clone(store)));
        }

        app.state(defaults.clone());
        app.state(Arc::clone(&registry));
        app.state(Arc::clone(&cache));

        install_slash_middleware(app);
        install_headers_middleware(app);

        app.get("/robots.txt", |req: Request| async move {
            let defaults = req
                .try_state::<MetaDefaults>()
                .map(|d| (*d).clone())
                .unwrap_or_default();
            let table = req.try_state::<RouteTable>();
            let body = render_robots(&defaults, table.as_deref());
            Response::text(body).header("content-type", "text/plain; charset=utf-8")
        });

        let reg_r = Arc::clone(&registry);
        let cache_r = Arc::clone(&cache);
        let defaults_r = defaults.clone();
        #[cfg(feature = "store")]
        let kv_r = kv.clone();
        app.get("/sitemap.xml", move |req: Request| {
            let registry = Arc::clone(&reg_r);
            let cache = Arc::clone(&cache_r);
            let defaults = defaults_r.clone();
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
                    &defaults,
                    &cache,
                    #[cfg(feature = "store")]
                    (ttl, kv.as_ref()),
                )
                .await
            }
        });

        let reg_p = Arc::clone(&registry);
        let defaults_p = defaults.clone();
        app.get("/sitemap-:n.xml", move |req: Request| {
            let registry = Arc::clone(&reg_p);
            let defaults = defaults_p.clone();
            async move {
                let n: usize = req
                    .params
                    .get("n")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1)
                    .max(1);
                let public = match &defaults.public_url {
                    Some(u) => u.clone(),
                    None => return Response::text("public_url required").status(500),
                };
                let hreflang = hreflang_from_req(&req);
                match collect_from_req(&req, &registry).await {
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

        register_meta_check(app, registry);
    }
}

fn xml_response(body: String) -> Response {
    Response::text(body).header("content-type", "application/xml; charset=utf-8")
}

async fn sitemap_response(
    req: &Request,
    registry: &SitemapRegistry,
    defaults: &MetaDefaults,
    cache: &SitemapCache,
    #[cfg(feature = "store")] kv: (Duration, Option<&Arc<dyn KvStore>>),
) -> Response {
    let hreflang = hreflang_from_req(req);
    let state = state_from_req(req);
    match build_sitemap_body(&state, registry, defaults, hreflang.as_ref()).await {
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

async fn collect_from_req(
    req: &Request,
    registry: &SitemapRegistry,
) -> Result<Vec<crate::sitemap::Entry>, String> {
    collect_entries(&state_from_req(req), registry).await
}
