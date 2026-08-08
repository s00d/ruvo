//! Mount the same router under locale prefixes.

use sova_core::Router;

/// How to apply locale path prefixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixMode {
    /// Mount under `/en`, `/de`, … for every locale.
    Prefix,
    /// Default locale has no prefix; others do.
    PrefixExceptDefault,
}

/// Strip a leading `/{locale}` segment when it matches a known locale code.
pub fn strip_locale_prefix(path: &str, locales: &[impl AsRef<str>]) -> String {
    let path = if path.is_empty() { "/" } else { path };
    let trimmed = path.trim_start_matches('/');
    let mut parts = trimmed.splitn(2, '/');
    let first = parts.next().unwrap_or("");
    if first.is_empty() {
        return "/".into();
    }
    if locales.iter().any(|l| l.as_ref() == first) {
        match parts.next() {
            Some(rest) if !rest.is_empty() => format!("/{rest}"),
            _ => "/".into(),
        }
    } else {
        path.to_string()
    }
}

/// Build a localized path for `locale` from an unprefixed `path`.
///
/// When `path_prefix` is false (query/cookie locale), the path is unchanged.
/// When true, behaves like [`PrefixMode::PrefixExceptDefault`].
pub fn localize_path(path: &str, locale: &str, default: &str, path_prefix: bool) -> String {
    let path = if path.is_empty() { "/" } else { path };
    if !path_prefix {
        return path.to_string();
    }
    if locale == default {
        return path.to_string();
    }
    if path == "/" {
        format!("/{locale}/")
    } else {
        format!("/{locale}{path}")
    }
}

/// Absolute URL for a localized path.
pub fn localized_url(
    public: &str,
    path: &str,
    locale: &str,
    default: &str,
    path_prefix: bool,
) -> String {
    let public = public.trim_end_matches('/');
    let loc = localize_path(path, locale, default, path_prefix);
    if loc == "/" {
        format!("{public}/")
    } else {
        format!("{public}{loc}")
    }
}

/// Mount `routes` under locale prefixes onto `app_router`.
///
/// `routes` is cloned via re-registration: pass a builder closure instead.
pub fn mount_localized<F>(
    app: &mut Router,
    locales: &[String],
    default: &str,
    mode: PrefixMode,
    mut build: F,
) where
    F: FnMut(&mut Router),
{
    match mode {
        PrefixMode::Prefix => {
            for loc in locales {
                let mut child = Router::new();
                build(&mut child);
                app.mount(&format!("/{loc}"), child);
            }
        }
        PrefixMode::PrefixExceptDefault => {
            build(app);
            for loc in locales {
                if loc == default {
                    continue;
                }
                let mut child = Router::new();
                build(&mut child);
                app.mount(&format!("/{loc}"), child);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_and_localize() {
        let locales = ["en", "de"];
        assert_eq!(strip_locale_prefix("/de/blog", &locales), "/blog");
        assert_eq!(strip_locale_prefix("/blog", &locales), "/blog");
        assert_eq!(strip_locale_prefix("/de", &locales), "/");
        assert_eq!(localize_path("/blog", "en", "en", true), "/blog");
        assert_eq!(localize_path("/blog", "de", "en", true), "/de/blog");
        assert_eq!(localize_path("/", "de", "en", true), "/de/");
        assert_eq!(localize_path("/blog", "de", "en", false), "/blog");
    }
}
