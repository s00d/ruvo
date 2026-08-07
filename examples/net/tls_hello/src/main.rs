use ruvo::prelude::*;
use ruvo::Tls;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.get("/", || async { "hello tls" });

    app.bind(3443)
        .tls(Tls::self_signed(&["localhost", "127.0.0.1"])?)?
        .serve()
        .await
}
