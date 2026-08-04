//! Safe local-file responses (core primitive — plugins call this, not the reverse).

use super::{BoxError, Response};
use futures_util::TryStreamExt;
use http::{header, HeaderValue};
use http_body::Frame;
use http_body_util::{BodyExt, StreamBody};
use std::path::{Component, Path, PathBuf};
use tokio_util::io::ReaderStream;

pub(super) async fn serve_path(path: &Path) -> Response {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path.file_name().map(Path::new).unwrap_or(path);
    serve_in(parent, name).await
}

pub(super) async fn serve_in(dir: &Path, relative: &Path) -> Response {
    if !is_safe_relative(relative) {
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

    let file = match tokio::fs::File::open(&full).await {
        Ok(f) => f,
        Err(_) => return Response::text("File Not Found").status(404),
    };

    let mime = mime_guess::from_path(&full)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    let stream = ReaderStream::new(file);
    let mapped = stream
        .map_ok(Frame::data)
        .map_err(|e| -> BoxError { Box::new(e) });
    let mut res = Response::stream(BodyExt::boxed(StreamBody::new(mapped)));
    if let Ok(v) = HeaderValue::from_str(&mime) {
        res.headers.insert(header::CONTENT_TYPE, v);
    }
    if let Ok(v) = HeaderValue::from_str(&meta.len().to_string()) {
        res.headers.insert(header::CONTENT_LENGTH, v);
    }
    res
}

fn is_safe_relative(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
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
