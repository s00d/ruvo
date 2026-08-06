use ruvo_core::{App,  Http, Request, Response};
use ruvo_core::extend::Bind;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::test]
async fn http_all_adds_alt_svc_header() {
    let mut app = App::new();
    app.get("/", |_r: Request| async { Response::text("ok") });

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        app.bind(Bind::Listener(listener))
            .http(Http::all())
            .shutdown(async move {
                let _ = rx.await;
            })
            .serve()
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let text = String::from_utf8_lossy(&buf).to_ascii_lowercase();

    let expected = format!("alt-svc: h3=\":{}\"; ma=86400", addr.port());
    assert!(text.contains(&expected), "response must contain Alt-Svc");

    let _ = tx.send(());
    let _ = server.await;
}

