//! Request helpers and route `i18n_scope`.

use crate::plural::{pluralize, PluralFn};
use crate::store::{Store, ROOT_SCOPE};
use arc_swap::ArcSwap;
use ruvo_core::{App, Request, Router};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Page scope attached via [`I18nRouteExt::i18n_scope`].
#[derive(Debug, Clone)]
pub struct I18nScope(pub Box<str>);

impl I18nScope {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Resolved locale code on the request.
#[derive(Debug, Clone)]
pub struct LocaleCode(pub Box<str>);

/// Shared runtime used by middleware and `I18nExt`.
#[derive(Clone)]
pub struct I18nState {
    pub store: Arc<ArcSwap<Store>>,
    pub fallback: Box<str>,
    pub plural_fn: Option<PluralFn>,
    pub missing_handler: Option<MissingHandler>,
    pub missing_keys: Arc<Mutex<HashSet<String>>>,
}

pub type MissingHandler = Arc<dyn Fn(&str, &str, &str) + Send + Sync>;

impl I18nState {
    pub fn translate(&self, locale: &str, scope: &str, key: &str) -> String {
        if let Some(v) = self.lookup(locale, scope, key) {
            return v;
        }
        self.record_missing(locale, scope, key);
        if let Some(h) = &self.missing_handler {
            h(locale, scope, key);
        }
        key.to_string()
    }

    pub fn has(&self, locale: &str, scope: &str, key: &str) -> bool {
        self.lookup(locale, scope, key).is_some()
    }

    fn lookup(&self, locale: &str, scope: &str, key: &str) -> Option<String> {
        let store = self.store.load();
        let fallback = self.fallback.as_ref();
        for (loc, sc) in [
            (locale, scope),
            (locale, ROOT_SCOPE),
            (fallback, scope),
            (fallback, ROOT_SCOPE),
        ] {
            if let Some(v) = store.lookup_flat(loc, sc, key) {
                return Some(v);
            }
        }
        None
    }

    fn record_missing(&self, locale: &str, scope: &str, key: &str) {
        let id = format!("{locale}|{scope}|{key}");
        let mut set = self.missing_keys.lock().unwrap();
        if set.insert(id) {
            tracing::warn!(locale, scope, key, "i18n missing translation");
        }
    }

    pub fn missing_snapshot(&self) -> Vec<String> {
        let mut v: Vec<_> = self.missing_keys.lock().unwrap().iter().cloned().collect();
        v.sort();
        v
    }
}

/// Attach an i18n page scope to the last registered route.
pub trait I18nRouteExt {
    fn i18n_scope(&mut self, scope: impl Into<String>) -> &mut Self;
}

impl I18nRouteExt for Router {
    fn i18n_scope(&mut self, scope: impl Into<String>) -> &mut Self {
        self.route_meta(I18nScope(scope.into().into_boxed_str()))
    }
}

impl I18nRouteExt for App {
    fn i18n_scope(&mut self, scope: impl Into<String>) -> &mut Self {
        Router::route_meta(self, I18nScope(scope.into().into_boxed_str()));
        self
    }
}

/// Translate helpers on [`Request`].
pub trait I18nExt {
    fn locale(&self) -> &str;
    fn i18n_scope_name(&self) -> String;
    fn t(&self, key: &str) -> String;
    fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String;
    fn tn(&self, key: &str, count: i64) -> String;
    fn has_t(&self, key: &str) -> bool;
}

impl I18nExt for Request {
    fn locale(&self) -> &str {
        self.get::<LocaleCode>()
            .map(|c| c.0.as_ref())
            .unwrap_or("en")
    }

    fn i18n_scope_name(&self) -> String {
        scope_of(self)
    }

    fn t(&self, key: &str) -> String {
        let Some(state) = self.try_state::<I18nState>() else {
            return key.to_string();
        };
        state.translate(self.locale(), &scope_of(self), key)
    }

    fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        interpolate(&self.t(key), args)
    }

    fn tn(&self, key: &str, count: i64) -> String {
        let Some(state) = self.try_state::<I18nState>() else {
            return key.to_string();
        };
        let scope = scope_of(self);
        let raw = state.translate(self.locale(), &scope, key);
        pluralize(state.plural_fn.as_ref(), key, count, self.locale(), &raw)
    }

    fn has_t(&self, key: &str) -> bool {
        let Some(state) = self.try_state::<I18nState>() else {
            return false;
        };
        state.has(self.locale(), &scope_of(self), key)
    }
}

fn scope_of(req: &Request) -> String {
    req.route_meta::<I18nScope>()
        .map(|s| s.0.to_string())
        .unwrap_or_else(|| ROOT_SCOPE.to_string())
}

/// Replace `{token}` placeholders; missing tokens stay as `{token}`.
pub fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_keeps_missing_token() {
        assert_eq!(
            interpolate("Hello {name}, {x}", &[("name", "Ada")]),
            "Hello Ada, {x}"
        );
    }
}
