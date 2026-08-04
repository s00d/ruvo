use ruvo::{Bind, init_tracing, logger, App, Cors, Request, Response, Result, Router};

mod modules {
    pub mod auth;
    pub mod blog;
}

use modules::{auth, blog};

#[derive(Clone)]
struct AppConfig {
    app_name: String,
}

fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
    }
    out
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let mut app = App::new();

    app.state(AppConfig {
        app_name: "ruvo-hello".into(),
    });

    app.use_middleware(logger());
    app.install(Cors::new().origin("*"));

    app.get("/", home);

    app.mount("/auth", auth::routes());
    app.mount("/blog", blog::routes());

    app.install(|app: &mut App| {
        app.get("/health", health);
    });

    app.install(ruvo::Static::new("/assets", concat!(env!("CARGO_MANIFEST_DIR"), "/examples/hello/public")));

    app.not_found(|_req: Request| async {
        Response::html(include_str!("views/not_found.html")).status(404)
    });

    app.bind(Bind::Port(3000)).serve().await
}

async fn home(req: Request) -> Response {
    let name = req
        .try_state::<AppConfig>()
        .map(|c| c.app_name.clone())
        .unwrap_or_else(|| "ruvo".into());
    Response::html(render(
        include_str!("views/home.html"),
        &[("name", &name)],
    ))
}

async fn health(_req: Request) -> Response {
    Response::json(&serde_json::json!({ "ok": true }))
}

#[allow(dead_code)]
fn _router_type(_: Router) {}
