//! Modular blog via `routes() -> Router` + mount.
use ruvo::{Bind, init_tracing, App, Request, Response, Result, Router};

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
    init_tracing();
    let mut app = App::new();
    app.mount("/blog", blog_routes());
    app.get("/", |_| async {
        Response::html(include_str!("blog/views/root.html"))
    });
    app.bind(Bind::Port(3002)).serve().await
}

async fn index(_: Request) -> Response {
    Response::html(include_str!("blog/views/index.html"))
}

async fn new_post(_: Request) -> Response {
    Response::text(include_str!("blog/views/new.txt").trim())
}

async fn show(req: Request) -> Response {
    let slug = req.param("slug").unwrap_or("?");
    Response::html(render(
        include_str!("blog/views/show.html"),
        &[("slug", slug)],
    ))
}
