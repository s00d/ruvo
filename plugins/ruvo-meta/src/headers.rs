//! HTTP headers and soft-404 / 302 warnings.

use crate::defaults::MetaDefaults;
use crate::overlay::MetaOverlay;
use crate::page::MetaPage;
use crate::resolve::resolve_parts;
use ruvo_core::extend::named;
use ruvo_core::{App, Next, Request};
use std::collections::HashSet;
use std::sync::Mutex;

static SOFT_404_WARNED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

pub fn install_headers_middleware(app: &mut App) {
    app.use_middleware(named("meta-headers", |mut req: Request, next: Next| async move {
        let page = req.route_meta::<MetaPage>().map(|p| (*p).clone());
        if req.get::<MetaOverlay>().is_none() {
            req.set(MetaOverlay::default());
        }
        // Clone Arc before `next` so handler mutations are visible after.
        let overlay = req.get::<MetaOverlay>().expect("seeded").clone();
        let defaults = req
            .try_state::<MetaDefaults>()
            .map(|d| (*d).clone())
            .unwrap_or_default();
        let path = req.path.clone();
        let query = req.raw_query().to_owned();

        let mut res = next(req).await;

        let resolved = resolve_parts(
            &defaults,
            page.as_ref(),
            &overlay.snapshot(),
            &path,
            &query,
        );

        let ct = res
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let is_html = ct.contains("text/html") || ct.is_empty();

        if resolved.noindex && !is_html {
            res = res.header("x-robots-tag", "noindex");
        }
        if let Some(ref c) = resolved.canonical {
            if !is_html {
                res = res.header("link", format!("<{c}>; rel=\"canonical\""));
            }
        }

        let status = res.status_code().as_u16();
        if status == 200 && !resolved.noindex && resolved.title.as_ref().is_none_or(|t| t.is_empty())
        {
            warn_soft_404_once(&path);
        }
        if status == 302 {
            if let Some(ref page) = page {
                if page.moved_to.is_some() {
                    tracing::warn!(
                        "meta: permanent move served as 302 — use Redirect::permanent (301)"
                    );
                }
            }
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
