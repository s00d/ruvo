use crate::error::{Error, Result};
use bytes::Bytes;
use http_body::Body;
use http_body_util::BodyExt;

/// Testable body reader with size limit (works with any HTTP body).
pub async fn collect_limited<B>(mut body: B, max_body: usize) -> Result<Bytes>
where
    B: Body<Data = Bytes> + Unpin,
    B::Error: std::fmt::Display,
{
    let mut buf = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|e| Error::BadRequest(format!("body: {e}")))?;
        if let Ok(data) = frame.into_data() {
            if buf.len().saturating_add(data.len()) > max_body {
                return Err(Error::PayloadTooLarge);
            }
            buf.extend_from_slice(&data);
        }
    }
    Ok(Bytes::from(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Full;

    #[tokio::test]
    async fn collect_limited_returns_413() {
        let body = Full::new(Bytes::from(vec![1u8; 100]));
        let err = collect_limited(body, 16).await.unwrap_err();
        assert!(matches!(err, Error::PayloadTooLarge));
    }
}
