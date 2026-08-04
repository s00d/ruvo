use bytes::Bytes;
use http::Method;
use ruvo_core::{Error, Request};
use ruvo_multipart::MultipartExt;

fn multipart_body(boundary: &str, parts: &str) -> Bytes {
    Bytes::from(format!(
        "--{boundary}\r\n{parts}--{boundary}--\r\n"
    ))
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
    let fields = req.multipart().await.unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "title");
    assert_eq!(fields[0].data.as_ref(), b"hello");
    assert_eq!(fields[1].name, "file");
    assert_eq!(fields[1].filename.as_deref(), Some("a.txt"));
    assert_eq!(fields[1].data.as_ref(), b"file-bytes");
}

#[tokio::test]
async fn missing_boundary_is_bad_request() {
    let mut req = Request::builder()
        .method(Method::POST)
        .path("/")
        .header("content-type", "multipart/form-data")
        .body("x")
        .build();
    let err = req.multipart().await.unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)));
}

#[tokio::test]
async fn oversize_body_is_413() {
    let boundary = "b";
    let big = "x".repeat(64);
    let parts = format!(
        "Content-Disposition: form-data; name=\"f\"\r\n\r\n{big}\r\n"
    );
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
    let err = req.multipart().await.unwrap_err();
    assert!(
        matches!(err, Error::PayloadTooLarge),
        "got {err:?}"
    );
}

#[tokio::test]
async fn broken_delimiter_is_bad_request() {
    let mut req = Request::builder()
        .method(Method::POST)
        .path("/")
        .header("content-type", "multipart/form-data; boundary=abc")
        .body("not-a-multipart-body")
        .build();
    let err = req.multipart().await.unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)), "got {err:?}");
}
