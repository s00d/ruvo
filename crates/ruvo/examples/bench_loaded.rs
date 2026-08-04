//! Loaded scale profile: logger + cors + cookies + session + rate-limit.
//! Used by `bench/scale.sh` — not a product demo.

use ruvo::{Bind, init_tracing, logger, App, Cors, RateLimit, Request, Response, Result, SessionExt};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let mut app = App::new();
    app.use_middleware(logger());
    app.install(Cors::new().origin("*"));
    app.install(ruvo::memory_sessions());
    // Cap far above load-test RPS so we measure lock cost, not 429s.
    app.install(RateLimit::new(10_000_000, Duration::from_secs(60)));

    app.get("/", |req: Request| async move {
        let n = req.session().get_or("hits", "0");
        let hits: u64 = n.parse().unwrap_or(0) + 1;
        req.session().set("hits", hits.to_string());
        Response::text(format!("ok {hits}"))
    });

    app.bind(Bind::Port(3001)).serve().await
}
