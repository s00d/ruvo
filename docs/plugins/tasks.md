---
title: tasks
editLink: false
---

# `tasks`

**Job worker, priorities, and optional cron/interval scheduler**

| | |
|--|--|
| Crate | [`sova-tasks`](https://docs.rs/sova-tasks/0.1.1) `0.1.1` |
| Plugin id | `tasks` |
| Category | Data |

## Install

```bash
cargo add sova --features tasks
```

## Features

| Feature | What you get |
|---------|-------------|
| `tasks` | Job queue, worker, scheduler, `tasks` CLI. |

## Overview

**When:** background jobs, priorities, cron / interval schedules.

**Does:**
- Dispatch jobs from handlers
- CLI: `tasks list` / `schedule` / `run NAME`
- Toml `[schedule.*]` overrides

### Example

```rust
app.install(Tasks::new().register(Ping));
// dispatch:
Ping.dispatch(&req).await?;
```

### Config

```toml
[schedule.ping]
every = "15s"
```

## Quick start

Tasks is its own runtime concern (queues, CLI `tasks …`, optional HTTP enqueue). Typical worker binary:

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

    app.get("/", || async {
        "Use CLI: tasks list | tasks schedule | tasks run greet"
    });

    app.run().await
}
```

```toml
# sova.toml — overrides Job::every in code
[schedule.ping]
every = "15s"
```

```bash
cargo run -p tasks
cargo run -p tasks -- tasks list
cargo run -p tasks -- tasks run greet
```

From a web/API app after install: `req.dispatch("welcome_email", json!({…})).await`. SQL/Redis: `tasks-sql` / `tasks-redis` (cabinet wires `tasks::Sql::from_db_pool`).

## Examples

- `examples/misc/tasks`

## Related

[`tasks-store`](/plugins/tasks-store) · [`db`](/plugins/db) · [`redis`](/plugins/redis)
