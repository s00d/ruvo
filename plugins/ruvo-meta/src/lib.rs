//! Document meta, OG/Twitter, JSON-LD, sitemap and robots for Ruvo.

mod canonical;
mod check;
mod defaults;
mod ext;
mod headers;
mod html;
mod inject;
mod overlay;
mod page;
mod plugin;
mod resolve;
mod robots;
mod robots_plugin;
pub mod schema;
mod sitemap;
mod sitemap_config;
mod sitemap_plugin;
mod slash;

#[cfg(feature = "i18n")]
mod i18n_meta;

#[cfg(feature = "templates")]
mod templates_meta;

pub use canonical::{absolute_url, apply_slash, strip_tracking};
pub use defaults::{MetaDefaults, TrailingSlash};
pub use ext::MetaExt;
pub use html::render_html;
pub use inject::inject_head;
pub use overlay::MetaOverlay;
pub use page::MetaPage;
pub use plugin::Meta;
pub use resolve::{resolve_meta, ResolvedMeta};
pub use robots::{RobotsConfig, RobotsDirective, RobotsGroup};
pub use robots_plugin::{Robots, RobotsUaBuilder};
pub use schema::{
    Article, BreadcrumbList, FAQPage, FaqAnswer, FaqEntry, ListItem, LocalBusiness, Offer,
    Organization, Product, ToJsonLd, WebSite,
};
pub use sitemap::{ChangeFreq, Entry, HreflangOpts, SitemapCtx, SitemapRegistry};
pub use sitemap_config::SitemapConfig;
pub use sitemap_plugin::Sitemap;
#[cfg(feature = "store")]
pub use sitemap_plugin::SitemapStore;

#[cfg(feature = "templates")]
pub use templates_meta::with_meta;
