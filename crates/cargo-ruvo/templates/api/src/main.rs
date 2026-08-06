use ruvo::prelude::*;
use ruvo::{Parser, ServerArgs};
use ruvo_cookies::CookieLayer;
use ruvo_openapi::OpenApi;
use ruvo_session::memory_sessions;

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    ruvo_env::load().ok();
    let mut app = App::new();
    app.install(CookieLayer);
    app.install(memory_sessions());
    app.install(OpenApi::new("{{name}} API", "0.1.0"));
    app.get("/health", || async { Json(serde_json::json!({ "ok": true })) });
    modules::register(&mut app);

    app.run().await
}
