use ruvo::prelude::*;
use ruvo::{Cors, Json, Static};

mod modules;

#[derive(Clone)]
struct AppConfig {
    app_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "ruvo".into(),
        }
    }
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
    let mut app = App::new();
    app.state(AppConfig {
        app_name: "ruvo-hello".into(),
    });
    app.use_middleware(logger());
    app.install(Cors::new().origin("*"));

    app.get("/", home);
    app.get("/health", || async { Json(serde_json::json!({ "ok": true })) });
    modules::register(&mut app);

    app.install(Static::new(
        "/assets",
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/hello/public"),
    ));
    app.not_found(|| async {
        Html(include_str!("views/not_found.html"))
            .into_response()
            .status(404)
    });

    app.listen(3000).await
}

async fn home(req: Request) -> Html<String> {
    let name = req.state_or_default::<AppConfig>().app_name.clone();
    Html(render(include_str!("views/home.html"), &[("name", &name)]))
}
