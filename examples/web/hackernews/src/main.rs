//! Hacker News–style demo for Sova.
//!
//! ```bash
//! DATABASE_URL=sqlite:./hn.db?mode=rwc cargo run -p hackernews -- migrate
//! DATABASE_URL=sqlite:./hn.db?mode=rwc cargo run -p hackernews -- seed
//! DATABASE_URL=sqlite:./hn.db?mode=rwc cargo run -p hackernews
//! ```

use hackernews::build_app;
use sova::{Parser, Result, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();
    let app = build_app()?;
    app.run().await
}
