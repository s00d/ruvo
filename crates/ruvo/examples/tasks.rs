//! Task worker example — enqueue via HTTP with bearer guard.

use ruvo::{Bind, bearer_guard, init_tracing, App, Request, Response, Result, TaskMemoryStore, Tasks};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let store = Arc::new(TaskMemoryStore::new());
    let mut app = App::new();
    app.install(
        Tasks::new(store)
            .on("ping", |_task| async move {
                tracing::info!("ping handled");
                Ok(())
            })
            .exposed()
            .guard(bearer_guard("secret")),
    );
    app.get("/", |_req: Request| async {
        Response::text("POST /_tasks/enqueue with Authorization: Bearer secret")
    });
    app.bind(Bind::Port(3010)).serve().await
}
