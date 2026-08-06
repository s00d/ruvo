//! CLI args demo: `cargo run -p ruvo --example cli --features cli -- --port 3010`
use ruvo::prelude::*;
use ruvo::{Parser, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::new();
    app.get("/", || async { "cli ok" });
    app.run().await
}
