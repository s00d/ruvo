//! App-level meta defaults.

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
    /// Staging / `[meta] robots = "block-all"` — force noindex on all pages.
    pub robots_block_all: bool,
    /// When false, missing title/description on indexable routes only warn.
    pub check_strict: bool,
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
            check_strict: true,
        }
    }
}
