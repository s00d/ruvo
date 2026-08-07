//! Unified form / multipart input (`Request::input`, [`FormData`], [`Upload`]).

use crate::error::{Error, Result};
use crate::request::Request;
use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// One uploaded file (or any multipart part with a filename).
#[derive(Debug, Clone)]
pub struct Upload {
    pub field: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Bytes,
}

impl Upload {
    /// Write bytes to `path` (creates parent dirs).
    pub async fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| Error::Internal(e.to_string()))?;
            }
        }
        tokio::fs::write(path, &self.data)
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// Save under `dir` / `filename` (rejects `..` and absolute names).
    pub async fn save_in(&self, dir: impl AsRef<Path>, filename: &str) -> Result<PathBuf> {
        let name = Path::new(filename);
        if !is_safe_relative(name) {
            return Err(Error::BadRequest("unsafe upload filename".into()));
        }
        let dest = dir.as_ref().join(name);
        self.save(&dest).await?;
        Ok(dest)
    }

    /// Preferred download name: client filename or field name.
    pub fn suggested_name(&self) -> &str {
        self.filename
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(self.field.as_str())
    }
}

/// Parsed form body: text fields + file uploads (urlencoded or multipart).
#[derive(Debug, Clone, Default)]
pub struct FormData {
    texts: HashMap<String, Vec<String>>,
    files: HashMap<String, Vec<Upload>>,
}

