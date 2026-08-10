//! Task worker + Console CLI + toml schedule example.
//!
//! ```bash
//! cargo run -p tasks
//! cargo run -p tasks -- tasks list
//! cargo run -p tasks -- tasks schedule
//! cargo run -p tasks -- tasks run greet
//! # POST /_tasks/enqueue with Authorization: Bearer secret
//! ```

use sova::prelude::*;
use sova::{ask, bearer_guard, confirm, info, is_interactive, priority, table, Job, Tasks};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let store = Arc::new(sova::tasks::Memory::new());
    let mut app = App::new();
    let _ = app.configure_from_path(root.join("sova.toml"));
    app.install(
        Tasks::new(store)
            .queues(["critical", "default", "mailer"])
            .scheduler_tick(Duration::from_secs(1))
            .job(
                Job::new("ping", |_task| async move {
                    tracing::info!("ping handled");
                    Ok(())
                })
                // Overridden by `[schedule.ping]` in sova.toml when present.
                .every(Duration::from_secs(10)),
            )
            .job(
                Job::new("mail_digest", |_task| async move {
                    tracing::info!("low-prio mail_digest");
                    Ok(())
                })
                .queue("mailer")
                .priority(priority::LOW)
                .cron("*/30 * * * * *"),
            )
            .job(Job::new("greet", |_task| async move {
                // Interactive only under `tasks run`; Dispatch/worker stay quiet.
                info("greet job");
                let name = ask("Your name").unwrap_or_else(|_| "world".into());
                table(
                    &["field", "value"],
                    &[
                        vec!["name".into(), name.clone()],
                        vec!["interactive".into(), is_interactive().to_string()],
                    ],
                );
                if confirm(format!("Greet {name}?"), true) {
                    info(format!("hello, {name}"));
                }
                Ok(())
            }))
            .exposed()
            .guard(bearer_guard("secret")),
    );
    app.get("/", || async {
        "POST /_tasks/enqueue — queues critical > default > mailer; CLI: tasks list|run|schedule"
    });
    app.listen(3010).await
}
