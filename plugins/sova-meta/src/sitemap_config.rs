//! Shared sitemap config visible to Robots (`Sitemap:` line).

/// Published by [`crate::Sitemap`] so [`crate::Robots`] can emit `Sitemap:`.
#[derive(Debug, Clone)]
pub struct SitemapConfig {
    pub public_url: Option<String>,
    /// Mount path, e.g. `/sitemap.xml`.
    pub path: String,
}

impl SitemapConfig {
    pub fn sitemap_url(&self) -> Option<String> {
        let base = self.public_url.as_ref()?;
        let base = base.trim_end_matches('/');
        let path = if self.path.starts_with('/') {
            self.path.clone()
        } else {
            format!("/{}", self.path)
        };
        Some(format!("{base}{path}"))
    }
}

impl Default for SitemapConfig {
    fn default() -> Self {
        Self {
            public_url: None,
            path: "/sitemap.xml".into(),
        }
    }
}
