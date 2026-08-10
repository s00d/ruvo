//! Locale resolution chain.

use sova_core::Request;

/// Sources checked in order (first match wins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocaleSource {
    Path,
    Query,
    Cookie,
    AcceptLanguage,
    Default,
}

#[derive(Debug, Clone)]
pub struct ResolvedLocale {
    pub code: String,
    pub source: LocaleSource,
}

#[derive(Debug, Clone)]
pub struct ResolveOptions {
    pub locales: Vec<String>,
    pub default: String,
    pub query_key: String,
    pub cookie_name: Option<String>,
    /// When true, first path segment may be a locale (`/en/...`).
    pub path_prefix: bool,
}

impl ResolveOptions {
    pub fn new(
        locales: impl IntoIterator<Item = impl Into<String>>,
        default: impl Into<String>,
    ) -> Self {
        Self {
            locales: locales.into_iter().map(|s| s.into()).collect(),
            default: default.into(),
            query_key: "locale".into(),
            cookie_name: None,
            path_prefix: true,
        }
    }
}

pub fn resolve_server_locale(req: &Request, opts: &ResolveOptions) -> ResolvedLocale {
    if opts.path_prefix {
        if let Some(code) = path_locale(&req.path, &opts.locales) {
            return ResolvedLocale {
                code,
                source: LocaleSource::Path,
            };
        }
    }

    if let Some(code) = req
        .query
        .get(&opts.query_key)
        .map(|s| s.as_str())
        .and_then(|c| normalize_known(c, &opts.locales))
    {
        return ResolvedLocale {
            code,
            source: LocaleSource::Query,
        };
    }

    #[cfg(feature = "cookie")]
    if let Some(name) = &opts.cookie_name {
        if let Some(code) = req
            .get::<sova_cookies::Cookies>()
            .and_then(|c| c.get(name))
            .and_then(|c| normalize_known(c, &opts.locales))
        {
            return ResolvedLocale {
                code,
                source: LocaleSource::Cookie,
            };
        }
    }

    if let Some(raw) = req.header("accept-language") {
        if let Some(code) = negotiate_accept_language(raw, &opts.locales) {
            return ResolvedLocale {
                code,
                source: LocaleSource::AcceptLanguage,
            };
        }
    }

    ResolvedLocale {
        code: opts.default.clone(),
        source: LocaleSource::Default,
    }
}

fn path_locale(path: &str, locales: &[String]) -> Option<String> {
    let seg = path.trim_start_matches('/').split('/').next().unwrap_or("");
    normalize_known(seg, locales)
}

fn normalize_known(code: &str, locales: &[String]) -> Option<String> {
    let lower = code.to_ascii_lowercase();
    locales
        .iter()
        .find(|l| l.eq_ignore_ascii_case(&lower))
        .cloned()
}

/// Parse `Accept-Language` and pick the best known locale.
pub fn negotiate_accept_language(header: &str, locales: &[String]) -> Option<String> {
    let mut tags: Vec<(String, f32)> = Vec::new();
    for part in header.split(',') {
        let mut it = part.trim().split(';');
        let tag = it.next()?.trim().to_ascii_lowercase();
        if tag.is_empty() || tag == "*" {
            continue;
        }
        let mut q = 1.0_f32;
        for param in it {
            let param = param.trim();
            if let Some(v) = param.strip_prefix("q=") {
                q = v.parse().unwrap_or(0.0);
            }
        }
        tags.push((tag, q));
    }
    tags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (tag, _) in &tags {
        if let Some(c) = normalize_known(tag, locales) {
            return Some(c);
        }
        if let Some((lang, _)) = tag.split_once('-') {
            if let Some(c) = normalize_known(lang, locales) {
                return Some(c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use sova_core::Request;

    fn opts() -> ResolveOptions {
        ResolveOptions::new(["en", "de"], "en")
    }

    #[test]
    fn path_beats_query() {
        let req = Request::builder().method(Method::GET).path("/de/x").build();
        // query would be de vs en — path wins
        let mut req = req;
        req.query.insert("locale".into(), "en".into());
        let r = resolve_server_locale(&req, &opts());
        assert_eq!(r.code, "de");
        assert_eq!(r.source, LocaleSource::Path);
    }

    #[test]
    fn query_beats_accept() {
        let req = Request::builder()
            .method(Method::GET)
            .path("/blog")
            .header("accept-language", "de")
            .build();
        let mut req = req;
        req.query.insert("locale".into(), "en".into());
        let mut o = opts();
        o.path_prefix = false;
        let r = resolve_server_locale(&req, &o);
        assert_eq!(r.code, "en");
        assert_eq!(r.source, LocaleSource::Query);
    }

    #[test]
    fn accept_language_negotiation() {
        assert_eq!(
            negotiate_accept_language(
                "fr-CH, fr;q=0.9, de;q=0.8, en;q=0.7",
                &["en".into(), "de".into()]
            ),
            Some("de".into())
        );
    }
}
