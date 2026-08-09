Install a model (or FakeAi), then call from handlers:

```rust
use sova::{Ai, AiExt, FakeAi, Json, Request};
use sova::ai::aisdk::providers::OpenAI; // feature ai-openai

app.install(Ai::new().model(OpenAI::gpt_4o()).system("You are helpful."));
// tests: app.install(Ai::fake(FakeAi::new().stub_text("pong")));

app.post("/api/chat", |mut req: Request| async move {
    let prompt = req.json::<serde_json::Value>().await?
        .get("prompt").and_then(|v| v.as_str()).unwrap_or("hi");
    Ok(Json(serde_json::json!({ "text": req.ai().prompt(prompt).text().await? })))
});

app.get("/api/chat/stream", |req: Request| async move {
    Ok(req.ai().prompt("Say hello").stream_response().await?)
});
```

Full AISDK tools/agents: `req.ai().builder()` + `sova::ai::prelude`. See `examples/api/api_ai`.
