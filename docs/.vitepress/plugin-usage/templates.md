If `views/` exists, **`App::web()`** already installs MiniJinja. Handlers only render:

```rust
use sova::prelude::*;
use sova::{Meta, Parser, RenderExt, Request, Response, ServerArgs};
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
            title: "Sova".into(),
        },
    )
    .unwrap_or_else(|e| e.into_response())
}
```

Extra per-request helpers after the preset: `sova_templates::register_per_request(&mut app, "t", sova::template_fn)`. Do not reinstall Templates on top of `App::web()` (duplicate id). Demos: `templates`, `templates_i18n`.
