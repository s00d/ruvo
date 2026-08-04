//! Static files with ETag / Range.
use ruvo::{init_tracing, App, Response, Result, Static};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let mut app = App::new();
    app.install(Static::new(
        "/assets",
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/hello/public"),
    ));
    app.get("/", |_| async {
        Response::html(include_str!("static_ex/views/index.html"))
    });
    app.listen(3005).await
}
