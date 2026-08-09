//! Collect request snapshot + inject thin host bridge (not the full Vue app).

use crate::collector::{DevToolsBag, RouteSnap};
use crate::hooks;
use crate::hub::{next_id, DevToolsHub};
use crate::inject::inject_body;
use sova_core::extend::named;
use sova_core::{MatchedRouteCapture, Next, Request, RequestId, Response};

pub fn install(app: &mut sova_core::App, hub: DevToolsHub) {
    let hub = hub.clone();
    app.use_middleware(named("devtools", move |mut req: Request, next: Next| {
        let hub = hub.clone();
        async move {
            if req.path.starts_with("/_devtools") {
                return next(req).await;
            }

            let request_id = req
                .get::<RequestId>()
                .map(|r| r.0.clone())
                .unwrap_or_else(|| "-".into());
            let bag = DevToolsBag::new(
                next_id(),
                request_id,
                req.method.as_str().to_string(),
                req.path.clone(),
            );
            req.set(bag.clone());
            crate::plugin::open_bags::insert(&bag);

            let route_cap = MatchedRouteCapture::new();
            req.set(route_cap.clone());

            #[cfg(feature = "session")]
            let session = req.get::<sova_session::Session>().cloned();
            #[cfg(feature = "mail")]
            let mail = req.try_state::<sova_mail::MailClient>();
            #[cfg(feature = "auth")]
            let user = req.get::<sova_auth::CurrentUser>().cloned();
            #[cfg(feature = "passport")]
            let passport = req.get::<sova_passport::Authenticated>().cloned();
            #[cfg(feature = "i18n")]
            let locale = req.get::<sova_i18n::LocaleCode>().map(|l| l.0.to_string());
            #[cfg(feature = "csrf")]
            let csrf_present = req.get::<sova_csrf::CsrfToken>().is_some();

            let mut res = next(req).await;

            bag.set_route(RouteSnap {
                path: bag.path.clone(),
                pattern: route_cap.get().map(|p| p.to_string()),
                captures: Vec::new(),
            });
            hooks::collect_response_meta(&bag, &res);

            #[cfg(feature = "i18n")]
            bag.set_locale(locale);
            #[cfg(feature = "csrf")]
            bag.set_csrf(Some(csrf_present));

            #[cfg(feature = "session")]
            hooks::collect_session_auth(
                &bag,
                session,
                #[cfg(feature = "auth")]
                user.clone(),
                #[cfg(feature = "passport")]
                passport.clone(),
            );

            #[cfg(all(not(feature = "session"), any(feature = "auth", feature = "passport")))]
            hooks::fill_auth_without_session(
                &bag,
                #[cfg(feature = "auth")]
                user,
                #[cfg(feature = "passport")]
                passport,
            );

            #[cfg(feature = "mail")]
            hooks::collect_mail(&bag, mail.as_deref());

            let status = res.status_code().as_u16();
            // Access line already lands in Logs via the tracing logger hook — avoid a duplicate.
            crate::plugin::open_bags::remove(&bag.request_id);
            let snap = bag.finish(status);
            let snap_id = snap.id.clone();
            let (st, ms, sql, err) = (
                snap.status,
                snap.duration_ms,
                snap.queries.len(),
                snap.logs
                    .iter()
                    .filter(|l| l.level.to_ascii_uppercase().contains("ERROR"))
                    .count(),
            );
            hub.push_snapshot(snap);
            if maybe_inject(&mut res, &snap_id, st, ms, sql, err) {
                disable_bfcache(&mut res);
            }
            res
        }
    }));
}

fn disable_bfcache(res: &mut Response) {
    let h = res.headers_mut();
    h.insert(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
    h.insert(
        http::header::PRAGMA,
        http::HeaderValue::from_static("no-cache"),
    );
    h.insert(
        http::header::EXPIRES,
        http::HeaderValue::from_static("0"),
    );
}

fn maybe_inject(
    res: &mut Response,
    snap_id: &str,
    status: u16,
    ms: f64,
    sql: usize,
    errors: usize,
) -> bool {
    let fragment = host_html(snap_id, status, ms, sql, errors);
    res.map_buffered_html(|html| inject_body(html, &fragment))
}

fn host_html(snap_id: &str, status: u16, ms: f64, sql: usize, errors: usize) -> String {
    let status_class = if status >= 500 {
        "err"
    } else if status >= 400 {
        "warn"
    } else {
        "ok"
    };
    format!(
        r#"<div id="sova-devtools" hidden data-snap="{snap_id}" data-status="{status}" data-status-class="{status_class}" data-ms="{ms:.1}" data-sql="{sql}" data-errors="{errors}" data-events="/_devtools/events" data-api="/_devtools"></div>
<script src="/_devtools/assets/bridge.js" defer></script>"#
    )
}
