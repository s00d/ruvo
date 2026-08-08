//! API key via Passport strategy.
//!
//! ```bash
//! cargo run -p api_auth
//! curl -H 'x-api-key: demo' http://127.0.0.1:3000/me
//! ```

use ruvo::{App, Auth, Json, Passport, PassportExt, Request, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize)]
struct User {
    id: i64,
    email: String,
}

#[derive(Clone)]
struct Keys(Arc<HashMap<String, User>>);

fn build_app() -> App {
    let keys = Keys(Arc::new(HashMap::from([(
        "demo".into(),
        User {
            id: 1,
            email: "ada@example.com".into(),
        },
    )])));

    let mut app = App::new();
    app.state(keys);

    app.install(
        Passport::new().strategy(
            "api-key",
            Auth::api_key("x-api-key", |key, req| {
                let keys = Arc::clone(&req.state::<Keys>());
                async move { Ok(keys.0.get(&key).cloned()) }
            })
            .skip("/healthz")
            .skip("/ready")
            .middleware(),
        ),
    );
    app.use_middleware(Passport::authenticate("api-key"));
    app.with_probes();

    app.get("/me", |req: Request| async move {
        Ok::<_, ruvo::Error>(Json(req.require_user::<User>()?.clone()))
    });

    app
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing::info!(
        "API http://127.0.0.1:3000  try: curl -H 'x-api-key: demo' http://127.0.0.1:3000/me"
    );
    build_app().listen(3000).await
}
