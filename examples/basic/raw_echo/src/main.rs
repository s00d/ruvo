//! Raw Hyper escape hatch (last resort — prefer normal routes + `on_upgrade` for WS).
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use sova::extend::{BoxError, ResponseBody};
use sova::{App, Response, Result};
use std::convert::Infallible;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();

    app.get("/", |_| async {
        Response::html(include_str!("views/index.html"))
    });

    app.raw("/raw", |req: HyperRequest<Incoming>| async move {
        let path = req.uri().path().to_string();
        HyperResponse::builder()
            .status(200)
            .header("content-type", "text/plain")
            .body(
                Full::new(Bytes::from(format!("raw echo path={path}")))
                    .map_err(|_: Infallible| -> BoxError { unreachable!() })
                    .boxed(),
            )
            .unwrap()
    });
    app.listen(3007).await
}

#[allow(dead_code)]
fn _ty(_: ResponseBody) {}
