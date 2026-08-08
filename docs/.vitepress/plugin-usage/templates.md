If `views/` exists, **`App::web()`** already installs MiniJinja. Handlers only render:

```rust
use ruvo::prelude::*;
use ruvo::{Meta, Parser, RenderExt, Request, Response, ServerArgs};
use serde::Serialize;

#[derive(Serialize)]
struct Page {
    title: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("Blog")
        .public_url("http://127.0.0.1:3000")
        .views("views");

    app.get("/", home).with(Meta::page().title("Home"));
    app.run().await
}

async fn home(req: Request) -> Response {
    req.render(
        "home.html",
        Page {
            title: "Ruvo".into(),
        },
    )
    .unwrap_or_else(|e| e.into_response())
}
```

i18n helper in templates: `.per_request("t", ruvo::template_fn)` after `into_app()` when you add the i18n plugin. Demos: `templates`, `templates_i18n`.
