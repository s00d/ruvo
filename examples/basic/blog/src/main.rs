//! Modular blog via `routes() -> Router` + mount.
use sova::prelude::*;

fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
    }
    out
}

fn blog_routes() -> Router {
    let mut r = Router::new();
    r.get("/", index);
    r.get("/new", new_post);
    r.get("/:slug", show);
    r
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.mount("/blog", blog_routes());
    app.get("/", || async { Html(include_str!("views/root.html")) });
    app.listen(3002).await
}

async fn index(_: Request) -> Html<&'static str> {
    Html(include_str!("views/index.html"))
}

async fn new_post(_: Request) -> Text<&'static str> {
    Text(include_str!("views/new.txt").trim())
}

async fn show(req: Request) -> Html<String> {
    let slug = req.param("slug").unwrap_or("?");
    Html(render(include_str!("views/show.html"), &[("slug", slug)]))
}
