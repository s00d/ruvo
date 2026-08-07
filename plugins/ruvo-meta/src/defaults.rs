//! App-level meta defaults.

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrailingSlash {
    #[default]
    Keep,
    Always,
    Never,
}

#[derive(Debug, Clone)]
pub struct MetaDefaults {
    pub site_name: Option<String>,
    pub title_template: Option<String>,
    pub default_image: Option<String>,
    pub twitter_site: Option<String>,
    pub public_url: Option<String>,
    pub trailing_slash: TrailingSlash,
    pub robots_block_all: bool,
    pub sitemap_ttl: Duration,
}

impl Default for MetaDefaults {
    fn default() -> Self {
        Self {
            site_name: None,
            title_template: None,
            default_image: None,
            twitter_site: None,
            public_url: None,
            trailing_slash: TrailingSlash::Keep,
            robots_block_all: false,
            sitemap_ttl: Duration::from_secs(3600),
        }
    }
}
