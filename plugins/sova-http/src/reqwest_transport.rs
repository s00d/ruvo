//! Reqwest-backed transport with SSRF checks and response size limit.

use crate::error::HttpError;
use crate::ssrf::SsrfPolicy;
use crate::transport::{OutRequest, OutResponse, Transport};
use bytes::Bytes;
use futures_util::StreamExt;
use http::HeaderMap;
use reqwest::redirect::Policy;
use sova_core::extend::BoxFuture;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_MAX_REDIRECTS: usize = 10;

#[derive(Clone)]
pub struct ReqwestTransport {
    client: reqwest::Client,
    ssrf: Arc<SsrfPolicy>,
    max_response_size: usize,
}

impl ReqwestTransport {
    pub fn new(ssrf: SsrfPolicy, max_response_size: usize) -> Result<Self, HttpError> {
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .build()
            .map_err(|e| HttpError::Other(e.to_string()))?;
        Ok(Self {
            client,
            ssrf: Arc::new(ssrf),
            max_response_size,
        })
    }

    #[allow(dead_code)]
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

impl Transport for ReqwestTransport {
    fn send(&self, req: OutRequest) -> BoxFuture<Result<OutResponse, HttpError>> {
        let this = self.clone();
        Box::pin(async move { this.send_follow(req).await })
    }
}

impl ReqwestTransport {
    async fn send_follow(&self, mut req: OutRequest) -> Result<OutResponse, HttpError> {
        for _ in 0..=DEFAULT_MAX_REDIRECTS {
            self.ssrf.check_url(&req.url)?;
            let mut builder = self
                .client
                .request(req.method.clone(), &req.url)
                .headers(req.headers.clone());
            if let Some(t) = req.timeout {
                builder = builder.timeout(t);
            }
            if let Some(body) = req.body.clone() {
                builder = builder.body(body);
            }
            let res = builder.send().await.map_err(HttpError::from)?;
            let status = res.status();
            if status.is_redirection() {
                let loc = res
                    .headers()
                    .get(http::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| HttpError::Other("redirect without Location".into()))?;
                let next = resolve_redirect(&req.url, loc)?;
                // Re-check on every hop (SSRF).
                self.ssrf.check_url(&next)?;
                req.url = next;
                if status == reqwest::StatusCode::SEE_OTHER {
                    req.method = http::Method::GET;
                    req.body = None;
                }
                continue;
            }
            let headers = header_map_from_reqwest(res.headers());
            let body = read_limited(res, self.max_response_size).await?;
            return Ok(OutResponse::new(
                http::StatusCode::from_u16(status.as_u16()).unwrap_or(http::StatusCode::OK),
                headers,
                body,
            ));
        }
        Err(HttpError::Other("too many redirects".into()))
    }
}

async fn read_limited(res: reqwest::Response, max: usize) -> Result<Bytes, HttpError> {
    let mut out = Vec::new();
    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(HttpError::from)?;
        if out.len().saturating_add(chunk.len()) > max {
            return Err(HttpError::ResponseTooLarge);
        }
        out.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(out))
}

fn header_map_from_reqwest(h: &reqwest::header::HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (k, v) in h.iter() {
        if let (Ok(name), Ok(val)) = (
            http::HeaderName::from_bytes(k.as_str().as_bytes()),
            http::HeaderValue::from_bytes(v.as_bytes()),
        ) {
            out.append(name, val);
        }
    }
    out
}

fn resolve_redirect(base: &str, loc: &str) -> Result<String, HttpError> {
    if loc.starts_with("http://") || loc.starts_with("https://") {
        return Ok(loc.to_string());
    }
    let base = url::Url::parse(base).map_err(|e| HttpError::Other(e.to_string()))?;
    Ok(base
        .join(loc)
        .map_err(|e| HttpError::Other(e.to_string()))?
        .to_string())
}

pub fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

pub const DEFAULT_MAX_RESPONSE: usize = 10 * 1024 * 1024;
