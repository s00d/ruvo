---
title: ai
editLink: false
---

# `ai`

**AISDK language models (chat, tools, stream, fake)**

| | |
|--|--|
| Crate | [`sova-ai`](https://docs.rs/sova-ai/0.1.0) `0.1.0` |
| Plugin id | `ai` |
| Category | Integrations |

## Install

```bash
cargo add sova --features ai-openai
```

## Features

| Feature | What you get |
|---------|-------------|
| `ai` | Plugin + `req.ai()` + `FakeAi` (AISDK shell). |
| `ai-anthropic` | Anthropic provider. |
| `ai-full` | All aisdk providers. |
| `ai-google` | Google provider. |
| `ai-openai` | OpenAI provider (`aisdk/openai`). |
| `ai-prompt` | File-based aisdk prompt templates. |

## Overview

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

## Quick start

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

Full AISDK tools/agents: `req.ai().builder()` + `sova::ai::prelude`. See [`examples/api/api_ai`](https://github.com/s00d/sova/tree/master/examples/api/api_ai).

## Examples

- [`examples/api/api_ai`](https://github.com/s00d/sova/tree/master/examples/api/api_ai)

## Related

[`http`](/plugins/http) · [`sse`](/plugins/sse)
