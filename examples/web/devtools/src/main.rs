//! DevTools demo — HTML pages get the bottom bar; SSE feeds the Timeline tab.
//!
//! ```bash
//! cargo run -p devtools_demo
//! # open http://127.0.0.1:3030/
//! ```

use sova::prelude::*;
use sova::{
    DevTools, Html, HttpExt, Mail, MailExt, Meta, OutboundHttp, Parser, ServerArgs, SessionExt,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::web()
        .site("DevTools demo")
        .public_url("http://127.0.0.1:3030")
        .into_app();

    std::env::set_var("SOVA_MAIL", "fake");
    std::env::set_var("SOVA_MAIL_FROM", "DevTools <devtools@localhost>");
    app.install(Mail::from_env());
    app.install(OutboundHttp::new());
    app.install(DevTools::new().enabled(true)); // demo forces on in debug

    app.get("/", home).with(Meta::page().title("Home"));
    app.get("/ping", ping);
    app.get("/mail", send_mail);
    app.get("/proxy", proxy);

    tracing::info!("listening on http://127.0.0.1:3030 — open / for DevTools bar");
    app.listen(3030).await
}

async fn home(req: Request) -> Result<Html<String>> {
    req.session().set("demo", "1");
    Ok(Html(
        r#"<!doctype html>
<html><head><title>DevTools demo</title></head>
<body style="font-family:system-ui;max-width:40rem;margin:2rem auto;padding:0 1rem">
  <h1>Sova DevTools</h1>
  <p>Look at the bottom bar. Click it to open the panel.</p>
  <ul>
    <li><a href="/ping">/ping</a> — another HTML hit (Timeline via SSE)</li>
    <li><a href="/mail">/mail</a> — send fake mail (Mail tab)</li>
    <li><a href="/proxy">/proxy</a> — outbound HTTP (HTTP tab)</li>
  </ul>
</body></html>"#
            .into(),
    ))
}

async fn ping() -> Html<&'static str> {
    Html("<!doctype html><html><body><p>pong — check Timeline</p></body></html>")
}

async fn send_mail(req: Request) -> Result<Html<&'static str>> {
    req.mail()
        .to("user@example.com")
        .subject("DevTools hello")
        .text("Hi from demo")
        .send()
        .await?;
    Ok(Html(
        "<!doctype html><html><body><p>mail sent (fake) — open Mail tab</p></body></html>",
    ))
}

async fn proxy(req: Request) -> Result<Html<String>> {
    let _ = req.http().get("https://example.com/").send().await;
    Ok(Html(
        "<!doctype html><html><body><p>outbound done — open HTTP tab</p></body></html>".into(),
    ))
}
