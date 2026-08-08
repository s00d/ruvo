//! Canonical URL helpers.

use crate::defaults::TrailingSlash;
use url::form_urlencoded;

const STRIP_PREFIXES: &[&str] = &["utm_"];
const STRIP_KEYS: &[&str] = &[
    "fbclid", "gclid", "yclid", "mc_eid", "mc_cid", "_ga", "ref",
];

pub fn strip_tracking(path: &str, raw_query: &str) -> String {
    if raw_query.is_empty() {
        return path.to_string();
    }
    let kept: Vec<(String, String)> = form_urlencoded::parse(raw_query.as_bytes())
        .filter(|(k, _)| {
            let key = k.as_ref();
            !STRIP_KEYS.iter().any(|s| key.eq_ignore_ascii_case(s))
                && !STRIP_PREFIXES
                    .iter()
                    .any(|p| key.to_ascii_lowercase().starts_with(p))
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{}", form_urlencoded::Serializer::new(String::new()).extend_pairs(&kept).finish())
    }
}

pub fn apply_slash(path: &str, policy: TrailingSlash) -> String {
    if path == "/" {
        return path.to_string();
    }
    match policy {
        TrailingSlash::Keep => path.to_string(),
        TrailingSlash::Always => {
            if path.ends_with('/') {
                path.to_string()
            } else {
                format!("{path}/")
            }
        }
        TrailingSlash::Never => path.trim_end_matches('/').to_string(),
    }
}

pub fn absolute_url(public_url: &str, path_or_url: &str) -> String {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        return path_or_url.to_string();
    }
    let base = public_url.trim_end_matches('/');
    if path_or_url.starts_with('/') {
        format!("{base}{path_or_url}")
    } else {
        format!("{base}/{path_or_url}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_utm() {
        assert_eq!(
            strip_tracking("/about", "utm_source=x&keep=1&fbclid=y"),
            "/about?keep=1"
        );
    }

    #[test]
    fn slash_policies() {
        assert_eq!(apply_slash("/about", TrailingSlash::Always), "/about/");
        assert_eq!(apply_slash("/about/", TrailingSlash::Never), "/about");
    }
}
