//! Field-level merge: request > route > defaults.

use crate::canonical::{absolute_url, apply_slash, strip_tracking};
use crate::defaults::{MetaDefaults, TrailingSlash};
use crate::overlay::{MetaOverlay, OverlaySnapshot};
use crate::page::MetaPage;
use chrono::{DateTime, Utc};
use ruvo_core::Request;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct ResolvedMeta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub noindex: bool,
    pub canonical: Option<String>,
    pub og_type: String,
    pub site_name: Option<String>,
    pub twitter_site: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub og_locale: Option<String>,
    pub og_locale_alternate: Vec<String>,
    pub hreflang: Vec<(String, String)>,
    pub jsonld: Vec<Value>,
}

pub fn resolve_meta(req: &Request) -> ResolvedMeta {
    let defaults = req
        .try_state::<MetaDefaults>()
        .map(|d| (*d).clone())
        .unwrap_or_default();
    let page = req.route_meta::<MetaPage>().map(|p| (*p).clone());
    let overlay = req
        .get::<MetaOverlay>()
        .map(|o| o.snapshot())
        .unwrap_or_default();
    #[allow(unused_mut)]
    let mut resolved = resolve_parts(
        &defaults,
        page.as_ref(),
        &overlay,
        &req.path,
        req.raw_query(),
    );

    #[cfg(feature = "i18n")]
    crate::i18n_meta::enrich(req, &defaults, &mut resolved);

    resolved
}

/// Resolve without request extensions beyond the overlay snapshot (headers middleware).
pub(crate) fn resolve_parts(
    defaults: &MetaDefaults,
    page: Option<&MetaPage>,
    overlay: &OverlaySnapshot,
    path: &str,
    raw_query: &str,
) -> ResolvedMeta {
    let page = page.cloned().unwrap_or_default();

    let title_raw = overlay
        .title
        .clone()
        .or(page.title)
        .or_else(|| defaults.site_name.clone());
    let title = title_raw.map(|t| apply_title_template(&t, defaults.title_template.as_deref()));

    let description = overlay.description.clone().or(page.description);
    let image = overlay
        .image
        .clone()
        .or(page.image)
        .or(defaults.default_image.clone())
        .map(|img| absolutize_image(defaults, &img));

    let noindex = overlay.noindex.unwrap_or(page.noindex) || defaults.robots_block_all;

    let path = overlay
        .canonical_path
        .clone()
        .or(page.canonical_path)
        .unwrap_or_else(|| path.to_string());
    let path = apply_slash(&path, defaults.trailing_slash);
    let canonical = defaults
        .public_url
        .as_ref()
        .map(|base| absolute_url(base, &strip_tracking(&path, raw_query)));

    let published = overlay.published;
    let mut og_type = overlay
        .og_type
        .clone()
        .or(page.og_type)
        .unwrap_or_else(|| "website".into());
    if published.is_some() && og_type == "website" {
        og_type = "article".into();
    }

    let mut jsonld = overlay.jsonld.clone();
    if let Some(pub_at) = published {
        if !jsonld.iter().any(|v| {
            v.get("@type")
                .and_then(|t| t.as_str())
                .is_some_and(|t| t == "Article")
        }) {
            if let Some(ref headline) = title {
                use crate::schema::{Article, ToJsonLd};
                jsonld.push(
                    Article {
                        headline: headline.clone(),
                        date_published: Some(pub_at),
                        description: description.clone(),
                        image: image.clone(),
                        ..Default::default()
                    }
                    .json_ld(),
                );
            }
        }
    }

    ResolvedMeta {
        title,
        description,
        image,
        noindex,
        canonical,
        og_type,
        site_name: defaults.site_name.clone(),
        twitter_site: defaults.twitter_site.clone(),
        published,
        og_locale: None,
        og_locale_alternate: Vec::new(),
        hreflang: Vec::new(),
        jsonld,
    }
}

fn apply_title_template(title: &str, template: Option<&str>) -> String {
    match template {
        Some(t) if t.contains("{}") => t.replacen("{}", title, 1),
        _ => title.to_string(),
    }
}

fn absolutize_image(defaults: &MetaDefaults, img: &str) -> String {
    if img.starts_with("http://") || img.starts_with("https://") {
        return img.to_string();
    }
    match &defaults.public_url {
        Some(base) => absolute_url(base, img),
        None => img.to_string(),
    }
}

#[allow(dead_code)]
pub fn effective_slash(req: &Request) -> TrailingSlash {
    req.try_state::<MetaDefaults>()
        .map(|d| d.trailing_slash)
        .unwrap_or_default()
}
