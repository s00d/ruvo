use ruvo::{App, Bind, Request, Response, Result, Tls};

#[tokio::main]
async fn main() -> Result<()> {
    ruvo::init_tracing();
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("hello tls") });

    app.bind(Bind::Port(3443))
        .tls(Tls::self_signed(&["localhost", "127.0.0.1"])?)?
        .serve()
        .await
}
