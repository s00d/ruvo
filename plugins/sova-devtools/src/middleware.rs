//! Collect request snapshot + inject thin host bridge (not the full Vue app).

use crate::collector::DevToolsBag;
use crate::hub::{next_id, DevToolsHub};
use crate::inject::inject_body;
use sova_core::extend::named;
use sova_core::{Next, Request, RequestId, Response};

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

            #[cfg(feature = "session")]
            let session = req.get::<sova_session::Session>().cloned();
            #[cfg(feature = "mail")]
            let mail = req.try_state::<sova_mail::MailClient>();
            #[cfg(feature = "auth")]
            let user = req.get::<sova_auth::CurrentUser>().cloned();

            let mut res = next(req).await;

            #[cfg(feature = "session")]
            {
                use crate::collector::AuthSnap;
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
                        session_keys: keys,
                    };
                    #[cfg(feature = "auth")]
                    if let Some(u) = user {
                        auth.user_id = Some(u.id.to_string());
                    }
                    bag.set_auth(auth);
                } else {
                    #[cfg(feature = "auth")]
                    if let Some(u) = user {
                        bag.set_auth(AuthSnap {
                            session_id: None,
                            user_id: Some(u.id.to_string()),
                            session_keys: Vec::new(),
                        });
                    }
                }
            }

            #[cfg(all(feature = "auth", not(feature = "session")))]
            {
                use crate::collector::AuthSnap;
                if let Some(u) = user {
                    bag.set_auth(AuthSnap {
                        session_id: None,
                        user_id: Some(u.id.to_string()),
                        session_keys: Vec::new(),
                    });
                }
            }

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
            // Access logger runs outside this MW, so attach a summary line here
            // before the bag is closed (otherwise per-request Logs would miss it).
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
                // Prevent bfcache / browser Back from restoring a frozen page
                // without a server hit (otherwise Timeline stays silent).
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

/// Host marker + tiny bridge. Full Vue app lives at `/_devtools/`.
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
