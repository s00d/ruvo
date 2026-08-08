//! MiniJinja bridge: ambient `t(...)` in templates.

use crate::ext::{interpolate, scope_of, I18nState, LocaleCode};
use crate::plural::pluralize;
use minijinja::value::Kwargs;
use minijinja::Value;
use sova_core::Request;

/// Build a MiniJinja callable `t(...)` for the current request.
///
/// Wire it into templates with `.per_request("t", sova_i18n::template_fn)`.
///
/// Template usage:
/// - `{{ t("nav.about") }}`
/// - `{{ t("greet", name="Ada") }}`
/// - `{{ t("cart.items", count=3) }}`
pub fn template_fn(req: &Request) -> Value {
    let locale = req
        .get::<LocaleCode>()
        .map(|c| c.0.to_string())
        .unwrap_or_else(|| "en".to_string());
    let scope = scope_of(req);
    let state = req.try_state::<I18nState>();

    Value::from_function(move |key: String, kwargs: Kwargs| -> Result<String, minijinja::Error> {
        let Some(state) = state.as_ref() else {
            kwargs.assert_all_used()?;
            return Ok(key);
        };

        let mut text = if kwargs.has("count") {
            let count: i64 = kwargs.get("count")?;
            let raw = state.translate(&locale, &scope, &key);
            pluralize(state.plural_fn.as_ref(), &key, count, &locale, &raw)
        } else {
            state.translate(&locale, &scope, &key)
        };

        let mut pairs: Vec<(String, String)> = Vec::new();
        for name in kwargs.args() {
            if name == "count" {
                continue;
            }
            let val: Value = kwargs.get(name)?;
            pairs.push((name.to_string(), kwarg_to_string(&val)));
        }
        kwargs.assert_all_used()?;

        if !pairs.is_empty() {
            let refs: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            text = interpolate(&text, &refs);
        }

        Ok(text)
    })
}

fn kwarg_to_string(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    if let Some(n) = v.as_i64() {
        return n.to_string();
    }
    v.to_string()
}