impl FormData {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.texts.get(name)?.first().map(String::as_str)
    }

    pub fn get_all(&self, name: &str) -> &[String] {
        self.texts
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn file(&self, name: &str) -> Option<&Upload> {
        self.files.get(name)?.first()
    }

    pub fn files(&self, name: &str) -> &[Upload] {
        self.files
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn text_map(&self) -> &HashMap<String, Vec<String>> {
        &self.texts
    }

    pub fn file_map(&self) -> &HashMap<String, Vec<Upload>> {
        &self.files
    }

    fn push_text(&mut self, name: String, value: String) {
        self.texts.entry(name).or_default().push(value);
    }

    #[cfg(feature = "multipart")]
    fn push_file(&mut self, upload: Upload) {
        self.files
            .entry(upload.field.clone())
            .or_default()
            .push(upload);
    }

    /// Flatten first text value per key for `serde_urlencoded` / simple structs.
    fn first_values(&self) -> HashMap<String, String> {
        self.texts
            .iter()
            .filter_map(|(k, v)| v.first().cloned().map(|val| (k.clone(), val)))
            .collect()
    }
}

impl Request {
    /// Parse form body once (urlencoded or multipart); cached on the request.
    pub async fn input(&mut self) -> Result<&FormData> {
        if self.get::<FormData>().is_some() {
            return Ok(self.get::<FormData>().expect("FormData"));
        }
        let parsed = parse_form_data(self).await?;
        self.set(parsed);
        Ok(self.get::<FormData>().expect("FormData"))
    }

    /// Deserialize text fields into `T` (works for urlencoded and multipart text parts).
    pub async fn form<T: DeserializeOwned>(&mut self) -> Result<T> {
        let data = self.input().await?;
        let map = data.first_values();
        let encoded = serde_urlencoded::to_string(&map)
            .map_err(|e| Error::BadRequest(format!("form encode: {e}")))?;
        serde_urlencoded::from_str(&encoded)
            .map_err(|e| Error::BadRequest(format!("form error: {e}")))
    }
}

async fn parse_form_data(req: &mut Request) -> Result<FormData> {
    let ct = req.content_type().unwrap_or("").to_ascii_lowercase();
    if ct.starts_with("multipart/") {
        #[cfg(feature = "multipart")]
        {
            return parse_multipart(req).await;
        }
        #[cfg(not(feature = "multipart"))]
        {
            return Err(Error::BadRequest(
                "multipart body requires the `multipart` feature".into(),
            ));
        }
    }

    // Default: urlencoded (also empty / missing CT for classic forms).
    let bytes = req.collect_body("form").await?;
    let mut data = FormData::default();
    if bytes.is_empty() {
        return Ok(data);
    }
    let pairs: Vec<(String, String)> = serde_urlencoded::from_bytes(&bytes)
        .map_err(|e| Error::BadRequest(format!("form error: {e}")))?;
    for (k, v) in pairs {
        data.push_text(k, v);
    }
    Ok(data)
}

#[cfg(feature = "multipart")]
async fn parse_multipart(req: &mut Request) -> Result<FormData> {
    use bytes::BytesMut;
    use futures_util::stream;
    use http_body_util::BodyExt;
    use multer::Multipart;

    let ct = req
        .header("content-type")
        .ok_or_else(|| Error::BadRequest("missing content-type".into()))?
        .to_string();
    let boundary = multer::parse_boundary(&ct)
        .map_err(|e| Error::BadRequest(format!("multipart boundary: {e}")))?;

    let limit = req.body_limit();
    let mut body = req.into_body_stream_as("multipart")?;
    let mut collected = BytesMut::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|e| Error::BadRequest(format!("multipart: {e}")))?;
        if let Ok(chunk) = frame.into_data() {
            if collected.len().saturating_add(chunk.len()) > limit {
                return Err(Error::PayloadTooLarge);
            }
            collected.extend_from_slice(&chunk);
        }
    }
    let bytes = collected.freeze();
    // Restore body so later readers (if any) see the same bytes; FormData is cached.
    req.body = crate::request::ReqBody::Bytes(bytes.clone());

    let stream = stream::once(async move { Ok::<_, std::io::Error>(bytes) });
    let mut mp = Multipart::new(stream, boundary);
    let mut data = FormData::default();
    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|e| Error::BadRequest(format!("multipart: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        let filename = field.file_name().map(str::to_string);
        let content_type = field.content_type().map(|m| m.to_string());
        let part = field
            .bytes()
            .await
            .map_err(|e| Error::BadRequest(format!("multipart field: {e}")))?;
        if filename.is_some() {
            data.push_file(Upload {
                field: name,
                filename,
                content_type,
                data: part,
            });
        } else {
            let s = String::from_utf8_lossy(&part).into_owned();
            data.push_text(name, s);
        }
    }
    Ok(data)
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

#[cfg(all(test, feature = "multipart"))]
mod tests {
    use super::*;
    use crate::Request;
    use bytes::Bytes;
    use http::Method;

    fn multipart_body(boundary: &str, parts: &str) -> Bytes {
        Bytes::from(format!("--{boundary}\r\n{parts}--{boundary}--\r\n"))
    }

    fn multipart_req(boundary: &str, parts: &str) -> Request {
        Request::builder()
            .method(Method::POST)
            .path("/upload")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(multipart_body(boundary, parts))
            .build()
    }

    #[tokio::test]
    async fn parses_text_and_file_fields() {
        let boundary = "----ruvoBound";
        let parts = concat!(
            "Content-Disposition: form-data; name=\"title\"\r\n\r\n",
            "hello\r\n",
            "------ruvoBound\r\n",
            "Content-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\n",
            "Content-Type: text/plain\r\n\r\n",
            "file-bytes\r\n",
        );
        let mut req = multipart_req(boundary, parts);
        let data = req.input().await.unwrap();
        assert_eq!(data.get("title"), Some("hello"));
        let file = data.file("file").unwrap();
        assert_eq!(file.filename.as_deref(), Some("a.txt"));
        assert_eq!(file.data.as_ref(), b"file-bytes");
    }

    #[tokio::test]
    async fn urlencoded_form_via_input() {
        let mut req = Request::builder()
            .method(Method::POST)
            .path("/")
            .header("content-type", "application/x-www-form-urlencoded")
            .body("name=Ada&age=1")
            .build();
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Body {
            name: String,
            age: u32,
        }
        let body: Body = req.form().await.unwrap();
        assert_eq!(
            body,
            Body {
                name: "Ada".into(),
                age: 1
            }
        );
    }

    #[tokio::test]
    async fn missing_boundary_is_bad_request() {
        let mut req = Request::builder()
            .method(Method::POST)
            .path("/")
            .header("content-type", "multipart/form-data")
            .body("x")
            .build();
        let err = req.input().await.unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
    }

    #[tokio::test]
    async fn oversize_body_is_413() {
        let boundary = "b";
        let big = "x".repeat(64);
        let parts = format!("Content-Disposition: form-data; name=\"f\"\r\n\r\n{big}\r\n");
        let mut req = Request::builder()
            .method(Method::POST)
            .path("/")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(multipart_body(boundary, &parts))
            .body_limit(16)
            .build();
        let err = req.input().await.unwrap_err();
        assert!(matches!(err, Error::PayloadTooLarge), "got {err:?}");
    }

    #[tokio::test]
    async fn broken_delimiter_is_bad_request() {
        let mut req = Request::builder()
            .method(Method::POST)
            .path("/")
            .header("content-type", "multipart/form-data; boundary=abc")
            .body("not-a-multipart-body")
            .build();
        let err = req.input().await.unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)), "got {err:?}");
    }
}
