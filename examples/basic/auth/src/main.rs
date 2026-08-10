//! Auth demo: extensions + cookie sessions.
use sova::{memory_sessions, App, Html, Redirect, Request, Result, SessionExt};

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
    app.install(memory_sessions());

    app.get("/", home);
    app.post("/login", login);
    app.post("/logout", logout);
    app.listen(3003).await
}

async fn home(req: Request) -> Html<String> {
    let user = req.session().get_or("user", "guest");
    Html(render(include_str!("views/home.html"), &[("user", &user)]))
}

async fn login(req: Request) -> Redirect {
    req.session().set("user", "ada");
    Redirect::see_other("/")
}

async fn logout(req: Request) -> Redirect {
    req.session().set("user", "guest");
    Redirect::see_other("/")
}
