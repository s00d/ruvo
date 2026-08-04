//! CLI args demo: `cargo run -p ruvo --example cli --features cli -- --port 3010`
use ruvo::{App, ListenArgs, Parser, Request, Response, Result, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("cli ok") });
    app.listen_args(&args).await
}
