//! HTTP headers, HTML head inject, soft-404 / moved_to redirects.

use crate::defaults::MetaDefaults;
use crate::html::render_html;
use crate::inject::inject_head;
use crate::overlay::MetaOverlay;
use crate::page::MetaPage;
use crate::resolve::resolve_parts;
use sova_core::extend::{named, Body, MatchedMetaCapture};
use sova_core::{App, IntoResponse, Next, Redirect, Request};
use std::collections::HashSet;
use std::sync::Mutex;

static SOFT_404_WARNED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

pub fn install_headers_middleware(app: &mut App) {
    app.use_middleware(named("meta-headers", |mut req: Request, next: Next| async move {
        if req.get::<MetaOverlay>().is_none() {
            req.set(MetaOverlay::default());
        }
        let overlay = req.get::<MetaOverlay>().expect("seeded").clone();
        let defaults = req
            .try_state::<MetaDefaults>()
            .map(|d| (*d).clone())
            .unwrap_or_default();
        let path = req.path.clone();
        let query = req.raw_query().to_owned();
        let capture = MatchedMetaCapture::new();
        req.set(capture.clone());

        #[cfg(feature = "i18n")]
        let i18n = {
            use sova_i18n::{I18nExt, I18nState};
            let state = req.try_state::<I18nState>().map(|s| (*s).clone());
            let locale = req.locale().to_string();
            (state, locale)
        };

        let mut res = next(req).await;

        let page = capture
            .get()
            .and_then(|m| m.get::<MetaPage>().map(|p| (*p).clone()));

        if let Some(ref page) = page {
            if let Some(ref to) = page.moved_to {
                return Redirect::permanent(to.clone()).into_response();
            }
        }

        let snap = overlay.snapshot();
        let manual = page.as_ref().is_some_and(|p| p.manual) || snap.manual;
        #[allow(unused_mut)]
        let mut resolved = resolve_parts(&defaults, page.as_ref(), &snap, &path, &query);

        #[cfg(feature = "i18n")]
        if let Some(ref state) = i18n.0 {
            crate::i18n_meta::enrich_parts(state, &i18n.1, &path, &defaults, &mut resolved);
        }

        let ct = res
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let is_html = ct.contains("text/html");

        if resolved.noindex && !is_html {
            res = res.header("x-robots-tag", "noindex");
        }
        if let Some(ref c) = resolved.canonical {
            if !is_html {
                res = res.header("link", format!("<{c}>; rel=\"canonical\""));
            }
        }

        if is_html && !manual {
            let body = res.take_body();
            match body {
                Body::Bytes(bytes) => {
                    if let Ok(html) = std::str::from_utf8(&bytes) {
                        let fragment = render_html(&resolved);
                        if let Some(new_html) = inject_head(html, &fragment) {
                            res.set_body(new_html);
                        } else {
                            res.set_body(bytes);
                        }
                    } else {
                        res.set_body(bytes);
                    }
                }
                other => res.set_body(other),
            }
        }

        let status = res.status_code().as_u16();
        if status == 200 && !resolved.noindex && resolved.title.as_ref().is_none_or(|t| t.is_empty())
        {
            warn_soft_404_once(&path);
        }

        res
    }));
}

fn warn_soft_404_once(path: &str) {
    let mut slot = SOFT_404_WARNED.lock().unwrap();
    let set = slot.get_or_insert_with(HashSet::new);
    if set.insert(path.to_string()) {
        tracing::warn!("meta: 200 response without title (possible soft-404) path={path}");
    }
}
