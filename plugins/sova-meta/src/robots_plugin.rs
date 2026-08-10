//! Plugin `Robots` — `/robots.txt`.

use crate::defaults::MetaDefaults;
use crate::robots::{render_robots, RobotsConfig, RobotsDirective, RobotsGroup};
use crate::sitemap_config::SitemapConfig;
use sova_core::extend::RouteTable;
use sova_core::{App, Plugin, Request, Response};

/// Full-control robots.txt plugin.
pub struct Robots {
    cfg: RobotsConfig,
}

impl Robots {
    pub fn new() -> Self {
        Self {
            cfg: RobotsConfig::default(),
        }
    }

    pub fn path(mut self, p: impl Into<String>) -> Self {
        self.cfg.path = p.into();
        self
    }

    pub fn block_all(mut self) -> Self {
        self.cfg.block_all = true;
        self
    }

    pub fn allow(mut self, path: impl Into<String>) -> Self {
        ensure_star_group(&mut self.cfg)
            .directives
            .push(RobotsDirective::Allow(path.into()));
        self
    }

    pub fn disallow(mut self, path: impl Into<String>) -> Self {
        ensure_star_group(&mut self.cfg)
            .directives
            .push(RobotsDirective::Disallow(path.into()));
        self
    }

    pub fn user_agent<F>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(RobotsUaBuilder) -> RobotsUaBuilder,
    {
        let built = f(RobotsUaBuilder {
            group: RobotsGroup {
                user_agent: name.into(),
                directives: Vec::new(),
            },
        });
        self.cfg.groups.push(built.group);
        self
    }

    pub fn sitemap(mut self, url: impl Into<String>) -> Self {
        self.cfg.sitemap_url = Some(url.into());
        self.cfg.sitemap_from_plugin = false;
        self
    }

    pub fn sitemap_from_plugin(mut self, yes: bool) -> Self {
        self.cfg.sitemap_from_plugin = yes;
        if !yes {
            self.cfg.sitemap_url = None;
        }
        self
    }

    pub fn from_noindex(mut self, yes: bool) -> Self {
        self.cfg.from_noindex = yes;
        self
    }

    pub fn crawl_delay(mut self, seconds: f32) -> Self {
        self.cfg.crawl_delay = Some(seconds);
        self
    }

    pub fn raw(mut self, line: impl Into<String>) -> Self {
        self.cfg.trailing_raw.push(line.into());
        self
    }
}

impl Default for Robots {
    fn default() -> Self {
        Self::new()
    }
}

/// Fluent builder for a User-agent group.
pub struct RobotsUaBuilder {
    group: RobotsGroup,
}

impl RobotsUaBuilder {
    pub fn allow(mut self, path: impl Into<String>) -> Self {
        self.group
            .directives
            .push(RobotsDirective::Allow(path.into()));
        self
    }

    pub fn disallow(mut self, path: impl Into<String>) -> Self {
        self.group
            .directives
            .push(RobotsDirective::Disallow(path.into()));
        self
    }

    pub fn crawl_delay(mut self, seconds: f32) -> Self {
        self.group
            .directives
            .push(RobotsDirective::CrawlDelay(seconds));
        self
    }

    pub fn raw(mut self, line: impl Into<String>) -> Self {
        self.group
            .directives
            .push(RobotsDirective::Raw(line.into()));
        self
    }
}

fn ensure_star_group(cfg: &mut RobotsConfig) -> &mut RobotsGroup {
    if let Some(i) = cfg.groups.iter().position(|g| g.user_agent == "*") {
        &mut cfg.groups[i]
    } else {
        cfg.groups.insert(
            0,
            RobotsGroup {
                user_agent: "*".into(),
                directives: Vec::new(),
            },
        );
        &mut cfg.groups[0]
    }
}

impl Plugin for Robots {
    fn id(&self) -> &'static str {
        "robots"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Robots")
            .description("Serve robots.txt with allow/disallow and Sitemap line")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("robots") {
                if section
                    .get("block_all")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                    || section.get("block_all").and_then(|v| v.as_str()) == Some("true")
                {
                    self.cfg.block_all = true;
                }
                if let Some(v) = section.get("path").and_then(|v| v.as_str()) {
                    self.cfg.path = v.to_string();
                }
                if let Some(v) = section.get("sitemap").and_then(|v| v.as_str()) {
                    self.cfg.sitemap_url = Some(v.to_string());
                    self.cfg.sitemap_from_plugin = false;
                }
                if let Some(v) = section.get("from_noindex").and_then(|v| v.as_bool()) {
                    self.cfg.from_noindex = v;
                }
                if let Some(arr) = section.get("allow").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            ensure_star_group(&mut self.cfg)
                                .directives
                                .push(RobotsDirective::Allow(s.to_string()));
                        }
                    }
                }
                if let Some(arr) = section.get("disallow").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            ensure_star_group(&mut self.cfg)
                                .directives
                                .push(RobotsDirective::Disallow(s.to_string()));
                        }
                    }
                }
            }
            // legacy [meta] robots = "block-all"
            if let Some(section) = doc.section("meta") {
                if section.get("robots").and_then(|v| v.as_str()) == Some("block-all") {
                    self.cfg.block_all = true;
                }
            }
        }

        let mount = self.cfg.path.clone();
        app.state(self.cfg.clone());

        app.get(mount.as_str(), |req: Request| async move {
            let cfg = req
                .try_state::<RobotsConfig>()
                .map(|c| (*c).clone())
                .unwrap_or_default();
            let table = req.try_state::<RouteTable>();
            let sitemap = req.try_state::<SitemapConfig>();
            let meta = req.try_state::<MetaDefaults>();
            let body = render_robots(&cfg, table.as_deref(), sitemap.as_deref(), meta.as_deref());
            Response::text(body).header("content-type", "text/plain; charset=utf-8")
        });
    }
}
