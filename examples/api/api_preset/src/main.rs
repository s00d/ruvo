//! Minimal API via `App::api()` preset (extractors + Problem+ + idempotency).
//!
//! ```ignore
//! cargo run -p api_preset
//! ```

use serde::Deserialize;
use sova::extract::{Json as ReqJson, State};
use sova::vld;
use sova::{
    doc_schema, App, AppStore, Cell, Doc, DocVldExt, Idempotency, Json, KvStore, OpenApiDocExt,
    ResponseCache, SharedStore,
};
use std::sync::Arc;

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Ping {
        pub message: String => vld::string().min(1).max(100),
    }
}

doc_schema!(Ping);

/// Serde body for `extract::Json` (vld schemas use `VldParse`, not `Deserialize`).
#[derive(Debug, Deserialize)]
struct PingBody {
    message: String,
}

#[derive(Clone)]
struct Hits {
    n: Cell<u64>,
}

impl Default for Hits {
    fn default() -> Self {
        Self { n: Cell::new(0) }
    }
}

#[tokio::main]
async fn main() -> sova::Result<()> {
    let store = AppStore::memory();
    let mut app = App::api().title("Ping API").version("1.0");
    app.state(Hits::default());
    app.install(SharedStore::new(
        Arc::clone(&store.inner) as Arc<dyn KvStore>
    ));
    app.install(Idempotency::from_store(Arc::clone(&store.inner)));
    app.install(
        ResponseCache::new(Arc::clone(&store.inner)).ttl(std::time::Duration::from_secs(30)),
    );

    app.post("/ping", ping)
        .doc(Doc::new().body::<Ping>().ok::<Ping>());
    app.get("/hits", hits);

    println!("api on :3000 — docs at /docs; POST /ping with Idempotency-Key");
    app.listen(3000).await
}

async fn ping(ReqJson(body): ReqJson<PingBody>, State(hits): State<Hits>) -> Json<Ping> {
    hits.n.update(|v| v + 1);
    Json(Ping {
        message: body.message,
    })
}

async fn hits(State(hits): State<Hits>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "hits": hits.n.get() }))
}
