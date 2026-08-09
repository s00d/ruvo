//! Embedded redb KvStore via `SharedStore::redb` — sessions + rate-limit without Redis.
//!
//! ```bash
//! cargo run -p redb_demo
//! # curl -c /tmp/c -b /tmp/c http://127.0.0.1:3012/
//! # curl -c /tmp/c -b /tmp/c http://127.0.0.1:3012/hit
//! ```

use sova::prelude::*;
use sova::{AppStore, RateLimit, SessionExt, SessionLayer, SharedStore};
use serde_json::json;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::var("SOVA_REDB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data/kv.redb"));

    let mut app = App::new();
    app.install(SharedStore::redb(&path));
    let sessions = SessionLayer::from_app_store(&app);
    app.install(sessions);
    let rate_limit = RateLimit::shared(&app, 100, std::time::Duration::from_secs(60));
    app.install(rate_limit);

    app.get("/", |req: Request| async move {
        let visits = {
            let s = req.session();
            let n: u64 = s.get("visits").and_then(|v| v.parse().ok()).unwrap_or(0) + 1;
            s.set("visits", n.to_string());
            n
        };
        Ok::<_, Error>(Json(json!({
            "ok": true,
            "backend": "redb",
            "visits": visits,
            "hint": "GET /hit uses AppStore namespace; cookie sid is session"
        })))
    });

    app.get("/hit", |req: Request| async move {
        let store = req
            .try_state::<AppStore>()
            .ok_or_else(|| Error::Internal("AppStore missing".into()))?;
        let n = store.namespaced("demo").incr("hits", 1, None).await;
        Ok::<_, Error>(Json(json!({ "hits": n })))
    });

    println!("redb at {} — http://127.0.0.1:3012/", path.display());
    app.listen(3012).await
}
