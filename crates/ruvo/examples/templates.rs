//! MiniJinja templates loaded from files.
use ruvo::{Bind, init_tracing, App, Request, Response, Result, RenderExt, Templates};
use serde::Serialize;

#[derive(Serialize)]
struct Page {
    title: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let mut app = App::new();
    let views_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/templates/views");
    app.install(Templates::minijinja(views_dir));

    app.get("/", home);
    app.bind(Bind::Port(3006)).serve().await
}

async fn home(req: Request) -> Response {
    req.render(
        "home.html",
        Page {
            title: "Ruvo templates".into(),
        },
    )
    .unwrap_or_else(|e| e.into_response())
}
