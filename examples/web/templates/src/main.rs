//! MiniJinja templates loaded from files.
use sova::{App, Request, Response, Result, RenderExt, Templates};
use serde::Serialize;

#[derive(Serialize)]
struct Page {
    title: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    let views_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/views");
    app.install(Templates::minijinja(views_dir));

    app.get("/", home);
    app.listen(3006).await
}

async fn home(req: Request) -> Response {
    req.render(
        "home.html",
        Page {
            title: "Sova templates".into(),
        },
    )
    .unwrap_or_else(|e| e.into_response())
}
