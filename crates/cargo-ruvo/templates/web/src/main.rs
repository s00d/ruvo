use ruvo::prelude::*;
use ruvo::{Parser, ServerArgs};

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    ruvo_env::load().ok();
    let mut app = App::new();
    app.get("/", || async { Html("<h1>{{name}}</h1>".to_string()) });
    modules::register(&mut app);
    app.run().await
}
