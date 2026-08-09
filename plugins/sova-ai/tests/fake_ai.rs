use http::Method;
use sova_ai::{Ai, AiExt, FakeAi};
use sova_core::{App, Json, Request, Response};

#[tokio::test]
async fn fake_generate_records_prompt() {
    let fake = FakeAi::new().stub_text("pong");
    let mut app = App::new();
    app.install(Ai::fake(fake.clone()));
    app.post("/chat", |req: Request| async move {
        let text = req.ai().prompt("ping").text().await?;
        Ok::<_, sova_core::Error>(Json(serde_json::json!({ "text": text })))
    });

    let res = app
        .handle_request(Method::POST, "/chat", "")
        .await;
    assert_eq!(res.status_code().as_u16(), 200);
    let body = String::from_utf8(res.body_bytes().unwrap().to_vec()).unwrap();
    assert!(body.contains("pong"), "{body}");
    fake.assert_called_times(1);
    assert!(
        fake.prompts().iter().any(|p| p.contains("ping")),
        "{:?}",
        fake.prompts()
    );
}

#[tokio::test]
async fn stream_response_is_sse() {
    let fake = FakeAi::new().stub_text("hello-stream");
    let mut app = App::new();
    app.install(Ai::fake(fake));
    app.get("/stream", |req: Request| async move {
        Ok::<_, sova_core::Error>(req.ai().prompt("go").stream_response().await?)
    });

    let res = app.handle_request(Method::GET, "/stream", "").await;
    assert_eq!(res.status_code().as_u16(), 200);
    assert_eq!(
        res.headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );
}

#[tokio::test]
async fn fake_stream_text_records() {
    use aisdk::core::LanguageModel;
    use aisdk::core::language_model::LanguageModelOptions;
    use futures_util::StreamExt;

    let mut fake = FakeAi::new().stub_text("chunk");
    let stream = fake
        .stream_text(LanguageModelOptions::default())
        .await
        .expect("stream");
    let chunks: Vec<_> = stream.collect().await;
    assert!(!chunks.is_empty());
    fake.assert_called_times(1);
}

#[tokio::test]
async fn missing_prompt_errors() {
    let fake = FakeAi::new().stub_text("x");
    let mut app = App::new();
    app.install(Ai::fake(fake));
    app.get("/bad", |req: Request| async move {
        match req.ai().generate().await {
            Ok(_) => Ok::<_, sova_core::Error>(Response::text("ok")),
            Err(e) => Ok(Response::text(e.to_string()).status(400)),
        }
    });
    let res = app.handle_request(Method::GET, "/bad", "").await;
    assert_eq!(res.status_code().as_u16(), 400);
}
