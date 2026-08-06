//! Task worker example — enqueue via HTTP with bearer guard.

use ruvo::prelude::*;
use ruvo::{bearer_guard, Tasks};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let store = Arc::new(ruvo::tasks::Memory::new());
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
    app.get("/", || async {
        "POST /_tasks/enqueue with Authorization: Bearer secret"
    });
    app.listen(3010).await
}
