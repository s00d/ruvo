use ruvo::prelude::*;
use ruvo::{Parser, ServerArgs};

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::api().title("{{name}} API").version("0.1.0");
    modules::register(&mut app);
    app.run().await
}
