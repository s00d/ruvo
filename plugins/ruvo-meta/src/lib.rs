//! Document meta, OG/Twitter, JSON-LD, sitemap and robots for Ruvo.

mod canonical;
mod check;
mod defaults;
mod ext;
mod headers;
mod html;
mod overlay;
mod page;
mod plugin;
mod resolve;
mod robots;
pub mod schema;
mod sitemap;
mod slash;

#[cfg(feature = "i18n")]
mod i18n_meta;

#[cfg(feature = "templates")]
mod templates_meta;

pub use canonical::{absolute_url, apply_slash, strip_tracking};
pub use defaults::{MetaDefaults, TrailingSlash};
pub use ext::MetaExt;
pub use html::render_html;
pub use overlay::MetaOverlay;
pub use page::MetaPage;
pub use plugin::Meta;
#[cfg(feature = "store")]
pub use plugin::MetaSitemapStore;
pub use resolve::{resolve_meta, ResolvedMeta};
pub use schema::{
    Article, BreadcrumbList, FAQPage, FaqAnswer, FaqEntry, ListItem, LocalBusiness, Offer,
    Organization, Product, ToJsonLd, WebSite,
};
pub use sitemap::{ChangeFreq, Entry, HreflangOpts, SitemapCtx, SitemapRegistry};

#[cfg(feature = "templates")]
pub use templates_meta::with_meta;

