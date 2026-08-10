//! Conditional static file serving (ETag / 304 / Range / Cache-Control).

use http::{header, HeaderValue};
use sova_core::{Request, Response};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub(crate) struct StaticOpts {
    pub max_age: Duration,
    pub immutable: bool,
    pub allow_dotfiles: bool,
}

pub(crate) struct FileOptions<'a> {
    pub range: Option<&'a str>,
    pub if_none_match: Option<&'a str>,
    pub if_modified_since: Option<&'a str>,
}

impl<'a> FileOptions<'a> {
    pub(crate) fn from_request(req: &'a Request) -> Self {
        Self {
            range: req.header("range"),
            if_none_match: req.header("if-none-match"),
            if_modified_since: req.header("if-modified-since"),
        }
    }
}

pub(crate) async fn serve_in(
    dir: &Path,
    relative: &Path,
    opts: FileOptions<'_>,
    static_opts: &StaticOpts,
) -> Response {
    if !is_safe_relative(relative) {
        return Response::text("Forbidden").status(403);
    }
    if !static_opts.allow_dotfiles && has_dotfile_segment(relative) {
        return Response::text("Forbidden").status(403);
    }

    let full = match resolve_safe(dir, relative).await {
        Ok(p) => p,
        Err(status) => {
            return Response::text(if status == 403 {
                "Forbidden"
            } else {
                "File Not Found"
            })
            .status(status);
        }
    };

    let meta = match tokio::fs::metadata(&full).await {
        Ok(m) if m.is_file() => m,
        Ok(_) => return Response::text("Forbidden").status(403),
        Err(_) => return Response::text("File Not Found").status(404),
    };

    let len = meta.len();
    let modified = meta.modified().ok();
    let etag = format_etag(len, modified);
    let mime = mime_guess::from_path(&full)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    if not_modified(&etag, modified, opts.if_none_match, opts.if_modified_since) {
        let mut res = Response::empty().status(304);
        set_file_headers(&mut res, &mime, &etag, modified, None, static_opts);
        return res;
    }

    if let Some(range) = opts.range.and_then(parse_byte_range) {
        if let Some((start, end)) = range.resolve(len) {
            return serve_range(&full, &mime, &etag, modified, start, end, len, static_opts).await;
        }
        return Response::text("Range Not Satisfiable")
            .status(416)
            .header("content-range", format!("bytes */{len}"));
    }

    let mut res = Response::file_in(dir, relative).await;
    if res.status_code().as_u16() != 200 {
        return res;
    }
    set_file_headers(&mut res, &mime, &etag, modified, Some(len), static_opts);
    res
}

fn has_dotfile_segment(path: &Path) -> bool {
    path.components().any(|c| match c {
        Component::Normal(s) => s.to_string_lossy().starts_with('.'),
        _ => false,
    })
}

fn is_safe_relative(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn not_modified(
    etag: &str,
    modified: Option<SystemTime>,
    if_none_match: Option<&str>,
    if_modified_since: Option<&str>,
) -> bool {
    if let Some(inm) = if_none_match {
        let inm = inm.trim();
        if inm == "*" || inm.split(',').any(|t| t.trim() == etag) {
            return true;
        }
    }
    if let (Some(ims), Some(m)) = (if_modified_since, modified) {
        if httpdate::fmt_http_date(m) == ims.trim() {
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
async fn serve_range(
    full: &Path,
    mime: &str,
    etag: &str,
    modified: Option<SystemTime>,
    start: u64,
    end: u64,
    total: u64,
    static_opts: &StaticOpts,
) -> Response {
    let mut file = match tokio::fs::File::open(full).await {
        Ok(f) => f,
        Err(_) => return Response::text("File Not Found").status(404),
    };
    if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
        return Response::text("Range Not Satisfiable").status(416);
    }
    let to_read = (end - start + 1) as usize;
    let mut buf = vec![0u8; to_read];
    if file.read_exact(&mut buf).await.is_err() {
        return Response::text("Range Not Satisfiable").status(416);
    }

    let mut res = Response::empty().status(206);
    res.set_body(bytes::Bytes::from(buf));
    set_file_headers(
        &mut res,
        mime,
        etag,
        modified,
        Some(end - start + 1),
        static_opts,
    );
    res.headers_mut().insert(
        header::CONTENT_RANGE,
        HeaderValue::from_str(&format!("bytes {start}-{end}/{total}")).expect("content-range"),
    );
    res
}

fn set_file_headers(
    res: &mut Response,
    mime: &str,
    etag: &str,
    modified: Option<SystemTime>,
    len: Option<u64>,
    static_opts: &StaticOpts,
) {
    if let Ok(v) = HeaderValue::from_str(mime) {
        res.headers_mut().insert(header::CONTENT_TYPE, v);
    }
    if let Ok(v) = HeaderValue::from_str(etag) {
        res.headers_mut().insert(header::ETAG, v);
    }
    if let Some(m) = modified {
        if let Ok(v) = HeaderValue::from_str(&httpdate::fmt_http_date(m)) {
            res.headers_mut().insert(header::LAST_MODIFIED, v);
        }
    }
    res.headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));

    let secs = static_opts.max_age.as_secs();
    let cc = if static_opts.immutable {
        format!("public, max-age={secs}, immutable")
    } else {
        format!("public, max-age={secs}")
    };
    if let Ok(v) = HeaderValue::from_str(&cc) {
        res.headers_mut().insert(header::CACHE_CONTROL, v);
    }

    if let Some(len) = len {
        if let Ok(v) = HeaderValue::from_str(&len.to_string()) {
            res.headers_mut().insert(header::CONTENT_LENGTH, v);
        }
    }
}

async fn resolve_safe(dir: &Path, relative: &Path) -> Result<PathBuf, u16> {
    let base = tokio::fs::canonicalize(dir).await.map_err(|_| 404u16)?;
    let candidate = dir.join(relative);
    let canon = tokio::fs::canonicalize(&candidate)
        .await
        .map_err(|_| 404u16)?;
    if !canon.starts_with(&base) {
        return Err(403);
    }
    Ok(canon)
}

struct ByteRange {
    start: Option<u64>,
    end: Option<u64>,
}

impl ByteRange {
    fn resolve(&self, len: u64) -> Option<(u64, u64)> {
        if len == 0 {
            return None;
        }
        let start = self.start.unwrap_or(0);
        let end = self.end.unwrap_or(len.saturating_sub(1)).min(len - 1);
        if start > end || start >= len {
            return None;
        }
        Some((start, end))
    }
}

fn parse_byte_range(header: &str) -> Option<ByteRange> {
    let header = header.strip_prefix("bytes=")?;
    let (start, end) = header.split_once('-')?;
    let start = if start.is_empty() {
        None
    } else {
        Some(start.parse().ok()?)
    };
    let end = if end.is_empty() {
        None
    } else {
        Some(end.parse().ok()?)
    };
    Some(ByteRange { start, end })
}

fn format_etag(len: u64, modified: Option<SystemTime>) -> String {
    modified
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| format!("\"{:x}-{:x}\"", d.as_secs(), len))
        .unwrap_or_else(|| format!("\"{len:x}\""))
}
