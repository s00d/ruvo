//! JSON endpoints under `/_i18n`.

use crate::ext::I18nState;
use crate::store::{select_tree, ROOT_SCOPE};
use sova_core::{Request, Response};
use serde_json::json;
use std::hash::{Hash, Hasher};

pub async fn locales_json(req: Request) -> Response {
    let Some(state) = req.try_state::<I18nState>() else {
        return Response::text("i18n not installed").status(500);
    };
    let store = state.store.load();
    let body = json!({
        "locales": store.locales,
        "version": store.version.as_ref(),
        "fallback": state.fallback.as_ref(),
    });
    with_cache_headers(Response::json(&body), &req, None, store.version.as_ref())
}

pub async fn all_json(req: Request) -> Response {
    let Some(state) = req.try_state::<I18nState>() else {
        return Response::text("i18n not installed").status(500);
    };
    let store = state.store.load();
    if !req
        .try_state::<crate::AllJsonEnabled>()
        .map(|f| f.0)
        .unwrap_or(false)
    {
        return Response::text("Not Found").status(404);
    }
    let mut map = serde_json::Map::new();
    for ((locale, scope), s) in &store.scopes {
        let key = if scope.as_ref() == ROOT_SCOPE {
            locale.to_string()
        } else {
            format!("{locale}/{scope}")
        };
        map.insert(
            key,
            serde_json::from_slice(&s.payload).unwrap_or(serde_json::Value::Null),
        );
    }
    with_cache_headers(
        Response::json(&serde_json::Value::Object(map)),
        &req,
        None,
        store.version.as_ref(),
    )
}

pub async fn missing_json(req: Request) -> Response {
    let Some(state) = req.try_state::<I18nState>() else {
        return Response::text("i18n not installed").status(500);
    };
    Response::json(&state.missing_snapshot())
}

pub async fn locale_or_scope_json(req: Request) -> Response {
    let Some(state) = req.try_state::<I18nState>() else {
        return Response::text("i18n not installed").status(500);
    };
    let store = state.store.load();

    let locale = req
        .param("locale")
        .unwrap_or("")
        .trim_end_matches(".json")
        .to_string();
    let scope = req
        .param("scope")
        .map(|s| s.trim_end_matches(".json").to_string())
        .unwrap_or_else(|| ROOT_SCOPE.to_string());

    if locale.is_empty() || locale == "_missing" {
        return Response::text("Not Found").status(404);
    }

    let Some(scope_data) = store.get(&locale, &scope) else {
        return Response::text("Not Found").status(404);
    };

    let prefix = req.query.get("prefix").map(|s| s.as_str());
    let keys = req.query.get("keys").map(|s| {
        s.split(',')
            .map(|k| k.trim())
            .filter(|k| !k.is_empty())
            .collect::<Vec<_>>()
    });

    let (body, etag) = if prefix.is_some() || keys.is_some() {
        let keys_ref = keys.as_deref();
        let tree = select_tree(&scope_data.tree, prefix, keys_ref);
        let bytes = serde_json::to_vec(&tree).unwrap_or_default();
        let etag = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut h);
            format!("\"{:016x}\"", h.finish())
        };
        (bytes, etag)
    } else {
        (
            scope_data.payload.to_vec(),
            scope_data.etag.to_string(),
        )
    };

    if let Some(inm) = req.header("if-none-match") {
        if inm.split(',').any(|t| t.trim() == etag) {
            return Response::empty()
                .status(304)
                .header("etag", &etag)
                .header("content-language", &locale);
        }
    }

    let res = Response::bytes(body, "application/json")
        .header("etag", &etag)
        .header("content-language", &locale);
    with_cache_headers(res, &req, Some(&etag), store.version.as_ref())
}

fn with_cache_headers(
    mut res: Response,
    req: &Request,
    etag: Option<&str>,
    version: &str,
) -> Response {
    if let Some(etag) = etag {
        res = res.header("etag", etag);
    }
    let immutable = req
        .query
        .get("v")
        .map(|v| v.as_str() == version)
        .unwrap_or(false);
    if immutable {
        res = res.header(
            "cache-control",
            "public, max-age=31536000, immutable",
        );
    } else {
        res = res.header("cache-control", "no-cache");
    }
    res
}
