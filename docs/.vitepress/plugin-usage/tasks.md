Worker + schedule + CLI (see [Overview](#overview) for TOML keys). Minimal wiring:

```rust
use sova::prelude::*;
use sova::{ask, bearer_guard, info, Job, Parser, ServerArgs, Tasks};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::new();
    let _ = app.configure_from_path("sova.toml");

    app.install(
        Tasks::new(Arc::new(sova::tasks::Memory::new()))
            .queues(["critical", "default", "mailer"])
            .scheduler_tick(Duration::from_secs(1))
            .job(
                Job::new("ping", |_task| async move {
                    tracing::info!("ping handled");
                    Ok(())
                })
                .every(Duration::from_secs(10)),
            )
            .job(Job::new("greet", |_task| async move {
                info("greet job");
                let name = ask("Your name").unwrap_or_else(|_| "world".into());
                info(&format!("hello, {name}"));
                Ok(())
            }))
            .exposed()
            .guard(bearer_guard("secret")),
    );

    app.run().await
}
```

```toml
# sova.toml — overrides Job::every / Job::cron for named jobs
[schedule.ping]
every = "15s"

[schedule.mail_digest]
cron = "0 */5 * * * *"
queue = "mailer"
priority = -100
```

```bash
cargo run -p tasks
cargo run -p tasks -- tasks list
cargo run -p tasks -- tasks schedule
cargo run -p tasks -- tasks run greet
```

Dispatch from a web/API handler after install:

```rust
use sova::{Dispatch, TaskBackend};
use serde_json::json;

if let Some(tasks) = req.try_state::<TaskBackend>() {
    tasks
        .dispatch(Dispatch::new("welcome_email").data(json!({ "email": "a@b.c" })))
        .await?;
}
```

SQL/Redis stores: features `tasks-sql` / `tasks-redis` — see [tasks-store](/plugins/tasks-store). Cabinet wires `sova::tasks::Sql::from_db_pool`.
