//! CLI args demo: `cargo run -p cli -- --port 3010 --log-file logs/cli.log`
use sova::prelude::*;
use sova::{Parser, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::new();
    app.get("/", || async { "cli ok" });
    app.run().await
}
