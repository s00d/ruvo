//! Hacker News–style demo for Sova.
//!
//! ```bash
//! cargo run -p hackernews
//! ```
//!
//! DB URL + auto migrate/seed: `examples/web/hackernews/sova.toml`. Override with `DATABASE_URL`.
//! Manual: `cargo run -p hackernews -- migrate` / `-- seed`.

use hackernews::build_app;
use sova::{Parser, Result, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();
    let app = build_app()?;
    app.run().await
}
