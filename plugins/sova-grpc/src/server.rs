//! Optional HTTP mount + BackgroundService bind for unary methods.

use crate::error::GrpcError;
use crate::error_envelope::{connect_error_json, grpc_error_to_connect, status_for_rpc_code};
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
                        return connect_error_response(400, "invalid_argument", format!("{e}"));
                    }
                };
                let started = std::time::Instant::now();
                let bytes_in = body.len() as u64;
                let result = r.invoke_with_request(&method_name, req, body).await;
                crate::trace::emit_server(
                    &method_name,
                    started.elapsed().as_secs_f64() * 1000.0,
                    &result,
                    bytes_in,
                );
                match result {
                    Ok(out) => Response::bytes(out, "application/json"),
                    Err(e) => connect_error_from_grpc(&e),
                }
            }
        });
    }
}

fn connect_error_from_grpc(err: &GrpcError) -> Response {
    let (status, code, message) = grpc_error_to_connect(err);
    connect_error_response(status, &code, message)
}

fn connect_error_response(status: u16, code: &str, message: impl Into<String>) -> Response {
    Response::json(&connect_error_json(code, message)).status(status)
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

    fn run(self: Box<Self>, _state: Arc<StateMap>, shutdown: Shutdown) -> BoxFuture<()> {
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
        return hyper_connect_error(405, "method_not_allowed", "method not allowed");
    }
    let path = req.uri().path().trim_start_matches('/').to_string();
    let collected = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return hyper_connect_error(400, "invalid_argument", format!("body: {e}"));
        }
    };
    let started = std::time::Instant::now();
    let bytes_in = collected.len() as u64;
    let result = router.invoke_raw(&path, collected).await;
    crate::trace::emit_server(
        &path,
        started.elapsed().as_secs_f64() * 1000.0,
        &result,
        bytes_in,
    );
    match result {
        Ok(out) => HyperResponse::builder()
            .status(200)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Full::new(out))
            .unwrap(),
        Err(e) => {
            let (status, code, message) = grpc_error_to_connect(&e);
            hyper_connect_error(status, &code, message)
        }
    }
}

fn hyper_connect_error(
    status: u16,
    code: &str,
    message: impl Into<String>,
) -> HyperResponse<Full<Bytes>> {
    let effective = if status == 400 && code != "invalid_argument" {
        status_for_rpc_code(code)
    } else {
        status
    };
    let body = connect_error_json(code, message);
    HyperResponse::builder()
        .status(effective)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(serde_json::to_vec(&body).unwrap())))
        .unwrap()
}
