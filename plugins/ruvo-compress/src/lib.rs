//! Gzip/Brotli response compression for Ruvo.

use async_compression::tokio::bufread::{BrotliEncoder, GzipEncoder};
use bytes::Bytes;
use http::header::{CONTENT_ENCODING, CONTENT_LENGTH};
use ruvo_core::extend::{named, IntoMiddleware, Middleware};
use ruvo_core::{App, Next, Plugin, Request};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, BufReader};

/// Gzip/Brotli response compression plugin.
pub struct Compress;

impl Plugin for Compress {
    fn install(self, app: &mut App) {
        app.use_middleware(named("compress", compress_middleware()));
    }
}

fn compress_middleware() -> Middleware {
    (move |req: Request, next: Next| async move {
        let accept = req
            .header("accept-encoding")
            .unwrap_or("")
            .to_ascii_lowercase();
        let mut res = next(req).await;

        if res.status_code().as_u16() == 206 {
            return res;
        }
        if res.headers().get(CONTENT_ENCODING).is_some() {
            return res;
        }
        if let Some(ct) = res
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
        {
            let ct = ct.to_ascii_lowercase();
            if ct.starts_with("image/")
                || ct.contains("zip")
                || ct.contains("gzip")
                || ct.contains("octet-stream")
            {
                return res;
            }
        }

        let encoding = if accept.contains("br") {
            Some("br")
        } else if accept.contains("gzip") {
            Some("gzip")
        } else {
            None
        };

        let Some(encoding) = encoding else {
            return res;
        };

        let body = res.take_body();
        let data = match body.collect().await {
            Ok(b) => b,
            Err(_) => return res,
        };

        if data.len() < 128 {
            res.set_body(data);
            return res;
        }

        let compressed = match encoding {
            "br" => compress_brotli(&data).await,
            _ => compress_gzip(&data).await,
        };

        match compressed {
            Ok(out) => {
                res.headers_mut().remove(CONTENT_LENGTH);
                if let Ok(v) = encoding.parse() {
                    res.headers_mut().insert(CONTENT_ENCODING, v);
                }
                res.set_body(out);
                res
            }
            Err(_) => {
                res.set_body(data);
                res
            }
        }
    })
    .into_middleware()
}

async fn compress_gzip(data: &Bytes) -> std::io::Result<Bytes> {
    let reader = BufReader::new(Cursor::new(data.clone()));
    let mut encoder = GzipEncoder::new(reader);
    let mut out = Vec::new();
    encoder.read_to_end(&mut out).await?;
    Ok(Bytes::from(out))
}

async fn compress_brotli(data: &Bytes) -> std::io::Result<Bytes> {
    let reader = BufReader::new(Cursor::new(data.clone()));
    let mut encoder = BrotliEncoder::new(reader);
    let mut out = Vec::new();
    encoder.read_to_end(&mut out).await?;
    Ok(Bytes::from(out))
}
