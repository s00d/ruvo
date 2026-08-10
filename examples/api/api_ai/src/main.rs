//! Minimal AI chat API using `sova-ai` + FakeAi (swap for OpenAI in production).

use serde::Deserialize;
use sova::{Ai, AiExt, App, FakeAi, Json, Request, Result};

#[derive(Deserialize)]
struct ChatIn {
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Production: `app.install(Ai::new().model(OpenAI::gpt_4o()));` with feature `ai-openai`
    let fake = FakeAi::new().stub_text("Hello from sova-ai (fake).");
    let mut app = App::new();
    app.install(Ai::fake(fake).system("You are a helpful assistant."));

    app.get("/api/health", || async {
        Json(serde_json::json!({ "ok": true }))
    });

    app.post("/api/chat", |mut req: Request| async move {
        let body = req.json::<ChatIn>().await.unwrap_or(ChatIn {
            prompt: "Say hi".into(),
        });
        let text = req.ai().prompt(body.prompt).text().await?;
        Ok::<_, sova::Error>(Json(serde_json::json!({ "text": text })))
    });

    app.get("/api/chat/stream", |req: Request| async move {
        Ok::<_, sova::Error>(
            req.ai()
                .prompt("Stream a short greeting.")
                .stream_response()
                .await?,
        )
    });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    eprintln!("api_ai listening on http://127.0.0.1:{port}");
    app.listen(port).await
}
