//! Optional HTTP mount + BackgroundService bind for unary methods.

use crate::error::GrpcError;
use crate::router::MethodRouter;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use sova_core::extend::{wait_shutdown, BoxFuture, StateMap};
use sova_core::{App, BackgroundService, Request, Response, Shutdown};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

pub(crate) fn mount_on_app(app: &mut App, router: MethodRouter) {
    let methods = router.methods();
    for method in methods {
        let path = format!("/{}", method.trim_start_matches('/'));
        let r = router.clone();
        let method_name = method.clone();
        app.post(&path, move |mut req: Request| {
            let r = r.clone();
            let method_name = method_name.clone();
            async move {
                let body = match req.body().await {
                    Ok(b) => b,
                    Err(e) => {
                        return Response::json(&serde_json::json!({
                            "code": "invalid_argument",
                            "message": format!("{e}"),
                        }))
                        .status(400);
                    }
                };
                match r.invoke_raw(&method_name, body).await {
                    Ok(out) => Response::bytes(out, "application/json"),
                    Err(GrpcError::NotFound(_)) => Response::json(&serde_json::json!({
                        "code": "not_found",
                        "message": method_name,
                    }))
                    .status(404),
                    Err(e) => Response::json(&serde_json::json!({
                        "code": "internal",
                        "message": e.to_string(),
                    }))
                    .status(500),
                }
            }
        });
    }
}

pub(crate) struct GrpcBindService {
    addr: SocketAddr,
    router: MethodRouter,
}

impl GrpcBindService {
    pub fn new(addr: SocketAddr, router: MethodRouter) -> Self {
        Self { addr, router }
    }
}

impl BackgroundService for GrpcBindService {
    fn name(&self) -> &str {
        "grpc-bind"
    }

    fn run(
        self: Box<Self>,
        _state: Arc<StateMap>,
        shutdown: Shutdown,
    ) -> BoxFuture<()> {
        Box::pin(async move {
            let listener = match TcpListener::bind(self.addr).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!(error = %e, addr = %self.addr, "grpc bind failed");
                    return;
                }
            };
            tracing::info!(addr = %self.addr, "grpc Connect-JSON listening");
            let router = self.router;
            loop {
                tokio::select! {
                    _ = wait_shutdown(shutdown.clone()) => break,
                    acc = listener.accept() => {
                        let Ok((stream, _)) = acc else { continue };
                        let router = router.clone();
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            let svc = service_fn(move |req: HyperRequest<Incoming>| {
                                let router = router.clone();
                                async move { Ok::<_, Infallible>(handle(router, req).await) }
                            });
                            let _ = hyper::server::conn::http1::Builder::new()
                                .serve_connection(io, svc)
                                .await;
                        });
                    }
                }
            }
        })
    }
}

async fn handle(router: MethodRouter, req: HyperRequest<Incoming>) -> HyperResponse<Full<Bytes>> {
    if req.method() != hyper::Method::POST {
        return HyperResponse::builder()
            .status(405)
            .body(Full::new(Bytes::from_static(b"method not allowed")))
            .unwrap();
    }
    let path = req.uri().path().trim_start_matches('/').to_string();
    let collected = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return json_status(400, &format!("body: {e}"));
        }
    };
    match router.invoke_raw(&path, collected).await {
        Ok(out) => HyperResponse::builder()
            .status(200)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Full::new(out))
            .unwrap(),
        Err(GrpcError::NotFound(m)) => json_status(404, &format!("not found: {m}")),
        Err(e) => json_status(500, &e.to_string()),
    }
}

fn json_status(status: u16, message: &str) -> HyperResponse<Full<Bytes>> {
    let body = serde_json::json!({ "code": "error", "message": message });
    HyperResponse::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(serde_json::to_vec(&body).unwrap())))
        .unwrap()
}
