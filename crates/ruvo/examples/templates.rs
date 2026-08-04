//! MiniJinja templates loaded from files.
use ruvo::{init_tracing, App, MiniJinjaEngine, Request, Response, Result};
use serde::Serialize;

#[derive(Serialize)]
struct Page {
    title: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let mut eng = MiniJinjaEngine::new();
    eng.add_template("home", include_str!("templates/views/home.html"))?;

    let mut app = App::new();
    app.state(eng);

    app.get("/", home);
    app.listen(3006).await
}

async fn home(req: Request) -> Response {
    let eng = req.state::<MiniJinjaEngine>();
    eng.render_html(
        "home",
        Page {
            title: "Ruvo templates".into(),
        },
    )
    .unwrap_or_else(|e| e.into_response())
}
