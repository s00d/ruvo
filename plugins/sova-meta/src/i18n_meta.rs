//! Optional i18n enrichment for hreflang / og:locale.

use crate::defaults::MetaDefaults;
use crate::resolve::ResolvedMeta;
use sova_core::Request;
use sova_i18n::{localized_url, strip_locale_prefix, I18nExt, I18nState, Locale};

pub fn enrich(req: &Request, defaults: &MetaDefaults, resolved: &mut ResolvedMeta) {
    let Some(state) = req.try_state::<I18nState>() else {
        return;
    };
    enrich_parts(
        state.as_ref(),
        req.locale(),
        &req.path,
        defaults,
        resolved,
    );
}

pub fn enrich_parts(
    state: &I18nState,
    current: &str,
    path: &str,
    defaults: &MetaDefaults,
    resolved: &mut ResolvedMeta,
) {
    let locales: Vec<Locale> = state.store.load().locales.clone();
    if locales.is_empty() {
        return;
    }

    if let Some(cur) = locales.iter().find(|l| l.code == current) {
        if cur.seo {
            resolved.og_locale = Some(cur.og_locale());
        }
    }

    let public = match &defaults.public_url {
        Some(u) => u.trim_end_matches('/').to_string(),
        None => return,
    };

    let codes: Vec<&str> = locales.iter().map(|l| l.code.as_str()).collect();
    let bare = strip_locale_prefix(path, &codes);

    let default_code = locales
        .iter()
        .find(|l| l.seo)
        .map(|l| l.code.as_str())
        .unwrap_or(state.default.as_ref());

    for loc in &locales {
        if !loc.seo {
            continue;
        }
        if loc.code != current {
            resolved.og_locale_alternate.push(loc.og_locale());
        }
        let href = localized_url(
            &public,
            &bare,
            &loc.code,
            default_code,
            state.path_prefix,
        );
        resolved.hreflang.push((loc.iso.clone(), href));
    }
    if locales.iter().any(|l| l.seo) {
        let href = localized_url(
            &public,
            &bare,
            default_code,
            default_code,
            state.path_prefix,
        );
        resolved.hreflang.push(("x-default".into(), href));
    }
}

/// SEO locales + path settings for sitemap `xhtml:link` alternates.
pub struct SitemapHreflang {
    pub default: String,
    pub path_prefix: bool,
    /// `(code, iso)` for locales with `seo: true`.
    pub locales: Vec<(String, String)>,
}

pub fn sitemap_hreflang_from_state(state: &I18nState) -> Option<SitemapHreflang> {
    let locales_meta = state.store.load().locales.clone();
    let locales: Vec<(String, String)> = locales_meta
        .iter()
        .filter(|l| l.seo)
        .map(|l| (l.code.clone(), l.iso.clone()))
        .collect();
    if locales.is_empty() {
        return None;
    }
    let default = locales_meta
        .iter()
        .find(|l| l.seo)
        .map(|l| l.code.clone())
        .unwrap_or_else(|| state.default.to_string());
    Some(SitemapHreflang {
        default,
        path_prefix: state.path_prefix,
        locales,
    })
}
