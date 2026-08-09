use crate::app::AppInner;
use crate::error::Result;
use crate::request::{parse_query, Request};
use crate::response::{Response, ResponseBody};
use crate::state::Extensions;
use crate::upgrade::PendingUpgrade;
use bytes::Bytes;
use http::HeaderValue;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::upgrade::OnUpgrade as HyperOnUpgrade;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use std::convert::Infallible;
use std::net::SocketAddr;

use crate::server::forwarded::forwarded_addr;
use crate::server::ClientAddr;

pub(super) async fn to_sova_request(
    req: HyperRequest<Incoming>,
    app: &AppInner,
    peer: SocketAddr,
) -> Result<Request> {
    use crate::request::{resolve_scheme_host, ReqBody};

    let max_body = app.max_body_size;
    let state = app.state();
    let trust_proxy = app.trust_proxy;

    let (mut parts, incoming) = req.into_parts();
    let on_upgrade = parts.extensions.remove::<HyperOnUpgrade>();

    let method = parts.method;
    let uri = parts.uri;
    let path = uri.path().to_string();
    let raw_query = uri.query().unwrap_or("").to_string();
    let query = if raw_query.is_empty() {
        Default::default()
    } else {
        parse_query(&raw_query)
    };
    let headers = parts.headers;

    // Content-Length is checked after route match via `body_limit` / [`MaxBody`].

    let (scheme, host) = resolve_scheme_host(&headers, uri.scheme_str(), trust_proxy);

    let stream = incoming
        .map_err(|e| -> crate::response::BoxError { Box::new(e) })
        .boxed();

    let client = if trust_proxy {
        forwarded_addr(&headers).unwrap_or(peer)
    } else {
        peer
    };

    let mut extensions = Extensions::new();
    extensions.insert(ClientAddr(client));
    if let Some(on_upgrade) = on_upgrade {
        extensions.insert(PendingUpgrade {
            on_upgrade,
            budget: app.max_upgraded.clone(),
        });
    }

    Ok(Request {
        method,
        path,
        headers,
        params: Default::default(),
        query,
        scheme,
        host,
        raw_query,
        body: ReqBody::Stream(stream),
        body_limit: max_body,
        state,
        extensions,
    })
}

pub(super) fn to_hyper_response(
    res: Response,
    hsts: bool,
    alt_svc: Option<&str>,
) -> HyperResponse<ResponseBody> {
    let (status, mut headers, body) = res.into_parts();
    if hsts {
        headers.insert(
            http::header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    if let Some(alt_svc) = alt_svc {
        if let Ok(v) = HeaderValue::from_str(alt_svc) {
            headers.insert(http::header::HeaderName::from_static("alt-svc"), v);
        }
    }
    let mut builder = HyperResponse::builder().status(status);
    *builder.headers_mut().expect("builder") = headers;
    builder.body(body).unwrap_or_else(|_| {
        HyperResponse::builder()
            .status(500)
            .body(
                Full::new(Bytes::from_static(b"internal error"))
                    .map_err(|_: Infallible| unreachable!())
                    .boxed(),
            )
            .expect("fallback")
    })
}
