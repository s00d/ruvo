//! Static files with ETag / Range.
use sova::{App, Response, Result, Static};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.install(Static::new(
        "/assets",
        concat!(env!("CARGO_MANIFEST_DIR"), "/public"),
    ));
    app.get("/", |_| async {
        Response::html(include_str!("views/index.html"))
    });
    app.listen(3005).await
}
