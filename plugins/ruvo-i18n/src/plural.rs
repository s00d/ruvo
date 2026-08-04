//! Default pluralization (index = count) + hook.

/// Split `one|two|many` and pick a form.
pub type PluralFn = Arc<dyn Fn(&str, i64, &str, &[&str]) -> String + Send + Sync>;

use std::sync::Arc;

/// Compatible with nuxt-i18n-micro `defaultPlural`: `forms[count]` clamped.
pub fn default_plural(_key: &str, count: i64, _locale: &str, forms: &[&str]) -> String {
    if forms.is_empty() {
        return String::new();
    }
    let idx = if count < 0 {
        0
    } else if (count as usize) >= forms.len() {
        forms.len() - 1
    } else {
        count as usize
    };
    forms[idx].to_string()
}

pub fn split_forms(raw: &str) -> Vec<&str> {
    raw.split('|').collect()
}

pub fn pluralize(
    plural_fn: Option<&PluralFn>,
    key: &str,
    count: i64,
    locale: &str,
    raw: &str,
) -> String {
    let forms = split_forms(raw);
    match plural_fn {
        Some(f) => f(key, count, locale, &forms),
        None => default_plural(key, count, locale, &forms),
    }
}
