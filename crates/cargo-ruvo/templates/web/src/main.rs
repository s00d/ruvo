use ruvo::prelude::*;
use ruvo::{Html, Meta, Parser, ServerArgs};

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("{{name}}")
        .public_url("http://127.0.0.1:3000");
    app.get("/", || async { Html("<h1>{{name}}</h1>".to_string()) })
        .with(
            Meta::page()
                .title("Home")
                .description("Welcome to {{name}}"),
        );
    modules::register(&mut app);
    app.run().await
}
