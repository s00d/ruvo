---
title: tasks
editLink: false
---

# `tasks`

**Job worker, priorities, and optional cron/interval scheduler** · crate `ruvo-tasks` · id `tasks`

```bash
cargo add ruvo --features tasks
```

| Feature | What you get |
|---------|-------------|
| `tasks` | Job queue, worker, scheduler, Console CLI (`ruvo-tasks`). |

Background task worker + optional scheduler + HTTP dispatch for Ruvo.

## Usage

Tasks is its own runtime concern (queues, CLI `tasks …`, optional HTTP enqueue). Typical worker binary:

```rust
use ruvo::prelude::*;
use ruvo::{ask, bearer_guard, info, Job, Parser, ServerArgs, Tasks};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::new();
    let _ = app.configure_from_path("ruvo.toml");

    app.install(
        Tasks::new(Arc::new(ruvo::tasks::Memory::new()))
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

    app.get("/", || async {
        "Use CLI: tasks list | tasks schedule | tasks run greet"
    });

    app.run().await
}
```

```toml
# ruvo.toml — overrides Job::every in code
[schedule.ping]
every = "15s"
```

```bash
cargo run -p tasks
cargo run -p tasks -- tasks list
cargo run -p tasks -- tasks run greet
```

From a web/API app after install: `req.dispatch("welcome_email", json!({…})).await`. SQL/Redis: `tasks-sql` / `tasks-redis` (cabinet wires `tasks::Sql::from_db_pool`).
