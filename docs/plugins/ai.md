---
title: ai
editLink: false
---

# `ai`

**AISDK language models (chat, tools, stream, fake)** · crate `sova-ai` `0.1.0` · id `ai`

Thin Sova shell over [`aisdk`](https://github.com/lazy-hq/aisdk) (Vercel AI SDK for Rust): install a default model, call it from handlers, or use the full AISDK builder / providers / `#[tool]` agents.

```bash
cargo add sova --features ai-openai
# or: ai-anthropic / ai-google / ai-full / ai-prompt
```

| Feature | What you get |
|---------|-------------|
| `ai` | Plugin + `req.ai()` + `FakeAi` |
| `ai-openai` | OpenAI provider (`aisdk/openai`) |
| `ai-anthropic` | Anthropic |
| `ai-google` | Google |
| `ai-full` | All aisdk providers |
| `ai-prompt` | File-based aisdk prompt templates |

## Install

```rust
use sova::{Ai, App};
use sova::ai::aisdk::providers::OpenAI; // with feature ai-openai

app.install(Ai::new().model(OpenAI::gpt_4o()).system("You are helpful."));
```

Config unset-fill (optional default system):

```toml
[ai]
system = "You are a concise assistant."
```

API keys stay with aisdk / env (`OPENAI_API_KEY`, …).

## Handlers

```rust
use sova::{AiExt, Json, Request};

app.post("/api/chat", |mut req: Request| async move {
    let prompt = req.json::<serde_json::Value>().await?
        .get("prompt").and_then(|v| v.as_str()).unwrap_or("hi");
    let text = req.ai().prompt(prompt).text().await?;
    Ok(Json(serde_json::json!({ "text": text })))
});

// SSE stream (text/event-stream)
app.get("/api/chat/stream", |req: Request| async move {
    Ok(req.ai().prompt("Say hello").stream_response().await?)
});
```

## Full AISDK (tools / agents)

```rust
use sova::ai::prelude::*;

let out = req.ai().builder()
    .system("You are helpful.")
    .prompt("What is the weather in NY?")
    .with_tool(get_weather())
    .stop_when(step_count_is(3))
    .build()
    .generate_text()
    .await?;
```

Re-exports: `sova::ai::aisdk` and `sova::ai::prelude`.

## Tests (`FakeAi`)

```rust
let fake = FakeAi::new().stub_text("pong");
app.install(Ai::fake(fake.clone()));
// …
fake.assert_called();
assert!(fake.prompts().iter().any(|p| p.contains("ping")));
```

## Example

`examples/api/api_ai` — JSON chat + SSE stream with `FakeAi` (swap to OpenAI for production).
