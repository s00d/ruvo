//! Response compression for Ruvo (Express [`compression`](https://expressjs.com/en/resources/middleware/compression.html)-style).
//!
//! Supports `br`, `gzip`, and `deflate`. Bodies are buffered then compressed
//! (not streamed chunk-by-chunk).
//!
//! ```ignore
//! app.install(Compress::new().threshold(1024).level(6));
//! ```

use async_compression::tokio::bufread::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use async_compression::Level;
use bytes::Bytes;
use http::header::{CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, ETAG, VARY};
use http::Method;
use ruvo_core::extend::{named, with_leaked};
use ruvo_core::{App, Next, Plugin, Request, Response};
use std::io::Cursor;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, BufReader};

/// `true` → consider compression (Express `filter`).
pub type Filter = Arc<dyn Fn(&Request, &Response) -> bool + Send + Sync>;

/// Gzip / Deflate / Brotli response compression.
pub struct Compress {
    threshold: usize,
    /// zlib level for gzip/deflate (`0..=9`, default `6`).
    level: i32,
    /// Brotli quality (`0..=11`, Express default `4`).
    brotli_quality: i32,
    filter: Filter,
}

impl Compress {
    pub fn new() -> Self {
        Self {
            threshold: 1024,
            level: 6,
            brotli_quality: 4,
            filter: Arc::new(Self::default_filter),
        }
    }

    /// Minimum body size in bytes before compressing (Express default: 1 KiB).
    pub fn threshold(mut self, bytes: usize) -> Self {
        self.threshold = bytes;
        self
    }

    /// Gzip/deflate compression level `0..=9` (default `6`).
    pub fn level(mut self, level: i32) -> Self {
        self.level = level.clamp(0, 9);
        self
    }

    /// Brotli quality `0..=11` (default `4`, as in Express).
    pub fn brotli_quality(mut self, q: i32) -> Self {
        self.brotli_quality = q.clamp(0, 11);
        self
    }

    /// Replace the filter (`true` → consider compression).
    pub fn filter<F>(mut self, f: F) -> Self
    where
        F: Fn(&Request, &Response) -> bool + Send + Sync + 'static,
    {
        self.filter = Arc::new(f);
        self
    }

    /// Default filter: no `x-no-compression`, compressible `Content-Type`
    /// (same role as Express `compression.filter` + `compressible`).
    pub fn default_filter(req: &Request, res: &Response) -> bool {
        if req.header("x-no-compression").is_some() {
            return false;
        }
        let Some(ct) = res
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
        else {
            return false;
        };
        is_compressible(ct)
    }
}

impl Default for Compress {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Compress {
    fn id(&self) -> &'static str {
        "compress"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("Compress")
            .description("gzip / deflate / brotli response compression")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.use_middleware(named(
            "compress",
            with_leaked(self, |cfg, req, next| async move { run(cfg, req, next).await }),
        ));
    }
}

async fn run(cfg: &'static Compress, req: Request, next: Next) -> Response {
    let accept = req
        .header("accept-encoding")
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_head = req.method == Method::HEAD;
    let filter_req = snapshot_request(&req);
    let mut res = next(req).await;

    // Express varies on Accept-Encoding when the client sent it.
    if !accept.is_empty() {
        append_vary(&mut res, "Accept-Encoding");
    }

    if is_head || res.status_code().as_u16() == 206 {
        return res;
    }
    if !should_transform(&res) {
        return res;
    }
    if res.headers().get(CONTENT_ENCODING).is_some() {
        return res;
    }
    if !(cfg.filter)(&filter_req, &res) {
        return res;
    }

    let Some(encoding) = negotiate(&accept) else {
        return res;
    };

    // ponytail: buffer whole body; stream wrap if SSE/large downloads need it.
    let body = res.take_body();
    let data = match body.collect().await {
        Ok(b) => b,
        Err(_) => return res,
    };

    if data.len() < cfg.threshold {
        res.set_body(data);
        return res;
    }

    match encode(encoding, &data, cfg.level, cfg.brotli_quality).await {
        Ok(out) => {
            res.headers_mut().remove(CONTENT_LENGTH);
            if let Ok(v) = encoding.parse() {
                res.headers_mut().insert(CONTENT_ENCODING, v);
            }
            weaken_etag(&mut res);
            res.set_body(out);
            res
        }
        Err(_) => {
            res.set_body(data);
            res
        }
    }
}

