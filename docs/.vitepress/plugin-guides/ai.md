**When:** chat, tools, or streaming LLM responses from handlers.

**Does:**
- Install a default model via `Ai::new().model(...)` / `Ai::fake(...)`
- `req.ai().prompt(...).text()` / `.generate()` / `.stream_response()`
- Full AISDK builder (`req.ai().builder()`), providers, `#[tool]` agents
- `FakeAi` records prompts for tests

### Example

```rust
use sova::{Ai, AiExt, Json, Request};
use sova::ai::aisdk::providers::OpenAI; // feature ai-openai

app.install(Ai::new().model(OpenAI::gpt_4o()).system("You are helpful."));

app.post("/api/chat", |mut req: Request| async move {
    let prompt = req.json::<serde_json::Value>().await?
        .get("prompt").and_then(|v| v.as_str()).unwrap_or("hi");
    let text = req.ai().prompt(prompt).text().await?;
    Ok(Json(serde_json::json!({ "text": text })))
});
```

### Config

```toml
[ai]
system = "You are a concise assistant."
```

API keys stay with aisdk / env (`OPENAI_API_KEY`, …).

### Testing

```rust
let fake = FakeAi::new().stub_text("pong");
app.install(Ai::fake(fake.clone()));
// …
fake.assert_called();
```

### Notes
- Prefer `ai-openai` / `ai-anthropic` / `ai-google` over bare `ai`
- Re-exports: `sova::ai::aisdk`, `sova::ai::prelude`
