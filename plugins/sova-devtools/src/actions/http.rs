//! HTTP console — in-process app dispatch.

use super::{truncate_body, MAX_BODY};
use crate::console::DevToolsConsole;
use http::Method;
use serde::Deserialize;
use serde_json::{json, Value};
use sova_core::AppDispatch;
use sova_core::{Request, Response};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct HttpActionRequest {
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub query: HashMap<String, String>,
}

fn default_target() -> String {
    "app".into()
}

fn default_method() -> String {
    "GET".into()
}

pub async fn execute(
    incoming: &Request,
    cfg: &DevToolsConsole,
    action: HttpActionRequest,
) -> Result<Value, String> {
    if action.path.starts_with("/_devtools") {
        return Err("cannot proxy /_devtools paths".into());
    }
    if action.path.is_empty() || !action.path.starts_with('/') {
        return Err("path must start with /".into());
    }

    match action.target.as_str() {
        "app" => dispatch_app(incoming, cfg, action).await,
        "external" => dispatch_external(incoming, cfg, action).await,
        other => Err(format!("unknown target `{other}`")),
    }
}

async fn dispatch_app(
    incoming: &Request,
    cfg: &DevToolsConsole,
    action: HttpActionRequest,
) -> Result<Value, String> {
    let method = Method::from_str(&action.method.to_uppercase())
        .map_err(|_| format!("invalid method `{}`", action.method))?;

    let mut path = action.path.clone();
    if !action.query.is_empty() {
        let qs = serde_urlencoded::to_string(&action.query).map_err(|e| e.to_string())?;
        if path.contains('?') {
            path.push('&');
            path.push_str(&qs);
        } else {
            path.push('?');
            path.push_str(&qs);
        }
    }

    let mut builder = Request::builder().method(method).path(path);
    for (k, v) in &action.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    if let Some(cookie) = incoming.header("cookie") {
        builder = builder.header("cookie", cookie);
    }
    #[cfg(feature = "csrf")]
    if let Some(token) = incoming.get::<sova_csrf::CsrfToken>() {
        builder = builder.header("x-xsrf-token", token.0.as_str());
    } else if let Some(hdr) = incoming.header("x-xsrf-token") {
        builder = builder.header("x-xsrf-token", hdr);
    } else if let Some(hdr) = incoming.header("x-csrf-token") {
        builder = builder.header("x-csrf-token", hdr);
    }
    if let Some(body) = &action.body {
        if body.len() > cfg.body_limit.min(MAX_BODY) {
            return Err(format!("body exceeds limit ({} bytes)", cfg.body_limit));
        }
        builder = builder.body(body.clone());
    }
    let req = builder.build();

    let dispatch = incoming
        .try_state::<AppDispatch>()
        .ok_or_else(|| "AppDispatch not installed".to_string())?;
    let fut = dispatch
        .try_dispatch(req)
        .ok_or_else(|| "app dispatch not ready (server not started?)".to_string())?;
    let started = std::time::Instant::now();
    let mut res: Response = fut.await;
    let status = res.status_code().as_u16();
    let mut header_map = json!({});
    if let Some(obj) = header_map.as_object_mut() {
        for (name, val) in res.headers().iter() {
            obj.insert(
                name.as_str().to_string(),
                Value::String(val.to_str().unwrap_or("").to_string()),
            );
        }
    }

    let body_bytes = match res.body_bytes() {
        Some(b) => b.to_vec(),
        None => {
            use http_body_util::BodyExt;
            use sova_core::extend::Body;
            let body = res.take_body();
            match body {
                Body::Bytes(b) => b.to_vec(),
                Body::Stream(s) => s
                    .collect()
                    .await
                    .map(|c| c.to_bytes().to_vec())
                    .map_err(|e| e.to_string())?,
            }
        }
    };

    let limit = cfg.body_limit.min(MAX_BODY);
    let (body_text, truncated) = truncate_body(&body_bytes, limit);
    let duration_ms = started.elapsed().as_secs_f64() * 1000.0;

    Ok(json!({
        "status": status,
        "headers": header_map,
        "body": body_text,
        "truncated": truncated,
        "duration_ms": duration_ms,
    }))
}

#[cfg(feature = "console-http-external")]
async fn dispatch_external(
    incoming: &Request,
    cfg: &DevToolsConsole,
    action: HttpActionRequest,
) -> Result<Value, String> {
    use http::Method;
    use sova_http::HttpExt;
    use std::str::FromStr;

    if !cfg.console_external {
        return Err("external HTTP console disabled (DevTools::console_external(true))".into());
    }

    let method = Method::from_str(&action.method.to_uppercase())
        .map_err(|_| format!("invalid method `{}`", action.method))?;

    let mut url = action.path.clone();
    if !action.query.is_empty() {
        let qs = serde_urlencoded::to_string(&action.query).map_err(|e| e.to_string())?;
        if url.contains('?') {
            url.push('&');
            url.push_str(&qs);
        } else {
            url.push('?');
            url.push_str(&qs);
        }
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("external target requires absolute http(s) URL".into());
    }

    let started = std::time::Instant::now();
    let mut pending = match method {
        Method::GET => incoming.http().get(&url),
        Method::POST => incoming.http().post(&url),
        Method::PUT => incoming.http().put(&url),
        Method::PATCH => incoming.http().patch(&url),
        Method::DELETE => incoming.http().delete(&url),
        _ => return Err(format!("unsupported method for external: {method}")),
    };

    for (k, v) in &action.headers {
        pending = pending.header(k, v);
    }
    if let Some(body) = &action.body {
        if body.len() > cfg.body_limit.min(MAX_BODY) {
            return Err(format!("body exceeds limit ({} bytes)", cfg.body_limit));
        }
        pending = pending.body(body.clone());
    }

    let res = pending.send().await.map_err(|e| e.to_string())?;
    let status = res.status_u16();
    let mut header_map = json!({});
    if let Some(obj) = header_map.as_object_mut() {
        for (name, val) in res.headers().iter() {
            obj.insert(
                name.as_str().to_string(),
                Value::String(val.to_str().unwrap_or("").to_string()),
            );
        }
    }
    let limit = cfg.body_limit.min(MAX_BODY);
    let (body_text, truncated) = truncate_body(res.bytes(), limit);
    Ok(json!({
        "status": status,
        "headers": header_map,
        "body": body_text,
        "truncated": truncated,
        "duration_ms": started.elapsed().as_secs_f64() * 1000.0,
    }))
}

#[cfg(not(feature = "console-http-external"))]
async fn dispatch_external(
    _incoming: &Request,
    cfg: &DevToolsConsole,
    _action: HttpActionRequest,
) -> Result<Value, String> {
    if !cfg.console_external {
        Err("external HTTP console disabled (DevTools::console_external(true))".into())
    } else {
        Err("external HTTP requires devtools console-http-external feature".into())
    }
}
