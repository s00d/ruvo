//! Plugin builder and install (head / SEO only).

use crate::check::register_meta_check;
use crate::defaults::{MetaDefaults, TrailingSlash};
use crate::headers::install_headers_middleware;
use crate::page::MetaPage;
use crate::slash::install_slash_middleware;
use sova_core::{App, Plugin};

/// Outbound document-meta plugin.
pub struct Meta {
    defaults: MetaDefaults,
}

impl Meta {
    pub fn new() -> Self {
        Self {
            defaults: MetaDefaults::default(),
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

    /// Soft `check`: missing title/description warn via tracing instead of failing.
    pub fn soft_check(mut self) -> Self {
        self.defaults.check_strict = false;
        self
    }

    pub fn page() -> MetaPage {
        MetaPage::new()
    }

    pub fn noindex() -> MetaPage {
        MetaPage::new().noindex()
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

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Meta")
            .description("Document meta, OG/Twitter, JSON-LD, and head inject")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        // Unset-fill from `[meta]` — explicit builder values win.
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("meta") {
                if section.get("robots").and_then(|v| v.as_str()) == Some("block-all") {
                    self.defaults.robots_block_all = true;
                }
                if self.defaults.public_url.is_none() {
                    if let Some(v) = section.get("public_url").and_then(|v| v.as_str()) {
                        self.defaults.public_url = Some(v.to_string());
                    }
                }
                if self.defaults.site_name.is_none() {
                    if let Some(v) = section.get("site_name").and_then(|v| v.as_str()) {
                        self.defaults.site_name = Some(v.to_string());
                    }
                }
                if self.defaults.title_template.is_none() {
                    if let Some(v) = section.get("title_template").and_then(|v| v.as_str()) {
                        self.defaults.title_template = Some(v.to_string());
                    }
                }
                if self.defaults.default_image.is_none() {
                    if let Some(v) = section.get("default_image").and_then(|v| v.as_str()) {
                        self.defaults.default_image = Some(v.to_string());
                    }
                }
                if section.get("check").and_then(|v| v.as_str()) == Some("soft") {
                    self.defaults.check_strict = false;
                }
            }
        }

        let defaults = self.defaults.clone();
        app.state(defaults);

        install_slash_middleware(app);
        install_headers_middleware(app);

        #[cfg(feature = "templates")]
        {
            use crate::html::render_html;
            use crate::resolve_meta;
            use minijinja::Value;
            use sova_core::Request;
            sova_templates::register_per_request(app, "meta", |req: &Request| {
                let html = render_html(&resolve_meta(req));
                Value::from_function(move || html.clone())
            });
        }

        register_meta_check(app);
    }
}
