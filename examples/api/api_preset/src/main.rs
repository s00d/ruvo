//! Minimal validated API via `App::api()` preset.
//!
//! ```ignore
//! cargo run -p api_preset
//! ```

use sova::vld;
use sova::{doc_schema, App, Doc, DocVldExt, Json, OpenApiDocExt, Request, Result, ValidationError, ValidationExt};

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Ping {
        pub message: String => vld::string().min(1).max(100),
    }
}

doc_schema!(Ping);

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::api().title("Ping API").version("1.0");
    app.post("/ping", ping).doc(Doc::new().body::<Ping>().ok::<Ping>());
    println!("api on :3000 — docs at /docs, probes /healthz /ready");
    app.listen(3000).await
}

async fn ping(mut req: Request) -> std::result::Result<Json<Ping>, ValidationError> {
    let body: Ping = req.validate().await?;
    Ok(Json(body))
}