fn snapshot_request(req: &Request) -> Request {
    let mut b = Request::builder()
        .method(req.method.clone())
        .path(req.path.clone());
    for (name, value) in req.headers.iter() {
        if let Ok(v) = value.to_str() {
            b = b.header(name.as_str(), v);
        }
    }
    b.build()
}

fn should_transform(res: &Response) -> bool {
    let Some(cc) = res
        .headers()
        .get(CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
    else {
        return true;
    };
    !cc.to_ascii_lowercase()
        .split(',')
        .any(|d| d.trim() == "no-transform")
}

fn append_vary(res: &mut Response, token: &str) {
    let existing = res
        .headers()
        .get(VARY)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if existing
        .split(',')
        .any(|t| t.trim().eq_ignore_ascii_case(token))
    {
        return;
    }
    let value = if existing.is_empty() {
        token.to_string()
    } else {
        format!("{existing}, {token}")
    };
    if let Ok(v) = value.parse() {
        res.headers_mut().insert(VARY, v);
    }
}

/// Rough port of the `compressible` npm heuristics used by Express.
fn is_compressible(content_type: &str) -> bool {
    let ct = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    if ct.is_empty() {
        return false;
    }
    if ct.starts_with("text/") || ct == "image/svg+xml" {
        return true;
    }
    if ct.starts_with("image/")
        || ct.starts_with("audio/")
        || ct.starts_with("video/")
        || ct.contains("zip")
        || ct.contains("gzip")
        || ct.contains("octet-stream")
        || ct.contains("wasm")
    {
        return false;
    }
    matches!(
        ct.as_str(),
        "application/json"
            | "application/javascript"
            | "application/ecmascript"
            | "application/xml"
            | "application/x-www-form-urlencoded"
            | "application/graphql"
            | "application/ld+json"
            | "application/manifest+json"
            | "application/vnd.api+json"
            | "application/xhtml+xml"
            | "application/rss+xml"
            | "application/atom+xml"
            | "font/ttf"
            | "font/otf"
    ) || ct.ends_with("+json")
        || ct.ends_with("+xml")
        || ct.ends_with("+text")
}

fn weaken_etag(res: &mut Response) {
    let Some(tag) = res.headers().get(ETAG).and_then(|v| v.to_str().ok()) else {
        return;
    };
    if tag.starts_with("W/") {
        return;
    }
    let weak = format!("W/{tag}");
    if let Ok(v) = weak.parse() {
        res.headers_mut().insert(ETAG, v);
    }
}

fn negotiate(accept: &str) -> Option<&'static str> {
    // Preferred order on equal q (Express: br, gzip).
    const CANDIDATES: &[&str] = &["br", "gzip", "deflate"];
    let mut best: Option<(&'static str, f32, usize)> = None;
    for part in accept.split(',') {
        let mut it = part.trim().split(';');
        let coding = it.next().unwrap_or("").trim();
        let idx = match coding {
            "br" => 0,
            "gzip" => 1,
            "deflate" => 2,
            _ => continue,
        };
        let mut q = 1.0_f32;
        for param in it {
            let p = param.trim();
            if let Some(v) = p.strip_prefix("q=") {
                q = v.parse().unwrap_or(0.0);
            }
        }
        if q <= 0.0 {
            continue;
        }
        let coding = CANDIDATES[idx];
        match best {
            None => best = Some((coding, q, idx)),
            Some((_, prev_q, prev_idx)) => {
                if q > prev_q || (q == prev_q && idx < prev_idx) {
                    best = Some((coding, q, idx));
                }
            }
        }
    }
    best.map(|(c, _, _)| c)
}

async fn encode(encoding: &str, data: &Bytes, level: i32, brotli_q: i32) -> std::io::Result<Bytes> {
    let reader = BufReader::new(Cursor::new(data.clone()));
    let mut out = Vec::new();
    match encoding {
        "br" => {
            let mut enc = BrotliEncoder::with_quality(reader, Level::Precise(brotli_q));
            enc.read_to_end(&mut out).await?;
        }
        "deflate" => {
            let mut enc = DeflateEncoder::with_quality(reader, Level::Precise(level));
            enc.read_to_end(&mut out).await?;
        }
        _ => {
            let mut enc = GzipEncoder::with_quality(reader, Level::Precise(level));
            enc.read_to_end(&mut out).await?;
        }
    }
    Ok(Bytes::from(out))
}
