//! Collect request snapshot + inject thin host bridge (not the full Vue app).

use crate::collector::{DevToolsBag, RateLimitSnap, RouteSnap};
#[cfg(any(feature = "session", feature = "auth", feature = "passport"))]
use crate::collector::AuthSnap;
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
            collect_response_meta(&bag, &res);

            #[cfg(feature = "i18n")]
            bag.set_locale(locale);
            #[cfg(feature = "csrf")]
            bag.set_csrf(Some(csrf_present));

            #[cfg(feature = "session")]
            collect_session_auth(
                &bag,
                session,
                #[cfg(feature = "auth")]
                user.clone(),
                #[cfg(feature = "passport")]
                passport.clone(),
            );

            #[cfg(all(not(feature = "session"), any(feature = "auth", feature = "passport")))]
            fill_auth_without_session(
                &bag,
                #[cfg(feature = "auth")]
                user,
                #[cfg(feature = "passport")]
                passport,
            );

            #[cfg(feature = "mail")]
            {
                use crate::collector::MailLine;
                if let Some(client) = mail {
                    if let Some(fake) = client.fake() {
                        for m in fake.sent().into_iter().rev().take(5) {
                            bag.push_mail(MailLine {
                                to: m.to,
                                subject: m.subject,
                                backend: "fake".into(),
                            });
                        }
                    }
                }
            }

            let status = res.status_code().as_u16();
            let ms = bag.started.elapsed().as_secs_f64() * 1000.0;
            bag.push_log(crate::collector::LogLine {
                level: "INFO".into(),
                target: "http.server".into(),
                message: format!("{} {} → {status} ({ms:.0}ms)", bag.method, bag.path),
                request_id: Some(bag.request_id.clone()),
                at_ms: crate::collector::now_ms(),
            });
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

fn collect_response_meta(bag: &DevToolsBag, res: &Response) {
    let h = res.headers();
    let encoding = h
        .get(http::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    bag.set_encoding(encoding);

    let limit = h
        .get("ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    let remaining = h
        .get("ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    let reset = h
        .get("ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    if limit.is_some() || remaining.is_some() || reset.is_some() {
        bag.set_rate_limit(Some(RateLimitSnap {
            limit,
            remaining,
            reset,
        }));
    }
}

#[cfg(feature = "session")]
fn collect_session_auth(
    bag: &DevToolsBag,
    session: Option<sova_session::Session>,
    #[cfg(feature = "auth")] user: Option<sova_auth::CurrentUser>,
    #[cfg(feature = "passport")] passport: Option<sova_passport::Authenticated>,
) {
    use crate::redact::mask_value;
    if let Some(sess) = session {
        let mut keys = Vec::new();
        for (k, v) in sess.data() {
            keys.push((k.clone(), mask_value(&k, &v)));
        }
        keys.sort_by(|a, b| a.0.cmp(&b.0));
        #[allow(unused_mut)]
        let mut auth = AuthSnap {
            session_id: Some(sess.id()),
            user_id: sess.user_id(),
            email: None,
            roles: Vec::new(),
            session_keys: keys,
        };
        #[cfg(feature = "auth")]
        if let Some(u) = user {
            auth.user_id = Some(u.id.to_string());
            auth.email = Some(u.email.clone());
            auth.roles = u.roles.clone();
        }
        #[cfg(feature = "passport")]
        if auth.user_id.is_none() {
            if let Some(p) = passport {
                auth.user_id = Some(p.id);
            }
        }
        bag.set_auth(auth);
    } else {
        fill_auth_without_session(
            bag,
            #[cfg(feature = "auth")]
            user,
            #[cfg(feature = "passport")]
            passport,
        );
    }
}

#[cfg(any(feature = "auth", feature = "passport"))]
fn fill_auth_without_session(
    bag: &DevToolsBag,
    #[cfg(feature = "auth")] user: Option<sova_auth::CurrentUser>,
    #[cfg(feature = "passport")] passport: Option<sova_passport::Authenticated>,
) {
    #[allow(unused_mut)]
    let mut auth = AuthSnap::default();
    let mut any = false;
    #[cfg(feature = "auth")]
    if let Some(u) = user {
        auth.user_id = Some(u.id.to_string());
        auth.email = Some(u.email);
        auth.roles = u.roles;
        any = true;
    }
    #[cfg(feature = "passport")]
    if let Some(p) = passport {
        if auth.user_id.is_none() {
            auth.user_id = Some(p.id);
            any = true;
        }
    }
    if any {
        bag.set_auth(auth);
    }
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
