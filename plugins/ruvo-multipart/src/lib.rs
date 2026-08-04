//! Multipart form parsing via [`multer`].

use bytes::{Bytes, BytesMut};
use futures_util::stream;
use http_body_util::BodyExt;
use multer::Multipart;
use ruvo_core::{Error, Request, Result};

/// One multipart field (text or file bytes).
#[derive(Debug, Clone)]
pub struct MultipartField {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Bytes,
}

/// Parse `multipart/form-data` from a request body stream.
pub trait MultipartExt {
    fn multipart(
        &mut self,
    ) -> impl std::future::Future<Output = Result<Vec<MultipartField>>> + Send;
}

impl MultipartExt for Request {
    async fn multipart(&mut self) -> Result<Vec<MultipartField>> {
        let ct = self
            .header("content-type")
            .ok_or_else(|| Error::BadRequest("missing content-type".into()))?
            .to_string();
        if !ct.to_ascii_lowercase().starts_with("multipart/") {
            return Err(Error::BadRequest(format!(
                "expected multipart content-type, got {ct}"
            )));
        }
        let boundary = multer::parse_boundary(&ct)
            .map_err(|e| Error::BadRequest(format!("multipart boundary: {e}")))?;

        let limit = self.body_limit();
        let mut body = self.into_body_stream_as("multipart")?;
        let mut collected = BytesMut::new();
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|e| Error::BadRequest(format!("multipart: {e}")))?;
            if let Ok(data) = frame.into_data() {
                if collected.len().saturating_add(data.len()) > limit {
                    return Err(Error::PayloadTooLarge);
                }
                collected.extend_from_slice(&data);
            }
        }
        let bytes = collected.freeze();
        let stream = stream::once(async move { Ok::<_, std::io::Error>(bytes) });
        let mut mp = Multipart::new(stream, boundary);
        let mut fields = Vec::new();
        while let Some(field) = mp
            .next_field()
            .await
            .map_err(|e| Error::BadRequest(format!("multipart: {e}")))?
        {
            let name = field.name().unwrap_or("").to_string();
            let filename = field.file_name().map(str::to_string);
            let content_type = field.content_type().map(|m| m.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| Error::BadRequest(format!("multipart field: {e}")))?;
            fields.push(MultipartField {
                name,
                filename,
                content_type,
                data,
            });
        }
        Ok(fields)
    }
}
