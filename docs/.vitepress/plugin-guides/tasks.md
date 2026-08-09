**When:** background jobs, priorities, cron / interval schedules, CLI `tasks …`.

**Does:**
- Worker over a `TaskStore` (memory / file / sql / redis)
- Register jobs with `Job::new` + optional `.every` / `.cron` / `.queue` / `.priority`
- Schedule overrides from `sova.toml` → `[schedule.<job>]`
- Dispatch via `TaskBackend` + `Dispatch` (handlers / HTTP)
- CLI: `tasks list` | `schedule` | `run NAME`
- Optional `POST /_tasks/enqueue` (`.exposed().guard(...)`)

### Example

```rust
use sova::{bearer_guard, Job, Tasks};
use std::sync::Arc;
use std::time::Duration;

let _ = app.configure_from_path("sova.toml"); // required for [schedule.*]

app.install(
    Tasks::new(Arc::new(sova::tasks::Memory::new()))
        .queues(["critical", "default", "mailer"])
        .job(
            Job::new("ping", |_task| async move {
                tracing::info!("ping");
                Ok(())
            })
            .every(Duration::from_secs(10)), // overridden by toml when present
        )
        .exposed()
        .guard(bearer_guard("secret")),
);
```

### Schedule (TOML)

Load config **before** `install(Tasks…)` (`configure_from_path` / equivalent). Toml **wins** over code `.every` / `.cron` for the same job name. Unknown job names fail startup / audit.

```toml
[schedule.ping]
every = "15s"          # 15s | 2m | 1h | 500ms — either every OR cron

[schedule.mail_digest]
cron = "0 */5 * * * *" # 5 fields → seconds padded with `0 `; or 6-field
queue = "mailer"       # optional
priority = -100        # optional (LOW=-100, NORMAL=0, HIGH=100)
# payload = { "mode" = "full" }  # optional JSON data for scheduler enqueues
```

| Key | Required | Notes |
|-----|----------|--------|
| `cron` **or** `every` | one of | mutually exclusive |
| `queue` | no | else job default / first of `Tasks::queues` |
| `priority` | no | int or numeric string |
| `payload` | no | any TOML → JSON; default `{}` |

There is **no** `[tasks]` section — worker tunables are builder-only: `.lease`, `.poll_interval`, `.max_attempts`, `.retry_base`, `.scheduler_tick`.

### Dispatch (handlers)

```rust
use sova::{Dispatch, TaskBackend};
use serde_json::json;

if let Some(tasks) = req.try_state::<TaskBackend>() {
    tasks
        .dispatch(
            Dispatch::new("welcome_email")
                .data(json!({ "email": user.email }))
                .queue("mailer")      // optional
                .priority(100)       // optional
                .delay(Duration::from_secs(30)) // or .at(SystemTime)
                .dedup("welcome:42"), // optional
        )
        .await?;
}
```

### CLI

```bash
cargo run -p tasks -- tasks list
cargo run -p tasks -- tasks schedule
cargo run -p tasks -- tasks run greet
cargo run -p tasks -- tasks run greet --json '{"name":"Ada"}'
```

Console helpers (`info`, `ask`, `confirm`, `table`, …) are live **only** under `tasks run`. Worker / HTTP dispatch stay quiet.

### HTTP enqueue

With `.exposed().guard(bearer_guard("secret"))`:

`POST /_tasks/enqueue` + `Authorization: Bearer secret`

```json
{
  "name": "ping",
  "payload": {},
  "queue": "default",
  "priority": 0,
  "delay_secs": 10,
  "run_at": "2026-08-09T12:00:00Z",
  "dedup_key": "ping:1"
}
```

### Notes
- Queues claim order = `Tasks::queues([...])` (first = highest). Within a queue, higher `priority` wins.
- Defaults: queues=`["default"]`, lease=30s, poll=200ms, max_attempts=5, retry_base=5s, scheduler_tick=1s.
- See `examples/misc/tasks` and cabinet (`tasks::Sql::from_db_pool`).
